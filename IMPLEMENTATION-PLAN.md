# Scribe 实现计划

## 概述

本文档详细分解 Scribe 的实现任务，按照 5 个 Phase 逐步推进，每个 Phase 包含具体的任务、验收标准和预估时间。

**总体时间估算：4-6 周**

---

## Phase 0: 项目初始化 (1-2 天)

### 任务列表

#### 0.1 创建 Cargo 项目
```bash
cargo new --lib scribe
cd scribe
```

**产物：**
- `Cargo.toml`
- `src/lib.rs`
- `.gitignore`

**验收标准：**
- `cargo build` 成功
- `cargo test` 成功

---

#### 0.2 配置项目结构

**创建目录：**
```bash
mkdir -p src/{storage,platform,stages,outputs}
mkdir -p tests/{unit,integration}
mkdir -p benches
mkdir -p examples
```

**创建文件：**
```
src/
├── lib.rs              # FFI 接口
├── core.rs             # 全局管理
├── error.rs            # 错误类型
├── config.rs           # 配置
├── storage/
│   ├── mod.rs
│   ├── buffer.rs       # MmapBuffer
│   ├── manager.rs      # DoubleBufferManager
│   └── frame.rs        # LogFrame
├── pipeline/
│   ├── mod.rs
│   ├── router.rs       # Router
│   └── stage.rs        # PipelineStage trait
├── stages/
│   ├── mod.rs
│   ├── compress.rs     # 压缩
│   └── encrypt.rs      # 加密
├── outputs/
│   ├── mod.rs
│   └── disk.rs         # 磁盘输出
└── platform/
    ├── mod.rs
    ├── android.rs      # Android 适配
    └── ios.rs          # iOS 适配
```

**验收标准：**
- 目录结构创建完成
- 所有 `.rs` 文件包含基础模块声明
- `cargo build` 成功

---

#### 0.3 配置依赖

**编辑 `Cargo.toml`：**
```toml
[package]
name = "scribe"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib", "rlib"]

[dependencies]
# 内存映射
memmap2 = "0.9"

# 原子操作
parking_lot = "0.12"

# 错误处理
thiserror = "1.0"

# 序列化
byteorder = "1.5"

# CRC 校验
crc32fast = "1.4"

# 时间处理
once_cell = "1.19"

[target.'cfg(target_os = "android")'.dependencies]
# Android NDK (可选)
# ndk = "0.8"

[dev-dependencies]
criterion = "0.5"
tempfile = "3.0"

[[bench]]
name = "write_bench"
harness = false
```

**验收标准：**
- `cargo build` 成功
- 所有依赖下载完成

---

#### 0.4 设置 CI/CD（可选）

**创建 `.github/workflows/ci.yml`：**
```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --verbose
      - run: cargo test --verbose
      - run: cargo test --release --verbose
```

**验收标准：**
- CI 配置文件创建
- Push 后 CI 运行成功（如果配置了）

---

## Phase 1: 核心基础 (MVP) (1 周)

### 目标
实现单缓冲区版本，验证 mmap 写入和日志帧序列化。

---

### 1.1 定义错误类型 (0.5 天)

**文件：`src/error.rs`**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScribeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("mmap error: {0}")]
    Mmap(String),
    
    #[error("Disk full")]
    DiskFull,
    
    #[error("Buffer full")]
    BufferFull,
    
    #[error("Already initialized")]
    AlreadyInitialized,
    
    #[error("Not initialized")]
    NotInitialized,
    
    #[error("Invalid log frame")]
    InvalidFrame,
    
    #[error("CRC mismatch")]
    CrcMismatch,
}

pub type Result<T> = std::result::Result<T, ScribeError>;
```

**验收标准：**
- 错误类型定义完整
- `cargo build` 成功

---

### 1.2 定义日志帧格式 (1 天)

**文件：`src/storage/frame.rs`**

```rust
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crc32fast::Hasher;
use std::io::{Cursor, Write};

