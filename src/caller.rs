use std::backtrace::Backtrace;

#[derive(Debug, Clone, PartialEq)]
pub struct StackTraceElement {
    pub class_name: Option<String>,
    pub method_name: Option<String>,
    pub file_name: Option<String>,
    pub line_number: Option<u32>,
}

/// 获取调用类名（自动跳过 scribe 内部调用）
pub fn get_calling_class() -> Option<String> {
    let backtrace = Backtrace::capture();
    parse_calling_class(&format!("{:?}", backtrace))
}

/// 获取完整调用栈
pub fn get_stack_trace() -> Vec<StackTraceElement> {
    let backtrace = Backtrace::capture();
    parse_stack_trace(&format!("{:?}", backtrace))
}

/// 解析调用类名
fn parse_calling_class(backtrace: &str) -> Option<String> {
    for line in backtrace.lines() {
        // 跳过 scribe:: 和 std:: 的调用
        if line.contains("::")
            && !line.contains("scribe::")
            && !line.contains("std::")
            && !line.contains("core::")
            && !line.contains("alloc::")
            && !line.contains("backtrace::")
        {
            if let Some(class) = extract_class_from_line(line) {
                return Some(class);
            }
        }
    }
    None
}

/// 解析完整堆栈
fn parse_stack_trace(backtrace: &str) -> Vec<StackTraceElement> {
    let mut elements = Vec::new();

    for line in backtrace.lines() {
        if let Some(element) = parse_stack_line(line) {
            elements.push(element);
        }
    }

    elements
}

fn extract_class_from_line(line: &str) -> Option<String> {
    // 从 "at my_app::MainActivity::onCreate" 提取 "MainActivity"
    // 或从 "my_app::utils::Logger" 提取 "Logger"

    // 查找函数调用部分
    let line = line.trim();

    // 尝试多种格式
    // 格式1: "   3: my_app::MainActivity::onCreate"
    // 格式2: "at my_app::MainActivity::onCreate"

    let function_part = if let Some(idx) = line.find(": ") {
        &line[idx + 2..]
    } else if let Some(idx) = line.find("at ") {
        &line[idx + 3..]
    } else {
        line
    };

    // 提取到空格或换行符之前的部分
    let function_part = function_part.split_whitespace().next()?;

    // 按 :: 分割
    let parts: Vec<&str> = function_part.split("::").collect();

    // 至少需要 module::Class::method 三段
    if parts.len() >= 2 {
        // 倒数第二个部分是类名（倒数第一个是方法名）
        let class_name = parts[parts.len() - 2];

        // 过滤掉泛型参数
        let class_name = class_name.split('<').next()?;

        return Some(class_name.to_string());
    }

    None
}

fn parse_stack_line(line: &str) -> Option<StackTraceElement> {
    // 解析单行堆栈信息
    // 示例格式：
    //   3: my_app::MainActivity::onCreate
    //      at ./src/main.rs:42:5
    //   或
    //   at my_app::MainActivity::onCreate (src/main.rs:42)

    let line = line.trim();

    // 提取函数全路径
    let function_part = if let Some(idx) = line.find(": ") {
        &line[idx + 2..]
    } else if let Some(idx) = line.find("at ") {
        &line[idx + 3..]
    } else {
        return None;
    };

    let function_part = function_part.split_whitespace().next()?;

    // 解析路径：module::Class::method
    let parts: Vec<&str> = function_part.split("::").collect();

    let (class_name, method_name) = if parts.len() >= 2 {
        let method = parts.last()?.split('<').next()?.to_string();
        let class = parts[parts.len() - 2].split('<').next()?.to_string();
        (Some(class), Some(method))
    } else {
        (None, None)
    };

    // 提取文件名和行号
    // 格式1: "at ./src/main.rs:42:5"
    // 格式2: "(src/main.rs:42)"
    let (file_name, line_number) = extract_file_info(line);

    Some(StackTraceElement {
        class_name,
        method_name,
        file_name,
        line_number,
    })
}

fn extract_file_info(line: &str) -> (Option<String>, Option<u32>) {
    // 查找文件路径模式
    // 格式1: at ./src/main.rs:42:5
    // 格式2: (src/main.rs:42)

    if let Some(start) = line.find("at ./") {
        let rest = &line[start + 3..];
        return parse_file_path(rest);
    }

    if let Some(start) = line.find('(') {
        if let Some(end) = line.find(')') {
            let path_part = &line[start + 1..end];
            return parse_file_path(path_part);
        }
    }

    (None, None)
}

