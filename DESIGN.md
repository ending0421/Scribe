# Scribe 架构设计与实现脑暴

## 1. 核心问题分析

### 1.1 传统日志框架的痛点
- **JVM GC Spikes**: Java/Kotlin 的 GC 暂停影响性能
- **UI Thread I/O Blocking**: 主线程写入日志导致卡顿
- **Crash Data Loss**: 应用崩溃时缓冲区数据丢失
- **Multi-Process Conflicts**: 多进程访问同一日志文件导致冲突

### 1.2 Scribe 的解决方案
- **Rust + mmap**: 零 GC，OS 级别持久化
- **Lock-Free Double Buffer**: 无锁原子交换，零阻塞
- **CRC32 + Magic Header**: 崩溃时检测并丢弃损坏的日志
- **PID-Based File Isolation**: 每个进程独立的 mmap 文件

---

## 2. 核心模块设计

### 2.1 模块依赖关系图

```
┌─────────────────────────────────────────────────────┐
│  FFI Layer (lib.rs)                                 │
│  - C-ABI exports: scribe_init, scribe_write, etc.   │
└─────────────────┬───────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────┐
│  Core Coordinator (core.rs)                         │
│  - Global state management                          │
│  - Thread-safe singleton pattern                    │
└─────────────┬───────────────────────────────────────┘
                  │
         ┌────────┴────────┐
         ▼                 ▼
┌──────────────────┐  ┌──────────────────┐
│  Storage Engine  │  │  Platform Layer  │
│  (storage.rs)    │  │  (platform/)     │
│                  │  │                  │
│  DoubleBuffer    │  │  - android.rs    │
│  Manager         │  │  - ios.rs        │
└────┬────┬────────┘  └──────────────────┘
     │    │
     │    └─────────────┐
     ▼                  ▼
┌─────────────┐  ┌──────────────┐
│ Compressor  │  │  Encryptor   │
│ (compress.rs)│  │ (encrypt.rs) │
└─────────────┘  └──────────────┘
```

### 2.2 模块职责详细拆解

#### **2.2.1 FFI Layer (lib.rs)**
**职责**:
- 暴露 C-ABI 兼容的函数接口
- 处理 FFI 安全性（空指针检查、字符串转换）
- 初始化/销毁全局状态

**关键 API**:
```rust
// 初始化
scribe_init(log_dir: *const c_char, config: *const ScribeConfig) -> i32

// 写入日志
scribe_write(level: i32, tag: *const c_char, msg: *const c_char) -> i32

// 刷新缓冲区
scribe_flush() -> i32

// 清理资源
scribe_destroy() -> i32
```

**设计要点**:
- 使用 `std::ffi::{CStr, CString}` 处理 C 字符串
- 返回错误码（0 = 成功，负数 = 错误）
- 线程安全：多个线程同时调用 `scribe_write`

---

#### **2.2.2 Core Coordinator (core.rs)**
**职责**:
- 管理全局单例 `StorageManager`
- 提供线程安全的访问接口
- 协调各模块初始化

**设计挑战**:
- **全局单例模式**: 使用 `once_cell::sync::OnceCell` 或 `lazy_static`
- **线程安全**: 确保 `Send + Sync`

**伪代码**:
```rust
static GLOBAL_MANAGER: OnceCell<Arc<StorageManager>> = OnceCell::new();

pub fn init_global_manager(config: Config) -> Result<()> {
    GLOBAL_MANAGER.set(Arc::new(StorageManager::new(config)?))
        .map_err(|_| Error::AlreadyInitialized)
}

pub fn get_global_manager() -> &'static Arc<StorageManager> {
    GLOBAL_MANAGER.get().expect("Manager not initialized")
}
```

---

#### **2.2.3 Storage Engine (storage.rs) - 核心难点**

