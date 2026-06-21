     STDIN
   1 # Scribe Release 流程
   2 
   3 本文档说明如何发布新版本的 Scribe，包括 Android AAR、iOS XCFramework 和 Swift Package Manager。
   4 
   5 ## 发布流程
   6 
   7 ### 1. 准备发布
   8 
   9 确保所有测试通过：
  10 ```bash
  11 cargo test
  12 cargo clippy --lib --bins --tests -- -D warnings
  13 cargo fmt --all -- --check
  14 ```
  15 
  16 ### 2. 创建 Git Tag
  17 
  18 ```bash
  19 # 创建并推送 tag（例如 v1.0.0）
  20 git tag v1.0.0
  21 git push origin v1.0.0
  22 ```
  23 
  24 ### 3. CI 自动构建
  25 
  26 推送 tag 后，CI 会自动：
  27 - 构建 Android AAR
  28 - 构建 iOS XCFramework
  29 - 创建 GitHub Release
  30 - 上传构件到 Release
  31 
  32 等待 CI 完成（约 6 分钟）。
  33 
  34 ### 4. 更新 Package.swift 以支持 SPM
  35 
  36 下载 Release 中的 `Scribe.xcframework.zip`，计算 checksum：
  37 
  38 ```bash
  39 # 下载 XCFramework
  40 curl -L -o Scribe.xcframework.zip \
  41   https://github.com/ending0421/Scribe/releases/download/v1.0.0/Scribe.xcframework.zip
  42 
  43 # 计算 checksum
  44 swift package compute-checksum Scribe.xcframework.zip
  45 ```
  46 
  47 使用脚本更新 Package.swift：
  48 
  49 ```bash
  50 # 例如: ./scripts/update-package-for-release.sh 1.0.0 <checksum>
  51 ./scripts/update-package-for-release.sh 1.0.0 abc123def456...
  52 ```
  53 
  54 ### 5. 提交更新后的 Package.swift
  55 
  56 ```bash
  57 git add Package.swift
  58 git commit -m "chore: update Package.swift for v1.0.0 release"
  59 git push origin master
  60 
  61 # 也更新到 tag（可选，让 SPM 从 tag 拉取）
  62 git tag -d v1.0.0
  63 git push origin :refs/tags/v1.0.0
  64 git tag v1.0.0
  65 git push origin v1.0.0
  66 ```
  67 
  68 ### 6. 验证 SPM 集成
  69 
  70 创建测试项目验证：
  71 
  72 ```swift
  73 // Package.swift
  74 dependencies: [
  75     .package(url: "https://github.com/ending0421/Scribe.git", from: "1.0.0")
  76 ]
  77 ```
  78 
  79 或在 Xcode 中：
  80 ```
  81 File → Add Package Dependencies
  82 URL: https://github.com/ending0421/Scribe.git
  83 Version: 1.0.0
  84 ```
  85 
  86 ## 标准工作流程总结
  87 
  88 ```
  89 1. 创建 tag (v1.0.0)
  90    ↓
  91 2. CI 自动构建并发布到 GitHub Releases
  92    ↓
  93 3. 下载 XCFramework.zip 并计算 checksum
  94    ↓
  95 4. 更新 Package.swift 指向 Release URL
  96    ↓
  97 5. 提交并推送更新
  98    ↓
  99 6. 验证 SPM 可以正常拉取
 100 ```
 101 
 102 ## 版本号规范
 103 
 104 遵循 [Semantic Versioning](https://semver.org/)：
 105 
 106 - **MAJOR** (1.x.x): 不兼容的 API 变更
 107 - **MINOR** (x.1.x): 向后兼容的新功能
 108 - **PATCH** (x.x.1): 向后兼容的 bug 修复
 109 
 110 ## 产物说明
 111 
 112 每个 Release 包含：
 113 
 114 1. **scribe-release.aar** - Android 库
 115    - 支持 ARMv7 和 ARM64
 116    - 可通过 Gradle/Maven 本地引用
 117 
 118 2. **Scribe.xcframework.zip** - iOS 框架
 119    - 支持 iOS 设备 (ARM64)
 120    - 支持 iOS 模拟器 (ARM64 + x86_64)
 121    - 可通过 SPM 或手动集成
 122 
 123 ## 自动化改进（未来）
 124 
 125 可以考虑在 CI 中自动化步骤 4-5：
 126 - CI 计算 checksum
 127 - CI 自动更新 Package.swift
 128 - CI 自动提交和推送
 129 
 130 这需要配置 GitHub Actions 的写权限。
