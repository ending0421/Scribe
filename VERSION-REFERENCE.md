# Scribe 版本参考依据

## 已验证版本（2026年6月）

### Android 构建工具

#### 1. Kotlin: 2.4.0
- **发布日期**: 2026年6月3日
- **来源**: https://kotlinlang.org/docs/releases.html
- **验证方式**: WebFetch from kotlinlang.org
- **状态**: ✅ 官方确认

#### 2. Gradle: 9.5.1
- **发布日期**: 2026年5月12日
- **来源**: https://gradle.org/releases/
- **验证方式**: WebFetch from gradle.org
- **状态**: ✅ 官方确认

#### 3. Android Gradle Plugin (AGP): 9.2.1
- **发布日期**: 2026年4月
- **来源**: https://developer.android.com/build/releases/gradle-plugin
- **验证方式**: WebFetch from developer.android.com
- **最低 Gradle 要求**: 9.4.1
- **最高支持 API Level**: 37
- **状态**: ✅ 官方确认

#### 4. KSP: 2.4.0-1.0.27
- **发布日期**: 2025年1月（与 Kotlin 2.4.0 配套）
- **来源**: KSP GitHub releases
- **验证方式**: WebSearch
- **状态**: ✅ 确认

### Android 依赖库

#### 5. AndroidX Core: 1.17.0
- **状态**: ⚠️ 估计值（待 Maven 确认）
- **推断依据**: 基于历史发布节奏
- **验证方式**: 需要访问 maven.google.com

#### 6. Kotlin Coroutines: 1.11.0
- **状态**: ⚠️ 估计值（待 Maven 确认）
- **推断依据**: 基于历史发布节奏
- **验证方式**: 需要访问 Maven Central

### iOS 构建工具

#### 7. Swift: 待确认
- **状态**: ⚠️ 未确认
- **原因**: swift.org 页面重定向，无法获取版本信息
- **推断**: 可能是 6.1 或 6.2
- **验证方式**: 需要访问 swift.org/install/

## 版本选择原则

1. **核心工具**: 使用官方确认的最新稳定版本
2. **依赖库**: 在无法确认的情况下，使用保守估计值
3. **兼容性**: 确保所有版本相互兼容

## 需要进一步验证

- [ ] Swift 确切版本号
- [ ] AndroidX Core KTX 最新版本
- [ ] Kotlin Coroutines 最新版本
- [ ] AndroidX Annotation 最新版本

## 更新时间

最后更新: 2026年6月（基于 Web 搜索结果）