##### **A. DoubleBufferManager 结构**
```rust
pub struct DoubleBufferManager {
    // 双缓冲区
    buffers: [MmapBuffer; 2],
    
    // 当前活跃的缓冲区索引 (0 或 1)
    active_index: AtomicU8,
    
    // 活跃写入者计数器 (用于安全交换)
    active_writers: [AtomicUsize; 2],
    
    // 后台工作线程句柄
    worker_handle: Option<JoinHandle<()>>,
    
    // 发送满缓冲区到工作线程的通道
    swap_sender: Sender<MmapBuffer>,
}
```

##### **B. 无锁交换算法 (Lock-Free Swap)**
**核心流程**:
```
1. Writer Thread (多个并发):
   ┌─────────────────────────────────────┐
   │ 1. idx = active_index.load()       │
   │ 2. active_writers[idx].fetch_add(1) │
   │ 3. write_to_buffer(buffers[idx])    │
   │ 4. active_writers[idx].fetch_sub(1) │
   └─────────────────────────────────────┘

2. Swapper Thread (单个):
   ┌─────────────────────────────────────────┐
   │ 1. old_idx = active_index.fetch_xor(1)  │  ← 原子交换
   │ 2. spin_wait(active_writers[old_idx])   │  ← 等待写入完成
   │ 3. send(buffers[old_idx]) to worker    │  ← 发送到后台
   │ 4. reset(buffers[old_idx])              │  ← 重置缓冲区
   └─────────────────────────────────────────┘
```

**关键原子操作**:
- `fetch_xor(1)`: 在 0 和 1 之间切换（`0 ^ 1 = 1`, `1 ^ 1 = 0`）
- `Ordering`: 使用 `AcqRel` 确保内存可见性

##### **C. MmapBuffer 设计**
```rust
pub struct MmapBuffer {
    // mmap 内存映射
    mmap: MmapMut,
    
    // 当前写入位置
    position: AtomicUsize,
    
    // 缓冲区容量
    capacity: usize,
    
    // 文件路径 (包含 PID)
    file_path: PathBuf,
}
```

**多进程安全**:
- 文件名格式: `scribe_{pid}_{buffer_id}.mmap`
- 每个进程使用独立的 mmap 文件

##### **D. 日志帧格式 (Log Frame)**
```
┌──────────────────────────────────────────┐
│ Magic Header (4 bytes): 0xFEEDC0DE       │
├──────────────────────────────────────────┤
│ Frame Length (4 bytes): u32              │
├──────────────────────────────────────────┤
│ Timestamp (8 bytes): i64                 │
├──────────────────────────────────────────┤
│ Level (1 byte): u8                       │
├──────────────────────────────────────────┤
│ Tag Length (2 bytes): u16                │
├──────────────────────────────────────────┤
│ Tag Data (variable)                      │
├──────────────────────────────────────────┤
│ Message Length (4 bytes): u32            │
├──────────────────────────────────────────┤
│ Message Data (variable)                  │
├──────────────────────────────────────────┤
│ CRC32 (4 bytes): u32                     │
└──────────────────────────────────────────┘
```

**崩溃检测**:
- 读取时验证 Magic Header
- 验证 CRC32
- 如果损坏，丢弃并继续读取下一帧

---

#### **2.2.4 Compressor (compress.rs)**
**依赖**: `zstd` crate

**核心功能**:
```rust
pub fn compress_buffer(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level)
}

pub fn decompress_buffer(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data)
}
```

**优化**:
- 支持字典训练（Dictionary Training）用于日志负载
- 压缩级别配置（默认 3，范围 1-22）

---

#### **2.2.5 Encryptor (encrypt.rs)**
**依赖**: `chacha20poly1305` crate

**核心功能**:
```rust
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce
};

pub struct Encryptor {
    cipher: ChaCha20Poly1305,
}

impl Encryptor {
    pub fn encrypt(&self, plaintext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        let nonce = Nonce::from_slice(nonce);
        self.cipher.encrypt(nonce, plaintext)
            .map_err(|_| Error::EncryptionFailed)
    }
}
```

**安全考虑**:
- Nonce 不能重复（使用递增计数器或随机数）
- Key 从配置传入（32 字节）

---

#### **2.2.6 Platform Layer (platform/)**

