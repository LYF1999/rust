use crate::{LateContext, LateLintPass, LintContext};
use rustc_ast as ast;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_middle::ty;
use rustc_span::sym;

declare_lint! {
    /// The `panic_fmt` lint detects `panic!("..")` with `{` or `}` in the string literal.
    ///
    /// ### Example
    ///
    /// ```rust,no_run
    /// panic!("{}");
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// `panic!("{}")` panics with the message `"{}"`, as a `panic!()` invocation
    /// with a single argument does not use `format_args!()`.
    /// A future version of Rust will interpret this string as format string,
    /// which would break this.
    PANIC_FMT,
    Warn,
    "detect braces in single-argument panic!() invocations",
    report_in_external_macro
}

declare_lint_pass!(PanicFmt => [PANIC_FMT]);

impl<'tcx> LateLintPass<'tcx> for PanicFmt {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::Call(f, [arg]) = &expr.kind {
            if let &ty::FnDef(def_id, _) = cx.typeck_results().expr_ty(f).kind() {
                if Some(def_id) == cx.tcx.lang_items().begin_panic_fn()
                    || Some(def_id) == cx.tcx.lang_items().panic_fn()
                {
                    check_panic(cx, f, arg);
                }
            }
        }
    }
}

fn check_panic<'tcx>(cx: &LateContext<'tcx>, f: &'tcx hir::Expr<'tcx>, arg: &'tcx hir::Expr<'tcx>) {
    if let hir::ExprKind::Lit(lit) = &arg.kind {
        if let ast::LitKind::Str(sym, _) = lit.node {
            if sym.as_str().contains(&['{', '}'][..]) {
                let expn = f.span.ctxt().outer_expn_data();
                if let Some(id) = expn.macro_def_id {
                    if cx.tcx.is_diagnostic_item(sym::std_panic_macro, id)
                        || cx.tcx.is_diagnostic_item(sym::core_panic_macro, id)
                    {
                        cx.struct_span_lint(PANIC_FMT, expn.call_site, |lint| {
                            let mut l = lint.build("Panic message contains a brace");
                            l.note("This message is not used as a format string, but will be in a future Rust version");
                            if expn.call_site.contains(arg.span) {
                                l.span_suggestion(
                                    arg.span.shrink_to_lo(),
                                    "add a \"{}\" format string to use the message literally",
                                    "\"{}\", ".into(),
                                    Applicability::MachineApplicable,
                                );
                            }
                            l.emit();
                        });
                    }
                }
            }
        }
    }
}
