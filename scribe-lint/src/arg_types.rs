//! ScribeArgTypes - 检测格式化参数类型错误

use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_hir::{Expr, ExprKind};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **错误：格式化参数类型错误**
    ///
    /// 检测格式化占位符与参数类型是否匹配
    ///
    /// # 错误示例
    ///
    /// ```rust,ignore
    /// scribe_d!("Value: {:x}", "string");  // ❌ 十六进制格式不支持字符串
    /// scribe_i!("Number: {}", vec![1,2]);  // ❌ Vec 没有实现 Display
    /// ```
    ///
    /// # 正确示例
    ///
    /// ```rust
    /// scribe_d!("Value: {:x}", 255);       // ✅
    /// scribe_i!("Number: {}", 42);         // ✅
    /// ```
    pub SCRIBE_ARG_TYPES,
    Deny,
    "检测格式化参数类型错误"
}

declare_lint_pass!(ScribeArgTypes => [SCRIBE_ARG_TYPES]);

impl<'tcx> LateLintPass<'tcx> for ScribeArgTypes {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // TODO: 实现类型检查逻辑
        // 这需要解析格式化字符串的格式说明符，并检查参数类型
    }
}