##### **Android (android.rs)**
```rust
#[cfg(target_os = "android")]
pub fn log_to_console(level: i32, tag: &str, msg: &str) {
    use std::ffi::CString;
    use std::os::raw::c_int;
    
    extern "C" {
        fn __android_log_write(
            prio: c_int,
            tag: *const c_char,
            text: *const c_char,
        ) -> c_int;
    }
    
    let tag_c = CString::new(tag).unwrap();
    let msg_c = CString::new(msg).unwrap();
    
    unsafe {
        __android_log_write(level, tag_c.as_ptr(), msg_c.as_ptr());
    }
}
```

##### **iOS (ios.rs)**
```rust
#[cfg(target_os = "ios")]
pub fn log_to_console(level: i32, tag: &str, msg: &str) {
    // 使用 os_log FFI
    // 需要链接 System framework
}
```

---

## 3. 数据流分析

### 3.1 写入流程 (Write Path)
```
App Thread
    │
    │ scribe_write(level, tag, msg)
    ▼
┌─────────────────────────────┐
│ 1. Get active buffer index  │  ← active_index.load(Acquire)
│ 2. Increment writer counter │  ← active_writers[idx].fetch_add(1)
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│ 3. Serialize log frame      │  ← Magic + Timestamp + Tag + Msg + CRC32
│ 4. Write to mmap buffer     │  ← memcpy to mmap region
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│ 5. Decrement writer counter │  ← active_writers[idx].fetch_sub(1)
│ 6. Check if buffer full     │  ← If full, trigger swap
└─────────────────────────────┘
```

### 3.2 交换流程 (Swap Path)
```
Swapper Thread (triggered when buffer full)
    │
    ▼
┌─────────────────────────────┐
│ 1. Atomic swap active index │  ← active_index.fetch_xor(1)
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│ 2. Wait for active writers  │  ← spin: while active_writers[old] != 0
└─────────┬───────────────────┘
          │
          ▼
┌─────────────────────────────┐
│ 3. Send to worker thread    │  ← swap_sender.send(old_buffer)
└─────────────────────────────┘

Worker Thread
    │
    ▼
┌─────────────────────────────┐
│ 1. Receive full buffer      │  ← swap_receiver.recv()
│ 2. Compress (zstd)          │
│ 3. Encrypt (ChaCha20)       │
│ 4. Write to disk            │
│ 5. Reset buffer             │
└─────────────────────────────┘
```

---

## 4. 错误处理与降级策略

### 4.1 磁盘空间不足 (ENOSPC)
**场景**: mmap 无法分配磁盘空间
**降级策略**:
```rust
match MmapBuffer::new(file_path, capacity) {
    Ok(buffer) => { /* 正常使用 mmap */ },
    Err(Error::NoSpace) => {
        // 降级到内存 Ring Buffer
        warn!("Disk full, fallback to in-memory ring buffer");
        fallback_to_ring_buffer()
    }
}
```

### 4.2 崩溃恢复 (Crash Recovery)
**场景**: 应用启动时检测到未完成的 mmap 文件
**恢复流程**:
```
1. 扫描 log_dir 中所有 *.mmap 文件
2. 对每个文件:
   - 验证 Magic Header
   - 逐帧读取，验证 CRC32
   - 遇到损坏帧时停止并丢弃剩余数据
3. 将有效数据压缩加密后写入持久化日志
4. 删除临时 mmap 文件
```

---

## 5. 存储管理与清理策略

### 5.1 设计目标

**核心目标：**
- 防止无限增长（日志文件不能无限累积）
- 可预测的存储占用（默认 50MB 上限）
- 保留关键日志（Error 日志保留更久）
- 透明可控（清理行为可配置、可观测）

**移动端约束：**
- 16GB 设备可用空间 < 1GB
- App Store 会拒绝占用过多存储
- 需要保留足够的历史日志排查问题（至少 7 天）
- 清理不能阻塞主线程

### 5.2 文件组织结构

