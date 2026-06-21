# Scribe 跨平台构建指南

## 构建工具链

### Android 构建

#### 前提条件
- Rust 工具链
- Android NDK r26c+
- cargo-ndk (可选)

#### 安装 Android 目标

```bash
rustup target add armv7-linux-androideabi
rustup target add aarch64-linux-android
rustup target add i686-linux-android
rustup target add x86_64-linux-android
```

#### 构建 Android 库

```bash
# ARMv7 (32-bit ARM)
cargo build --release --target armv7-linux-androideabi

# ARMv8 (64-bit ARM)
cargo build --release --target aarch64-linux-android

# x86 (模拟器)
cargo build --release --target i686-linux-android

# x86_64 (模拟器)
cargo build --release --target x86_64-linux-android
```

#### 构建 AAR

```bash
# 复制编译好的 .so 文件到 jniLibs
mkdir -p android/src/main/jniLibs/armeabi-v7a
mkdir -p android/src/main/jniLibs/arm64-v8a

cp target/armv7-linux-androideabi/release/libscribe.so android/src/main/jniLibs/armeabi-v7a/
cp target/aarch64-linux-android/release/libscribe.so android/src/main/jniLibs/arm64-v8a/

# 构建 AAR
cd android
./gradlew assembleRelease

# 输出: android/build/outputs/aar/scribe-release.aar
```

### iOS 构建

#### 前提条件
- Rust 工具链
- Xcode 15+
- macOS

#### 安装 iOS 目标

```bash
rustup target add aarch64-apple-ios          # Device (ARM64)
rustup target add aarch64-apple-ios-sim      # Simulator (Apple Silicon)
rustup target add x86_64-apple-ios           # Simulator (Intel)
```

#### 构建 iOS 库

```bash
# 设备 (ARM64)
cargo build --release --target aarch64-apple-ios

# 模拟器 (Apple Silicon)
cargo build --release --target aarch64-apple-ios-sim

# 模拟器 (Intel)
cargo build --release --target x86_64-apple-ios
```

#### 创建 XCFramework

```bash
#!/bin/bash

# 创建 fat 库（合并模拟器架构）
lipo -create \
  target/aarch64-apple-ios-sim/release/libscribe.a \
  target/x86_64-apple-ios/release/libscribe.a \
  -output libscribe-simulator.a

# 创建 XCFramework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libscribe.a \
  -headers ios/Scribe.h \
  -library libscribe-simulator.a \
  -headers ios/Scribe.h \
  -output Scribe.xcframework
```

## CI/CD

### GitHub Actions

工作流文件位于 `.github/workflows/build.yml`，提供：

- ✅ 代码格式检查和 Clippy
- ✅ 跨平台测试 (Linux, macOS)
- ✅ Android 构建 (ARMv7, ARMv8)
- ✅ Android AAR 打包
- ✅ iOS 构建 (Device, Simulator)
- ✅ iOS XCFramework 创建
- ✅ 自动发布 (tag 触发)

### 本地测试

```bash
# 运行所有测试
cargo test

# 运行基准测试
cargo bench

# 检查代码格式
cargo fmt --all -- --check

# 运行 Clippy
cargo clippy --all-targets --all-features -- -D warnings
```

## 使用指南

### Android (Kotlin)

```kotlin
// 初始化
Scribe.init(context.filesDir.absolutePath + "/logs")

// 开发环境：启用控制台输出
Scribe.registerConsole(Scribe.LogLevel.DEBUG.value)

// 记录日志
Scribe.d("MyTag", "Debug message")
Scribe.i("MyTag", "Info message")
Scribe.e("MyTag", "Error message")

// 刷新到磁盘
Scribe.flush()

// 清理
Scribe.destroy()
```

### iOS (Swift)

```swift
// 初始化
let logDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
Scribe.init(withLogDir: logDir.path + "/logs")

// 开发环境：启用控制台输出
Scribe.registerConsole(withMinLevel: .debug)

// 记录日志
Scribe.d("MyTag", message: "Debug message")
Scribe.i("MyTag", message: "Info message")
Scribe.e("MyTag", message: "Error message")

// 刷新到磁盘
Scribe.flush()

// 清理
Scribe.destroy()
```

## 故障排查

### Android

**问题：找不到 libscribe.so**
```
解决：确保 .so 文件在正确的 jniLibs 目录下
```

**问题：UnsatisfiedLinkError**
```
解决：检查 ABI 是否匹配，确保编译了正确的架构
```

### iOS

**问题：符号未找到**
```
解决：确保 XCFramework 包含了正确的架构
```

**问题：模拟器构建失败**
```
解决：检查是否安装了 Apple Silicon 和 Intel 两个模拟器目标
```
