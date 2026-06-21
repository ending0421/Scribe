//! Scribe Lint - 自定义 Clippy Lint 规则
//!
//! 提供编译时静态分析，确保 Scribe 日志 API 的正确使用

#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod arg_count;
mod arg_types;
mod tag_length;
mod log_usage;
mod manual_format;
mod string_concat;
mod error_format;

use rustc_lint::{LintStore, LateLintPass};
use rustc_session::Session;

/// 注册所有 Scribe Lint 规则
pub fn register_lints(store: &mut LintStore, _sess: &Session) {
    store.register_lints(&[
        // 错误级别
        arg_count::SCRIBE_ARG_COUNT,
        arg_types::SCRIBE_ARG_TYPES,
        tag_length::SCRIBE_TAG_LENGTH,

        // 警告级别
        log_usage::SCRIBE_LOG_USAGE,
        manual_format::SCRIBE_MANUAL_FORMAT,
        string_concat::SCRIBE_STRING_CONCAT,
        error_format::SCRIBE_ERROR_FORMAT,
    ]);

    store.register_late_pass(|_| Box::new(arg_count::ScribeArgCount));
    store.register_late_pass(|_| Box::new(arg_types::ScribeArgTypes));
    store.register_late_pass(|_| Box::new(tag_length::ScribeTagLength));
    store.register_late_pass(|_| Box::new(log_usage::ScribeLogUsage));
    store.register_late_pass(|_| Box::new(manual_format::ScribeManualFormat));
    store.register_late_pass(|_| Box::new(string_concat::ScribeStringConcat));
    store.register_late_pass(|_| Box::new(error_format::ScribeErrorFormat));
}

#[no_mangle]
pub fn __rustc_plugin_registrar(reg: &mut rustc_lint::LintStore) {
    register_lints(reg, &rustc_session::Session::default());
}
