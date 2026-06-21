//! ScribeTagLength - 检测 Tag 长度超过限制

use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_hir::{Expr, ExprKind};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **错误：Tag 长度超过 23 字符限制**
    ///
    /// Android 日志系统限制 tag 最大长度为 23 字符
    ///
    /// # 错误示例
    ///
    /// ```rust,ignore
    /// scribe_tag_d!("ThisIsAVeryLongTagName_MoreThan23", "message");  // ❌
    /// ```
    ///
    /// # 正确示例
    ///
    /// ```rust
    /// scribe_tag_d!("ShortTag", "message");  // ✅
    /// ```
    pub SCRIBE_TAG_LENGTH,
    Deny,
    "检测 Tag 长度超过 23 字符限制"
}

declare_lint_pass!(ScribeTagLength => [SCRIBE_TAG_LENGTH]);

const MAX_TAG_LENGTH: usize = 23;

impl<'tcx> LateLintPass<'tcx> for ScribeTagLength {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Call(func, args) = &expr.kind {
            if is_scribe_tag_macro(cx, func) && !args.is_empty() {
                // 第一个参数是 tag
                if let ExprKind::Lit(lit) = &args[0].kind {
                    if let rustc_ast::LitKind::Str(symbol, _) = lit.node {
                        let tag = symbol.as_str();
                        if tag.len() > MAX_TAG_LENGTH {
                            cx.struct_span_lint(
                                SCRIBE_TAG_LENGTH,
                                args[0].span,
                                |lint| {
                                    lint.build(&format!(
                                        "Tag 长度 {} 超过 Android 限制 {}  字符: '{}'",
                                        tag.len(),
                                        MAX_TAG_LENGTH,
                                        tag
                                    ))
                                    .help(&format!("请将 tag 缩短到 {} 字符以内", MAX_TAG_LENGTH))
                                    .emit();
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}

fn is_scribe_tag_macro(cx: &LateContext<'_>, func: &Expr<'_>) -> bool {
    if let ExprKind::Path(qpath) = &func.kind {
        if let Some(def_id) = cx.qpath_res(qpath, func.hir_id).opt_def_id() {
            let name = cx.tcx.item_name(def_id).as_str();
            return name.starts_with("scribe_tag_");
        }
    }
    false
}