const MAGIC_HEADER: u32 = 0xFEEDC0DE;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Verbose = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

pub struct LogFrame {
    pub timestamp: i64,
    pub level: LogLevel,
    pub tag: String,
    pub message: String,
}

impl LogFrame {
    pub fn new(level: LogLevel, tag: String, message: String) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64,
            level,
            tag,
            message,
        }
    }
    
    pub fn serialize(&self) -> crate::Result<Vec<u8>> {
        let mut buf = Vec::new();
        
        // Magic Header
        buf.write_u32::<LittleEndian>(MAGIC_HEADER)?;
        
        // 预留 Frame Length（稍后填充）
        let length_pos = buf.len();
        buf.write_u32::<LittleEndian>(0)?;
        
        // Timestamp
        buf.write_i64::<LittleEndian>(self.timestamp)?;
        
        // Level
        buf.write_u8(self.level as u8)?;
        
        // Tag
        let tag_bytes = self.tag.as_bytes();
        buf.write_u16::<LittleEndian>(tag_bytes.len() as u16)?;
        buf.write_all(tag_bytes)?;
        
        // Message
        let msg_bytes = self.message.as_bytes();
        buf.write_u32::<LittleEndian>(msg_bytes.len() as u32)?;
        buf.write_all(msg_bytes)?;
        
        // 计算 CRC32（不包括 Magic 和 Length）
        let mut hasher = Hasher::new();
        hasher.update(&buf[8..]);
        let crc = hasher.finalize();
        buf.write_u32::<LittleEndian>(crc)?;
        
        // 回填 Frame Length
        let frame_length = buf.len() as u32;
        let mut cursor = Cursor::new(&mut buf[length_pos..length_pos + 4]);
        cursor.write_u32::<LittleEndian>(frame_length)?;
        
        Ok(buf)
    }
    
    pub fn deserialize(data: &[u8]) -> crate::Result<Self> {
        let mut cursor = Cursor::new(data);
        
        // 验证 Magic Header
        let magic = cursor.read_u32::<LittleEndian>()?;
        if magic != MAGIC_HEADER {
            return Err(crate::ScribeError::InvalidFrame);
        }
        
        // Frame Length
        let _length = cursor.read_u32::<LittleEndian>()?;
        
        // Timestamp
        let timestamp = cursor.read_i64::<LittleEndian>()?;
        
        // Level
        let level = match cursor.read_u8()? {
            0 => LogLevel::Verbose,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => return Err(crate::ScribeError::InvalidFrame),
        };
        
        // Tag
        let tag_len = cursor.read_u16::<LittleEndian>()? as usize;
        let mut tag_bytes = vec![0u8; tag_len];
        std::io::Read::read_exact(&mut cursor, &mut tag_bytes)?;
        let tag = String::from_utf8_lossy(&tag_bytes).to_string();
        
        // Message
        let msg_len = cursor.read_u32::<LittleEndian>()? as usize;
        let mut msg_bytes = vec![0u8; msg_len];
        std::io::Read::read_exact(&mut cursor, &mut msg_bytes)?;
        let message = String::from_utf8_lossy(&msg_bytes).to_string();
        
        // CRC32
        let crc_stored = cursor.read_u32::<LittleEndian>()?;
        
        // 验证 CRC
        let mut hasher = Hasher::new();
        hasher.update(&data[8..data.len() - 4]);
        let crc_calculated = hasher.finalize();
        
        if crc_stored != crc_calculated {
            return Err(crate::ScribeError::CrcMismatch);
        }
        
        Ok(Self {
            timestamp,
            level,
            tag,
            message,
        })
    }
}
```

**测试：`tests/unit/frame_test.rs`**

```rust
use scribe::storage::frame::{LogFrame, LogLevel};

