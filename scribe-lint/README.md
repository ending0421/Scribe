# Scribe Lint - 自定义 Clippy Lint 规则

## 简介

Scribe Lint 提供编译时静态分析，确保 Scribe 日志 API 的正确使用。

## Lint 规则

### 错误级别（Deny）

#### 1. ScribeArgCount
检测格式化参数数量不匹配

```rust
// ❌ 错误
scribe_d!("Value: {} {}", value);  // 缺少一个参数

// ✅ 正确
scribe_d!("Value: {} {}", value1, value2);
```

#### 2. ScribeArgTypes
检测格式化参数类型错误

```rust
// ❌ 错误
scribe_d!("Value: {:x}", "string");  // 十六进制不支持字符串

// ✅ 正确
scribe_d!("Value: {:x}", 255);
```

#### 3. ScribeTagLength
检测 Tag 长度超过 23 字符（Android 限制）

```rust
// ❌ 错误
scribe_tag_d!("ThisIsAVeryLongTagName_MoreThan23", "message");

// ✅ 正确
scribe_tag_d!("ShortTag", "message");
```

### 警告级别（Warn）

#### 4. ScribeLogUsage
检测使用原生日志而非 Scribe

```rust
// ⚠️ 警告
println!("Debug: {}", value);

// ✅ 推荐
scribe_d!("Debug: {}", value);
```

#### 5. ScribeManualFormat
检测手动使用 format!

```rust
// ⚠️ 警告
scribe_d!("{}", format!("Value: {}", val));

// ✅ 推荐
scribe_d!("Value: {}", val);
```

#### 6. ScribeStringConcat
检测字符串拼接

```rust
// ⚠️ 警告
scribe_d!("{}", "Value: " + value);

// ✅ 推荐
scribe_d!("Value: {}", value);
```

#### 7. ScribeErrorFormat
检测错误日志格式不规范

```rust
// ⚠️ 警告
scribe_e!("Error: {}", error.to_string());

// ✅ 推荐
scribe_e!("Error: {}", error);
```

## 使用方法

### 作为 Clippy Lint 使用

在项目根目录创建 `clippy.toml`：

```toml
# 启用 Scribe Lint
[lints.scribe]
arg_count = "deny"
arg_types = "deny"
tag_length = "deny"
log_usage = "warn"
manual_format = "warn"
string_concat = "warn"
error_format = "warn"
```

### 运行 Lint 检查

```bash
cargo clippy -- -W clippy::scribe_arg_count \
                -W clippy::scribe_arg_types \
                -W clippy::scribe_tag_length \
                -W clippy::scribe_log_usage \
                -W clippy::scribe_manual_format \
                -W clippy::scribe_string_concat \
                -W clippy::scribe_error_format
```

## 开发

### 构建

```bash
cd scribe-lint
cargo build
```

### 测试

```bash
cargo test
```

## 实现状态

- ✅ ScribeArgCount - 完整实现
- ✅ ScribeArgTypes - 基础框架
- ✅ ScribeTagLength - 完整实现
- ✅ ScribeLogUsage - 完整实现
- ✅ ScribeManualFormat - 基础框架
- ✅ ScribeStringConcat - 基础框架
- ✅ ScribeErrorFormat - 基础框架

## 许可证

与 Scribe 主项目保持一致
