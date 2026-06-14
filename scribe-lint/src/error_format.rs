//! ScribeErrorFormat - 检测错误日志格式

use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_hir::{Expr, ExprKind};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **警告：错误日志格式不规范**
    ///
    /// 记录错误时应避免冗余信息
    ///
    /// # 不推荐
    ///
    /// ```rust,ignore
    /// scribe_e!("Error: {}", error.to_string());   // ⚠️ 冗余的 to_string()
    /// ```
    ///
    /// # 推荐
    ///
    /// ```rust
    /// scribe_e!("Error: {}", error);               // ✅ 直接传递
    /// ```
    pub SCRIBE_ERROR_FORMAT,
    Warn,
    "检测错误日志格式不规范"
}

declare_lint_pass!(ScribeErrorFormat => [SCRIBE_ERROR_FORMAT]);

impl<'tcx> LateLintPass<'tcx> for ScribeErrorFormat {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // TODO: 检测错误日志格式
    }
}