#[test]
fn test_frame_serialize_deserialize() {
    let frame = LogFrame::new(
        LogLevel::Info,
        "test".to_string(),
        "hello world".to_string(),
    );
    
    let serialized = frame.serialize().unwrap();
    let deserialized = LogFrame::deserialize(&serialized).unwrap();
    
    assert_eq!(deserialized.level, LogLevel::Info);
    assert_eq!(deserialized.tag, "test");
    assert_eq!(deserialized.message, "hello world");
}

#[test]
fn test_frame_crc_validation() {
    let frame = LogFrame::new(LogLevel::Error, "test".to_string(), "error".to_string());
    
    let mut serialized = frame.serialize().unwrap();
    
    // 篡改数据
    serialized[20] ^= 0xFF;
    
    // 验证失败
    assert!(LogFrame::deserialize(&serialized).is_err());
}
```

**验收标准：**
- 序列化/反序列化测试通过
- CRC 验证测试通过
- `cargo test` 成功

---

### 1.3 实现单缓冲区 mmap (2 天)

**文件：`src/storage/buffer.rs`**

```rust
use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct MmapBuffer {
    mmap: MmapMut,
    position: AtomicUsize,
    capacity: usize,
    file_path: PathBuf,
}

impl MmapBuffer {
    pub fn new(file_path: PathBuf, capacity: usize) -> crate::Result<Self> {
        // 创建或打开文件
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file_path)?;
        
        // 设置文件大小
        file.set_len(capacity as u64)?;
        
        // 创建 mmap
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        
        Ok(Self {
            mmap,
            position: AtomicUsize::new(0),
            capacity,
            file_path,
        })
    }
    
    pub fn write(&self, data: &[u8]) -> crate::Result<usize> {
        let data_len = data.len();
        
        // 原子地获取并更新位置
        let pos = self.position.fetch_add(data_len, Ordering::AcqRel);
        
        if pos + data_len > self.capacity {
            return Err(crate::ScribeError::BufferFull);
        }
        
        // 写入数据
        unsafe {
            let ptr = self.mmap.as_ptr().add(pos) as *mut u8;
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data_len);
        }
        
        Ok(pos)
    }
    
    pub fn position(&self) -> usize {
        self.position.load(Ordering::Acquire)
    }
    
    pub fn is_full(&self, threshold: f32) -> bool {
        let pos = self.position();
        pos as f32 >= (self.capacity as f32 * threshold)
    }
    
    pub fn flush(&self) -> crate::Result<()> {
        self.mmap.flush()?;
        Ok(())
    }
    
    pub fn reset(&mut self) {
        self.position.store(0, Ordering::Release);
    }
}
```

**测试：`tests/unit/buffer_test.rs`**

```rust
use scribe::storage::buffer::MmapBuffer;
use tempfile::NamedTempFile;

#[test]
fn test_buffer_write() {
    let temp_file = NamedTempFile::new().unwrap();
    let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 4096).unwrap();
    
    let data = b"hello world";
    let pos = buffer.write(data).unwrap();
    
    assert_eq!(pos, 0);
    assert_eq!(buffer.position(), data.len());
}

#[test]
fn test_buffer_full() {
    let temp_file = NamedTempFile::new().unwrap();
    let buffer = MmapBuffer::new(temp_file.path().to_path_buf(), 100).unwrap();
    
    let data = vec![0u8; 120];
    let result = buffer.write(&data);
    
    assert!(result.is_err());
}
```

**验收标准：**
- 写入测试通过
- 缓冲区满检测通过
- `cargo test` 成功

---

### 1.4 实现 FFI 接口 (1 天)

**文件：`src/lib.rs`**

```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

mod error;
mod storage;

#[no_mangle]
pub extern "C" fn scribe_init(
    log_dir: *const c_char,
    capacity: usize,
) -> i32 {
    if log_dir.is_null() {
        return -1;
    }
    
    // TODO: 实现初始化逻辑
    0
}