**目录结构：**
```
/data/data/com.example.app/files/logs/
├── scribe_20260614_001.log          # 当前活跃文件
├── scribe_20260614_000.log.zst      # 已归档（压缩）
├── scribe_20260613_002.log.zst
├── scribe_20260613_001.log.zst
└── .scribe_metadata.json            # 元数据文件
```

**文件命名规则：**
```rust
// 格式：scribe_{date}_{sequence}.log[.zst]
// 示例：scribe_20260614_001.log
pub struct LogFileName {
    date: String,        // "20260614" (YYYYMMDD)
    sequence: u32,       // 001, 002, 003...
    compressed: bool,    // .zst 后缀
}
```

### 5.3 清理策略

#### **策略 1: 基于大小的清理**

```rust
pub struct SizeBasedCleanup {
    max_total_size: usize,     // 总大小上限：50MB
    max_file_size: usize,      // 单文件上限：10MB
    cleanup_threshold: f32,    // 清理阈值：0.9 (90%)
}

impl SizeBasedCleanup {
    pub fn cleanup(&self, files: &mut Vec<LogFile>) -> Result<CleanupReport> {
        // 1. 按优先级排序（年龄 + 级别 + 大小）
        files.sort_by_key(|f| self.priority_score(f));
        
        // 2. 删除到目标大小（80%）
        let target_size = (self.max_total_size as f32 * 0.8) as usize;
        // ...
    }
    
    fn priority_score(&self, file: &LogFile) -> u64 {
        // 分数越高 = 越不重要 = 越先删除
        let age_score = file.age_days() as u64 * 100;
        let size_score = file.size as u64 / 1024;
        let level_score = match file.min_level {
            Level::Debug => 1000,
            Level::Error => 0,
            _ => 500,
        };
        age_score + size_score + level_score
    }
}
```

#### **策略 2: 基于时间的清理**

```rust
pub struct TimeBasedCleanup {
    retention_days: u32,  // 全局保留天数：7
    level_retention: HashMap<Level, u32>,  // 按级别差异化
}

impl Default for TimeBasedCleanup {
    fn default() -> Self {
        let mut level_retention = HashMap::new();
        level_retention.insert(Level::Debug, 1);   // Debug 保留 1 天
        level_retention.insert(Level::Info, 3);    // Info 保留 3 天
        level_retention.insert(Level::Warn, 7);    // Warn 保留 7 天
        level_retention.insert(Level::Error, 30);  // Error 保留 30 天
        
        Self {
            retention_days: 7,
            level_retention,
        }
    }
}
```

#### **推荐：混合策略**

```rust
pub struct HybridCleanupPolicy {
    // 硬限制（必须遵守）
    max_total_size: usize,     // 50MB 硬上限
    max_file_size: usize,      // 10MB/文件
    
    // 软限制（优先考虑）
    retention_days: u32,       // 7 天保留期
    level_retention: HashMap<Level, u32>,
    
    // 清理策略
    cleanup_threshold: f32,    // 0.9 = 90% 时触发
}

impl HybridCleanupPolicy {
    pub fn cleanup(&self, files: &mut Vec<LogFile>) -> Result<CleanupReport> {
        // 第一步：删除超过保留期的文件（时间策略）
        let time_based = self.cleanup_by_time(files)?;
        
        // 第二步：如果仍超过大小限制，按优先级删除（大小策略）
        if current_size > self.max_total_size {
            let size_based = self.cleanup_by_size(files)?;
        }
        
        Ok(report)
    }
}
```

### 5.4 文件轮转 (Log Rotation)

**轮转触发条件：**
```rust
pub enum RotationTrigger {
    Size(usize),           // 文件大小达到 10MB
    Time(Duration),        // 每 24 小时
    Manual,                // 手动触发
}
```

**轮转流程：**
```rust
impl StorageManager {
    pub fn rotate(&mut self) -> Result<()> {
        // 1. 刷新当前缓冲区
        self.flush()?;
        
        // 2. 关闭当前活跃文件
        let old_file = self.close_active_file()?;
        
        // 3. 压缩旧文件（异步）
        self.compress_async(old_file)?;
        
        // 4. 创建新文件
        let new_file = self.create_new_file()?;
        
        // 5. 触发清理检查
        self.maybe_cleanup()?;
        
        Ok(())
    }
}
```

