//! ScribeArgCount - 检测格式化参数数量不匹配

use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_hir::{Expr, ExprKind};
use rustc_session::{declare_lint, declare_lint_pass};
use rustc_span::Span;

declare_lint! {
    /// **错误：格式化参数数量不匹配**
    ///
    /// 检测 scribe_d!、scribe_i! 等宏调用时，格式化字符串中的占位符数量
    /// 与提供的参数数量是否匹配。
    ///
    /// # 错误示例
    ///
    /// ```rust,ignore
    /// scribe_d!("Value: {} {}", value);  // ❌ 缺少一个参数
    /// scribe_i!("Count: {}", a, b);      // ❌ 多余一个参数
    /// ```
    ///
    /// # 正确示例
    ///
    /// ```rust
    /// scribe_d!("Value: {} {}", value1, value2);  // ✅
    /// scribe_i!("Count: {}", value);              // ✅
    /// ```
    pub SCRIBE_ARG_COUNT,
    Deny,
    "检测格式化参数数量不匹配"
}

declare_lint_pass!(ScribeArgCount => [SCRIBE_ARG_COUNT]);

impl<'tcx> LateLintPass<'tcx> for ScribeArgCount {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Call(func, args) = &expr.kind {
            // 检查是否是 scribe_* 宏调用
            if is_scribe_macro_call(cx, func) {
                check_format_args(cx, expr.span, args);
            }
        }
    }
}

fn is_scribe_macro_call(cx: &LateContext<'_>, func: &Expr<'_>) -> bool {
    // 检查函数调用是否是 scribe_v/d/i/w/e 宏
    if let ExprKind::Path(qpath) = &func.kind {
        if let Some(def_id) = cx.qpath_res(qpath, func.hir_id).opt_def_id() {
            let name = cx.tcx.item_name(def_id).as_str();
            return name.starts_with("scribe_")
                && (name.ends_with("_v")
                    || name.ends_with("_d")
                    || name.ends_with("_i")
                    || name.ends_with("_w")
                    || name.ends_with("_e"));
        }
    }
    false
}

fn check_format_args(cx: &LateContext<'_>, span: Span, args: &[Expr<'_>]) {
    if args.is_empty() {
        return;
    }

    // 第一个参数是格式化字符串
    if let ExprKind::Lit(lit) = &args[0].kind {
        if let rustc_ast::LitKind::Str(symbol, _) = lit.node {
            let format_str = symbol.as_str();
            let placeholder_count = count_placeholders(&format_str);
            let arg_count = args.len() - 1; // 减去格式化字符串本身

            if placeholder_count != arg_count {
                cx.struct_span_lint(
                    SCRIBE_ARG_COUNT,
                    span,
                    |lint| {
                        lint.build(&format!(
                            "格式化参数数量不匹配: 需要 {} 个参数，但提供了 {} 个",
                            placeholder_count,
                            arg_count
                        ))
                        .emit();
                    },
                );
            }
        }
    }
}

fn count_placeholders(format_str: &str) -> usize {
    let mut count = 0;
    let mut chars = format_str.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            if let Some(&next) = chars.peek() {
                if next != '{' {
                    count += 1;
                } else {
                    chars.next(); // 跳过转义的 {{
                }
            }
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_placeholders() {
        assert_eq!(count_placeholders("Hello {}"), 1);
        assert_eq!(count_placeholders("Value: {} {}"), 2);
        assert_eq!(count_placeholders("{{escaped}}"), 0);
        assert_eq!(count_placeholders("Mixed {{ }} and {}"), 1);
    }
}