fn parse_file_path(path: &str) -> (Option<String>, Option<u32>) {
    let parts: Vec<&str> = path.split(':').collect();

    if parts.is_empty() {
        return (None, None);
    }

    let file_name = Some(parts[0].to_string());

    let line_number = if parts.len() >= 2 {
        parts[1].parse::<u32>().ok()
    } else {
        None
    };

    (file_name, line_number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_class_from_line_standard_format() {
        let line = "   3: my_app::MainActivity::onCreate";
        assert_eq!(
            extract_class_from_line(line),
            Some("MainActivity".to_string())
        );
    }

    #[test]
    fn test_extract_class_from_line_at_format() {
        let line = "at my_app::utils::Logger::log";
        assert_eq!(
            extract_class_from_line(line),
            Some("Logger".to_string())
        );
    }

    #[test]
    fn test_extract_class_from_line_with_generics() {
        let line = "   5: my_app::Container<T>::process";
        assert_eq!(
            extract_class_from_line(line),
            Some("Container".to_string())
        );
    }

    #[test]
    fn test_extract_class_from_line_nested_modules() {
        let line = "   7: my_app::features::auth::LoginScreen::render";
        assert_eq!(
            extract_class_from_line(line),
            Some("LoginScreen".to_string())
        );
    }

    #[test]
    fn test_extract_class_from_line_invalid() {
        let line = "   1: <unknown>";
        assert_eq!(extract_class_from_line(line), None);
    }

    #[test]
    fn test_parse_calling_class_skips_internal() {
        let backtrace = r#"
   0: std::backtrace::Backtrace::capture
   1: scribe::backtrace::get_calling_class
   2: scribe::logger::Logger::info
   3: my_app::MainActivity::onCreate
   4: std::rt::lang_start
"#;
        assert_eq!(
            parse_calling_class(backtrace),
            Some("MainActivity".to_string())
        );
    }

    #[test]
    fn test_parse_calling_class_skips_multiple_internal() {
        let backtrace = r#"
   0: core::backtrace::Backtrace::create
   1: std::backtrace::Backtrace::capture
   2: scribe::backtrace::get_calling_class
   3: scribe::logger::Logger::debug
   4: scribe::formatter::format_message
   5: my_app::services::UserService::fetch_user
   6: std::rt::lang_start
"#;
        assert_eq!(
            parse_calling_class(backtrace),
            Some("UserService".to_string())
        );
    }

    #[test]
    fn test_parse_calling_class_no_external_caller() {
        let backtrace = r#"
   0: std::backtrace::Backtrace::capture
   1: scribe::backtrace::get_calling_class
   2: core::ops::function::FnOnce::call_once
"#;
        assert_eq!(parse_calling_class(backtrace), None);
    }

    #[test]
    fn test_parse_stack_line_standard_format() {
        let line = "   3: my_app::MainActivity::onCreate";
        let element = parse_stack_line(line).unwrap();

        assert_eq!(element.class_name, Some("MainActivity".to_string()));
        assert_eq!(element.method_name, Some("onCreate".to_string()));
    }

    #[test]
    fn test_parse_stack_line_with_file_info_format1() {
        let line = "   5: my_app::Logger::log\n             at ./src/logger.rs:42:5";
        let element = parse_stack_line(line).unwrap();

        assert_eq!(element.class_name, Some("Logger".to_string()));
        assert_eq!(element.method_name, Some("log".to_string()));
    }

    #[test]
    fn test_parse_stack_line_with_file_info_format2() {
        let line = "at my_app::MainActivity::render (src/main.rs:100)";
        let element = parse_stack_line(line).unwrap();

        assert_eq!(element.class_name, Some("MainActivity".to_string()));
        assert_eq!(element.method_name, Some("render".to_string()));
    }

    #[test]
    fn test_extract_file_info_format1() {
        let line = "at ./src/main.rs:42:5";
        let (file, line_num) = extract_file_info(line);

        assert_eq!(file, Some("./src/main.rs".to_string()));
        assert_eq!(line_num, Some(42));
    }

    #[test]
    fn test_extract_file_info_format2() {
        let line = "(src/logger.rs:100)";
        let (file, line_num) = extract_file_info(line);

        assert_eq!(file, Some("src/logger.rs".to_string()));
        assert_eq!(line_num, Some(100));
    }

    #[test]
    fn test_extract_file_info_no_line_number() {
        let line = "(src/main.rs)";
        let (file, line_num) = extract_file_info(line);

        assert_eq!(file, Some("src/main.rs".to_string()));
        assert_eq!(line_num, None);
    }

    #[test]
    fn test_parse_stack_trace_multiple_frames() {
        let backtrace = r#"
   0: std::backtrace::Backtrace::capture
   1: scribe::backtrace::get_stack_trace
   2: my_app::MainActivity::onCreate
   3: my_app::App::run
   4: my_app::main
"#;
        let stack = parse_stack_trace(backtrace);

        assert!(stack.len() >= 3);

        // 查找 MainActivity 帧
        let main_activity_frame = stack.iter()
            .find(|e| e.class_name.as_deref() == Some("MainActivity"));
        assert!(main_activity_frame.is_some());

        let frame = main_activity_frame.unwrap();
        assert_eq!(frame.method_name, Some("onCreate".to_string()));
    }

    #[test]
    fn test_get_calling_class_integration() {
        // 这是一个集成测试，会实际捕获 backtrace
        fn outer_function() {
            fn inner_function() -> Option<String> {
                get_calling_class()
            }

            let class = inner_function();
            // 应该跳过 scribe 内部，找到这个测试模块
            assert!(class.is_some());
        }

        outer_function();
    }

    #[test]
    fn test_get_stack_trace_integration() {
        fn test_function() -> Vec<StackTraceElement> {
            get_stack_trace()
        }

        let stack = test_function();

        // 应该捕获到至少一些栈帧
        assert!(!stack.is_empty());

        // 验证栈帧包含我们的测试代码
        let has_test_frame = stack.iter().any(|e| {
            e.method_name.as_deref() == Some("test_function")
                || e.class_name.as_deref() == Some("tests")
        });

        assert!(has_test_frame);
    }

    #[test]
    fn test_parse_file_path() {
        let (file, line) = parse_file_path("src/main.rs:42:5");
        assert_eq!(file, Some("src/main.rs".to_string()));
        assert_eq!(line, Some(42));
    }

    #[test]
    fn test_parse_file_path_no_column() {
        let (file, line) = parse_file_path("src/logger.rs:100");
        assert_eq!(file, Some("src/logger.rs".to_string()));
        assert_eq!(line, Some(100));
    }

    #[test]
    fn test_parse_file_path_only_file() {
        let (file, line) = parse_file_path("src/app.rs");
        assert_eq!(file, Some("src/app.rs".to_string()));
        assert_eq!(line, None);
    }

    #[test]
    fn test_stack_trace_element_equality() {
        let elem1 = StackTraceElement {
            class_name: Some("MainActivity".to_string()),
            method_name: Some("onCreate".to_string()),
            file_name: Some("main.rs".to_string()),
            line_number: Some(42),
        };

        let elem2 = StackTraceElement {
            class_name: Some("MainActivity".to_string()),
            method_name: Some("onCreate".to_string()),
            file_name: Some("main.rs".to_string()),
            line_number: Some(42),
        };

        assert_eq!(elem1, elem2);
    }

    #[test]
    fn test_extract_class_handles_closure() {
        let line = "   8: my_app::Handler::{{closure}}";
        let class = extract_class_from_line(line);
        assert_eq!(class, Some("Handler".to_string()));
    }

    #[test]
    fn test_parse_calling_class_real_world_format() {
        let backtrace = r#"
stack backtrace:
   0: std::backtrace_rs::backtrace::libunwind::trace
             at /rustc/stable/library/std/src/../../backtrace/src/backtrace/libunwind.rs:93:5
   1: std::backtrace_rs::backtrace::trace_unsynchronized
             at /rustc/stable/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5
   2: std::backtrace::Backtrace::create
             at /rustc/stable/library/std/src/backtrace.rs:331:13
   3: std::backtrace::Backtrace::capture
             at /rustc/stable/library/std/src/backtrace.rs:296:9
   4: scribe::backtrace::get_calling_class
             at ./src/backtrace.rs:15:21
   5: my_app::services::AuthService::login
             at ./src/services/auth.rs:45:9
   6: my_app::main
             at ./src/main.rs:10:5
"#;
        let class = parse_calling_class(backtrace);
        assert_eq!(class, Some("AuthService".to_string()));
    }
}