### 5.5 压缩策略

**压缩时机：轮转时立即压缩（推荐）**

**压缩算法：zstd（压缩率 5:1）**

```rust
pub fn compress_log_file(input: &Path, output: &Path) -> Result<()> {
    let mut encoder = zstd::stream::Encoder::new(output_file, 3)?;  // 级别 3
    std::io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}
```

### 5.6 清理触发时机

```rust
pub enum CleanupTrigger {
    OnStartup,          // 应用启动时（推荐）
    OnRotation,         // 文件轮转时（推荐）
    OnLowStorage,       // 系统存储不足时
    Manual,             // 手动触发
}

impl Scribe {
    pub fn init(config: Config) -> Result<Self> {
        let mut scribe = Self::new(config)?;
        
        // 启动时清理一次
        scribe.cleanup_sync()?;
        
        Ok(scribe)
    }
    
    fn on_rotation(&mut self) -> Result<()> {
        self.rotate()?;
        
        // 轮转后异步清理（不阻塞）
        self.cleanup_async()?;
        
        Ok(())
    }
}
```

### 5.7 配置接口

```rust
pub struct StorageConfig {
    // 存储限制
    pub max_total_size: usize,         // 默认 50MB
    pub max_file_size: usize,          // 默认 10MB
    
    // 保留策略
    pub retention_days: u32,           // 默认 7 天
    pub level_retention: HashMap<Level, u32>,
    
    // 清理策略
    pub cleanup_threshold: f32,        // 默认 0.9 (90%)
    pub cleanup_on_startup: bool,      // 默认 true
    pub cleanup_on_rotation: bool,     // 默认 true
    
    // 压缩策略
    pub compress_on_rotation: bool,    // 默认 true
    pub compression_level: i32,        // 默认 3
}

impl Default for StorageConfig {
    fn default() -> Self {
        let mut level_retention = HashMap::new();
        level_retention.insert(Level::Debug, 1);
        level_retention.insert(Level::Info, 3);
        level_retention.insert(Level::Warn, 7);
        level_retention.insert(Level::Error, 30);
        
        Self {
            max_total_size: 50 * 1024 * 1024,  // 50MB
            max_file_size: 10 * 1024 * 1024,   // 10MB
            retention_days: 7,
            level_retention,
            cleanup_threshold: 0.9,
            cleanup_on_startup: true,
            cleanup_on_rotation: true,
            compress_on_rotation: true,
            compression_level: 3,
        }
    }
}
```

**使用示例：**
```rust
// 默认配置
let scribe = Scribe::init(StorageConfig::default())?;

// 自定义配置
let config = StorageConfig {
    max_total_size: 100 * 1024 * 1024,  // 100MB
    retention_days: 14,                  // 保留 14 天
    ..Default::default()
};
let scribe = Scribe::init(config)?;

// 极简配置（最小存储占用）
let config = StorageConfig {
    max_total_size: 10 * 1024 * 1024,   // 10MB
    retention_days: 3,                   // 仅保留 3 天
    ..Default::default()
};
```

### 5.8 清理报告与可观测性

```rust
pub struct CleanupReport {
    pub started_at: SystemTime,
    pub completed_at: SystemTime,
    pub files_deleted: usize,
    pub bytes_freed: u64,
    pub deleted_files: Vec<String>,
    pub errors: Vec<CleanupError>,
}

impl CleanupReport {
    pub fn summary(&self) -> String {
        format!(
            "Cleanup: deleted {} files, freed {:.2}MB",
            self.files_deleted,
            self.bytes_freed as f64 / 1024.0 / 1024.0,
        )
    }
}
```

---

## 6. 性能优化要点

### 6.1 内存对齐
- 使用 `#[repr(C)]` 确保结构体布局
- 关键原子变量使用 cache-line padding 避免 false sharing