#[no_mangle]
pub extern "C" fn scribe_write(
    level: i32,
    tag: *const c_char,
    message: *const c_char,
) -> i32 {
    if tag.is_null() || message.is_null() {
        return -1;
    }
    
    let tag_str = unsafe {
        CStr::from_ptr(tag).to_string_lossy().to_string()
    };
    
    let msg_str = unsafe {
        CStr::from_ptr(message).to_string_lossy().to_string()
    };
    
    // TODO: 实现写入逻辑
    0
}

#[no_mangle]
pub extern "C" fn scribe_flush() -> i32 {
    // TODO: 实现刷新逻辑
    0
}

#[no_mangle]
pub extern "C" fn scribe_destroy() -> i32 {
    // TODO: 实现销毁逻辑
    0
}
```

**验收标准：**
- FFI 接口编译成功
- 能够从 C 代码调用（创建简单的 C 测试）

---

## Phase 2: 并发与安全 (1 周)

### 目标
实现双缓冲区和无锁交换机制。

### 2.1 实现双缓冲区管理器 (2 天)
### 2.2 实现无锁交换算法 (2 天)
### 2.3 实现后台 Worker 线程 (1 天)
### 2.4 多线程压力测试 (2 天)

（详细任务分解类似 Phase 1）

---

## Phase 3: 持久化与加密 (1 周)

### 目标
集成压缩、加密和磁盘持久化。

### 3.1 集成 zstd 压缩 (1 天)
### 3.2 集成 ChaCha20 加密 (1 天)
### 3.3 实现磁盘持久化 (2 天)
### 3.4 实现存储清理策略 (3 天)

---

## Phase 4: 平台集成 (3-5 天)

### 目标
适配 Android 和 iOS 平台。

### 4.1 Android logcat 集成 (1 天)
### 4.2 iOS os_log 集成 (1 天)
### 4.3 多进程隔离 (PID-based) (1 天)
### 4.4 平台测试 (2 天)

---

## Phase 5: 错误处理与优化 (1 周)

### 目标
完善错误处理、性能优化、文档和示例。

### 5.1 崩溃恢复逻辑 (2 天)
### 5.2 磁盘空间不足降级 (1 天)
### 5.3 性能基准测试和优化 (2 天)
### 5.4 文档和示例 (2 天)

---

## 里程碑

| Phase | 目标 | 预计时间 | 关键验收 |
|-------|------|---------|---------|
| Phase 0 | 项目初始化 | 1-2 天 | Cargo 项目创建，依赖配置完成 |
| Phase 1 | 核心基础 (MVP) | 1 周 | 单缓冲区 mmap 写入成功 |
| Phase 2 | 并发与安全 | 1 周 | 双缓冲区无锁交换通过压力测试 |
| Phase 3 | 持久化与加密 | 1 周 | 完整的日志写入、压缩、加密流程 |
| Phase 4 | 平台集成 | 3-5 天 | Android + iOS 平台测试通过 |
| Phase 5 | 错误处理与优化 | 1 周 | 生产级特性完成，性能达标 |

**总计：4-6 周**

---

## 风险与缓解

### 风险 1: 无锁算法正确性
- **风险等级**：高
- **缓解措施**：
  - 使用 `loom` 进行并发测试
  - 参考 Rust 标准库的无锁实现
  - 编写详尽的多线程压力测试

### 风险 2: mmap 在不同平台的行为差异
- **风险等级**：中
- **缓解措施**：
  - 在 macOS、Linux、Android 上分别测试
  - 查阅 `memmap2` 的跨平台文档
  - 预留平台特定代码路径

### 风险 3: 性能达不到目标 (100ns 写入延迟)
- **风险等级**：中
- **缓解措施**：
  - 使用 `criterion` 进行微基准测试
  - 使用 `perf` 分析热点
  - 优先优化热路径

---

## 下一步行动

1. **立即开始 Phase 0**：初始化 Cargo 项目
2. **创建任务看板**：使用 GitHub Issues 或其他工具跟踪进度
3. **每周回顾**：每周五回顾进度，调整计划

**准备开始实现了吗？**
