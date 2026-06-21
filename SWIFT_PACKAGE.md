     STDIN
   1 # Scribe Swift Package Manager
   2 
   3 Scribe 现在支持 Swift Package Manager (SPM)，让 iOS/macOS 集成更加简单！
   4 
   5 ## 安装
   6 
   7 ### 方式 1：通过 Xcode（推荐）
   8 
   9 1. 在 Xcode 中打开你的项目
  10 2. 选择 File → Add Package Dependencies...
  11 3. 输入仓库 URL：`https://github.com/ending0421/Scribe.git`
  12 4. 选择版本规则（建议使用最新版本）
  13 5. 点击 Add Package
  14 
  15 ### 方式 2：通过 Package.swift
  16 
  17 在你的 `Package.swift` 文件中添加：
  18 
  19 ```swift
  20 dependencies: [
  21     .package(url: "https://github.com/ending0421/Scribe.git", from: "1.0.0")
  22 ],
  23 targets: [
  24     .target(
  25         name: "YourTarget",
  26         dependencies: ["Scribe"]
  27     )
  28 ]
  29 ```
  30 
  31 ## 使用
  32 
  33 ### 初始化
  34 
  35 ```swift
  36 import Scribe
  37 
  38 // 使用默认配置初始化
  39 try Scribe.initialize()
  40 
  41 // 或者使用自定义配置文件
  42 try Scribe.initialize(configPath: "/path/to/config.json")
  43 ```
  44 
  45 ### 基础日志记录
  46 
  47 ```swift
  48 // 便捷方法（推荐）
  49 Scribe.verbose("Verbose message")
  50 Scribe.debug("Debug message")
  51 Scribe.info("Info message")
  52 Scribe.warn("Warning message")
  53 Scribe.error("Error message")
  54 
  55 // 带自定义标签
  56 Scribe.info("User logged in", label: "Auth")
  57 Scribe.error("Network timeout", label: "Network")
  58 ```
  59 
  60 ### 完整 API
  61 
  62 ```swift
  63 // 使用 LogLevel 枚举
  64 try Scribe.log(.info, label: "MyApp", message: "Something happened")
  65 
  66 // 刷新日志到磁盘
  67 try Scribe.flush()
  68 
  69 // 获取性能统计
  70 let stats = try Scribe.getStatistics()
  71 print("Total writes: \(stats.totalWrites)")
  72 print("Bytes written: \(stats.bytesWritten)")
  73 print("Total errors: \(stats.totalErrors)")
  74 ```
  75 
  76 ### 错误处理
  77 
  78 ```swift
  79 do {
  80     try Scribe.initialize()
  81     try Scribe.log(.info, label: "App", message: "Started")
  82 } catch let error as ScribeError {
  83     print("Scribe error: \(error.localizedDescription)")
  84 } catch {
  85     print("Unknown error: \(error)")
  86 }
  87 ```
  88 
  89 ## 对比传统方式
  90 
  91 ### 传统方式：使用 XCFramework
  92 
  93 ```swift
  94 // 需要手动管理头文件和二进制
  95 // 需要配置 Bridging Header
  96 // 手动处理 C API
  97 
  98 import ScribeFFI
  99 
 100 let result = scribe_init(nil)
 101 if result != 0 {
 102     // 手动处理错误
 103 }
 104 scribe_log(2, "App", "Message")
 105 ```
 106 
 107 ### SPM 方式：更简洁
 108 
 109 ```swift
 110 import Scribe
 111 
 112 // 类型安全、Swift 风格的 API
 113 try Scribe.initialize()
 114 Scribe.info("Message", label: "App")
 115 ```
 116 
 117 ## 性能
 118 
 119 - ✅ 零拷贝字符串传递
 120 - ✅ 与直接使用 XCFramework 性能相同
 121 - ✅ Swift wrapper 仅是薄层封装
 122 
 123 ## 平台支持
 124 
 125 - iOS 13.0+
 126 - macOS 10.15+
 127 
 128 ## 示例项目
 129 
 130 查看 `Examples/iOS` 目录获取完整的示例项目。
 131 
 132 ## 传统集成方式
 133 
 134 如果你更喜欢传统方式，可以直接下载 XCFramework：
 135 
 136 1. 从 [Releases](https://github.com/ending0421/Scribe/releases) 下载 `Scribe.xcframework.zip`
 137 2. 解压并拖入 Xcode 项目
 138 3. 在 Build Settings 中添加头文件路径
 139 
 140 ## Android 集成
 141 
 142 Android 请使用 AAR 文件，参见 [Android 集成文档](ANDROID_INTEGRATION.md)。