### 6.2 预分配
- mmap buffer 预分配固定大小（如 4MB）
- 避免运行时动态分配

### 6.3 批量处理
- Worker 线程可批量压缩多个满缓冲区
- 减少系统调用次数

---

## 7. 测试策略

### 6.1 单元测试
- 日志帧序列化/反序列化
- CRC32 验证
- 压缩/解压缩
- 加密/解密

### 6.2 并发测试
- 多线程同时写入
- 验证数据完整性
- 无数据竞争（使用 `cargo test -- --test-threads=20`）

### 6.3 崩溃测试
- 模拟进程突然终止
- 验证 mmap 数据持久化
- 验证崩溃恢复逻辑

### 6.4 性能基准测试
- 吞吐量测试（logs/sec）
- 延迟测试（P99, P999）
- 与其他日志框架对比

---

## 8. 依赖项清单

```toml
[dependencies]
# 内存映射
memmap2 = "0.9"

# 压缩
zstd = "0.13"

# 加密
chacha20poly1305 = "0.10"

# 并发工具
once_cell = "1.19"
crossbeam-channel = "0.5"

# CRC 校验
crc32fast = "1.4"

# 时间戳
chrono = "0.4"

[target.'cfg(target_os = "android")'.dependencies]
# Android NDK 绑定 (如果需要)
ndk = "0.8"
ndk-sys = "0.5"

[dev-dependencies]
criterion = "0.5"  # 性能基准测试
tempfile = "3.0"   # 临时文件
```

---

## 9. 实现优先级

### Phase 1: 核心基础 (MVP)
1. ✅ 项目脚手架 (Cargo.toml, 基础文件结构)
2. ✅ 日志帧格式定义和序列化
3. ✅ 单缓冲区 mmap 写入（无交换）
4. ✅ FFI 接口定义

### Phase 2: 并发与安全
5. ✅ 双缓冲区无锁交换
6. ✅ 多线程写入测试
7. ✅ 后台 worker 线程

### Phase 3: 持久化与加密
8. ✅ 压缩集成 (zstd)
9. ✅ 加密集成 (ChaCha20-Poly1305)
10. ✅ 磁盘持久化

### Phase 4: 平台集成
11. ✅ Android logcat 集成
12. ✅ iOS os_log 集成
13. ✅ 多进程隔离 (PID-based files)

### Phase 5: 错误处理与优化
14. ✅ 崩溃恢复逻辑
15. ✅ 磁盘空间不足降级
16. ✅ 性能优化和基准测试

---

## 10. 开放问题与决策点

### Q1: mmap 缓冲区大小？
- **选项 A**: 固定 4MB（平衡内存和交换频率）
- **选项 B**: 可配置（1MB - 16MB）
- **推荐**: 选项 B，提供合理默认值

### Q2: 压缩是否可选？
- **场景**: 某些应用可能优先考虑速度而非存储
- **推荐**: 提供配置项，默认开启

### Q3: 加密 Key 如何管理？
- **选项 A**: 从配置文件读取
- **选项 B**: 从 Keychain/KeyStore 读取
- **推荐**: 先实现选项 A，后续扩展选项 B

### Q4: 日志文件轮转策略？
- **按大小**: 单个文件超过 100MB 时创建新文件
- **按时间**: 每天创建新文件
- **按数量**: 最多保留 N 个文件
- **推荐**: 三者结合，提供配置

---

## 11. 下一步行动

1. **创建项目脚手架**: 初始化 Cargo 项目，设置目录结构
2. **定义核心数据结构**: LogFrame, MmapBuffer, Config
3. **实现基础序列化**: 日志帧的编码/解码
4. **实现单缓冲区版本**: 先不考虑交换，验证 mmap 写入
5. **逐步添加并发**: 实现双缓冲区和无锁交换
6. **集成压缩和加密**: 完善持久化流程
7. **编写测试**: 单元测试、并发测试、崩溃测试
8. **性能调优**: 基准测试和优化

---

**准备开始实现？请确认是否需要调整设计细节，或者直接进入 Phase 1 的代码实现。**
