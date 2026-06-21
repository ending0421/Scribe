     STDIN
   1 # Scribe 示例代码
   2 
   3 本目录包含了 Scribe 日志库的完整使用示例。
   4 
   5 ## 示例列表
   6 
   7 ### 1. complete.rs - 完整使用示例
   8 
   9 展示 Scribe 的完整工作流程，包括：
  10 - 初始化配置和存储
  11 - 写入不同级别的日志
  12 - 查看性能指标 (Metrics)
  13 - 执行清理操作
  14 - 查看生成的日志文件
  15 
  16 **运行命令：**
  17 ```bash
  18 cargo run --example complete
  19 ```
  20 
  21 **适用场景：**
  22 - 快速了解 Scribe 的基本功能
  23 - 学习完整的使用流程
  24 - 作为项目集成的参考
  25 
  26 ---
  27 
  28 ### 2. recovery.rs - 崩溃恢复示例
  29 
  30 演示 Scribe 的崩溃恢复能力：
  31 - 写入日志数据
  32 - 模拟程序崩溃（不执行 flush）
  33 - 重新启动并恢复数据
  34 - 验证数据完整性
  35 
  36 **运行命令：**
  37 ```bash
  38 cargo run --example recovery
  39 ```
  40 
  41 **适用场景：**
  42 - 测试崩溃恢复功能
  43 - 验证数据持久性
  44 - 了解内存映射的优势
  45 
  46 **交互式选项：**
  47 1. 写入日志并模拟崩溃
  48 2. 恢复崩溃前的数据
  49 3. 完整流程（写入 -> 崩溃 -> 恢复）
  50 
  51 ---
  52 
  53 ### 3. custom_stage.rs - 自定义 Pipeline Stage 示例
  54 
  55 展示如何实现自定义的数据处理阶段：
  56 - 日志过滤器（按级别过滤）
  57 - 敏感信息脱敏器（替换密码、token 等）
  58 - 统计收集器（收集日志统计信息）
  59 - 错误处理和 Fallback 策略
  60 
  61 **运行命令：**
  62 ```bash
  63 cargo run --example custom_stage
  64 ```
  65 
  66 **适用场景：**
  67 - 实现自定义数据处理逻辑
  68 - 添加日志过滤和转换
  69 - 集成数据脱敏功能
  70 - 学习 Pipeline 架构
  71 
  72 **包含的自定义 Stage：**
  73 - `LogFilterStage` - 按日志级别过滤
  74 - `SensitiveDataSanitizerStage` - 敏感信息脱敏
  75 - `StatisticsStage` - 统计收集
  76 - `FlakyStage` - 错误处理演示
  77 - `AbortOnErrorStage` - Abort 策略演示
  78 
  79 ---
  80 
  81 ### 4. metrics.rs - 性能指标使用示例
  82 
  83 详细展示 Metrics 系统的使用：
  84 - 收集写入、刷新、压缩、加密等操作的指标
  85 - 获取性能快照并分析
  86 - 导出到监控系统（Prometheus、JSON）
  87 - 定期重置指标
  88 - 性能基准测试
  89 
  90 **运行命令：**
  91 ```bash
  92 cargo run --example metrics
  93 ```
  94 
  95 **适用场景：**
  96 - 性能监控和分析
  97 - 集成到监控系统（如 Prometheus、Grafana）
  98 - 性能基准测试
  99 - 故障排查
 100 
 101 **导出格式：**
 102 - Prometheus 格式
 103 - JSON 格式
 104 - 自定义格式扩展
 105 
 106 ---
 107 
 108 ## 快速开始
 109 
 110 ### 1. 安装依赖
 111 
 112 ```bash
 113 cd /path/to/scribe
 114 cargo build --examples
 115 ```
 116 
 117 ### 2. 运行示例
 118 
 119 选择任意示例运行：
 120 
 121 ```bash
 122 # 完整使用流程
 123 cargo run --example complete
 124 
 125 # 崩溃恢复测试
 126 cargo run --example recovery
 127 
 128 # 自定义 Pipeline
 129 cargo run --example custom_stage
 130 
 131 # 性能指标监控
 132 cargo run --example metrics
 133 ```
 134 
 135 ### 3. 查看输出
 136 
 137 所有示例都会在 `/tmp/` 目录下创建日志文件：
 138 - `/tmp/scribe_example/` - complete 示例
 139 - `/tmp/scribe_recovery_example/` - recovery 示例
 140 - `/tmp/scribe_metrics_example/` - metrics 示例
 141 
 142 ---
 143 
 144 ## 示例特点
 145 
 146 ✅ **详细注释** - 每个示例都有详细的中文注释  
 147 ✅ **完整流程** - 涵盖初始化、使用、清理的完整流程  
 148 ✅ **错误处理** - 演示正确的错误处理方式  
 149 ✅ **最佳实践** - 展示推荐的使用模式  
 150 ✅ **可运行** - 所有示例都可以直接运行  
 151 
 152 ---
 153 
 154 ## 进阶主题
 155 
 156 ### Pipeline 处理
 157 
 158 Pipeline 提供了灵活的数据处理架构：
 159 
 160 ```rust
 161 let pipeline = Pipeline::new()
 162     .add_stage(Box::new(CompressionStage))
 163     .add_stage(Box::new(EncryptionStage))
 164     .add_stage(Box::new(CustomStage));
 165 ```
 166 
 167 ### Router 路由
 168 
 169 根据条件选择不同的处理管道：
 170 
 171 ```rust
 172 let router = Router::new()
 173     .route(|frame| frame.level == LogLevel::Error, error_pipeline)
 174     .route(|frame| frame.tag == "security", security_pipeline)
 175     .default(default_pipeline);
 176 ```
 177 
 178 ### Metrics 监控
 179 
 180 持续收集性能指标：
 181 
 182 ```rust
 183 let metrics = ScribeMetrics::new();
 184 metrics.record_write(bytes);
 185 let snapshot = metrics.snapshot();
 186 println!("写入成功率: {:.2}%", snapshot.write_success_rate() * 100.0);
 187 ```
 188 
 189 ---
 190 
 191 ## 常见问题
 192 
 193 ### Q: 如何在移动端使用？
 194 
 195 Scribe 提供了 C FFI 接口，可以通过 JNI（Android）或 Swift（iOS）调用：
 196 
 197 ```c
 198 // C/C++ 调用
 199 scribe_init("/path/to/logs");
 200 scribe_write(2, "MyTag", "Log message");
 201 scribe_flush();
 202 scribe_destroy();
 203 ```
 204 
 205 ### Q: 如何处理大量日志？
 206 
 207 使用 Pipeline 进行压缩和批处理：
 208 
 209 ```rust
 210 let pipeline = Pipeline::new()
 211     .add_stage(Box::new(CompressionStage))
 212     .add_stage(Box::new(BatchStage));
 213 ```
 214 
 215 ### Q: 如何自定义清理策略？
 216 
 217 配置 `CleanupPolicy`：
 218 
 219 ```rust
 220 let policy = CleanupPolicy {
 221     max_total_size: Some(100 * 1024 * 1024),  // 100MB
 222     max_age: Some(Duration::from_secs(7 * 24 * 60 * 60)),  // 7天
 223     min_free_space: Some(50 * 1024 * 1024),  // 50MB
 224 };
 225 ```
 226 
 227 ---
 228 
 229 ## 更多资源
 230 
 231 - 📖 [API 文档](../README.md)
 232 - 🔧 [配置指南](../docs/configuration.md)
 233 - 🚀 [性能优化](../docs/performance.md)
 234 - 🐛 [故障排查](../docs/troubleshooting.md)
 235 
 236 ---
 237 
 238 ## 贡献
 239 
 240 欢迎提交新的示例代码！请确保：
 241 1. 代码可以正常编译运行
 242 2. 包含详细的注释说明
 243 3. 演示特定的使用场景
 244 4. 遵循项目的代码风格
 245 
 246 ---
 247 
 248 ## 许可证
 249 
 250 MIT License - 详见 [LICENSE](../LICENSE) 文件
