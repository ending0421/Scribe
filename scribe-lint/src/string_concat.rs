//! ScribeStringConcat - 检测字符串拼接

use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_hir::{Expr, ExprKind};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **警告：在 Scribe 中使用字符串拼接**
    ///
    /// 应该使用格式化而非字符串拼接
    ///
    /// # 不推荐
    ///
    /// ```rust,ignore
    /// scribe_d!("{}", "Value: " + value);          // ⚠️ 字符串拼接
    /// scribe_i!("{}", format!("{}{}", a, b));      // ⚠️ 拼接
    /// ```
    ///
    /// # 推荐
    ///
    /// ```rust
    /// scribe_d!("Value: {}", value);               // ✅ 直接格式化
    /// scribe_i!("{}{}", a, b);                     // ✅ 格式化
    /// ```
    pub SCRIBE_STRING_CONCAT,
    Warn,
    "检测在 Scribe 中使用字符串拼接"
}

declare_lint_pass!(ScribeStringConcat => [SCRIBE_STRING_CONCAT]);

impl<'tcx> LateLintPass<'tcx> for ScribeStringConcat {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // TODO: 检测字符串拼接操作
    }
}
