//! ScribeManualFormat - 检测在 Scribe 中手动使用 format!

use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_hir::{Expr, ExprKind};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **警告：在 Scribe 中手动使用 format!**
    ///
    /// Scribe 宏自动处理格式化，无需手动调用 format!
    ///
    /// # 不推荐
    ///
    /// ```rust,ignore
    /// scribe_d!("{}", format!("Value: {}", val));  // ⚠️ 多余的 format!
    /// ```
    ///
    /// # 推荐
    ///
    /// ```rust
    /// scribe_d!("Value: {}", val);                 // ✅ 直接格式化
    /// ```
    pub SCRIBE_MANUAL_FORMAT,
    Warn,
    "检测在 Scribe 中手动使用 format!"
}

declare_lint_pass!(ScribeManualFormat => [SCRIBE_MANUAL_FORMAT]);

impl<'tcx> LateLintPass<'tcx> for ScribeManualFormat {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // TODO: 检测 scribe_*!("{}", format!(...)) 模式
    }
}
