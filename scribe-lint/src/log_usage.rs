//! ScribeLogUsage - 检测使用原生日志而非 Scribe

use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_hir::{Expr, ExprKind};
use rustc_session::{declare_lint, declare_lint_pass};

declare_lint! {
    /// **警告：使用原生日志而非 Scribe**
    ///
    /// 应该使用 Scribe 的日志宏而不是 println!、eprintln! 或其他日志库
    ///
    /// # 不推荐
    ///
    /// ```rust,ignore
    /// println!("Debug: {}", value);        // ⚠️
    /// eprintln!("Error: {}", error);       // ⚠️
    /// log::info!("Info: {}", data);        // ⚠️
    /// ```
    ///
    /// # 推荐
    ///
    /// ```rust
    /// scribe_d!("Debug: {}", value);       // ✅
    /// scribe_e!("Error: {}", error);       // ✅
    /// scribe_i!("Info: {}", data);         // ✅
    /// ```
    pub SCRIBE_LOG_USAGE,
    Warn,
    "检测使用原生日志而非 Scribe"
}

declare_lint_pass!(ScribeLogUsage => [SCRIBE_LOG_USAGE]);

impl<'tcx> LateLintPass<'tcx> for ScribeLogUsage {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Call(func, _) = &expr.kind {
            if let ExprKind::Path(qpath) = &func.kind {
                if let Some(def_id) = cx.qpath_res(qpath, func.hir_id).opt_def_id() {
                    let name = cx.tcx.item_name(def_id).as_str();

                    // 检测常见的日志输出
                    if matches!(name.as_ref(), "println" | "eprintln" | "print" | "eprint") {
                        cx.struct_span_lint(
                            SCRIBE_LOG_USAGE,
                            expr.span,
                            |lint| {
                                lint.build(&format!("使用 {} 而非 Scribe 日志", name))
                                    .help("建议使用 scribe_d!、scribe_i! 等宏")
                                    .emit();
                            },
                        );
                    }
                }
            }
        }
    }
}
