use kermlc_diagnostics::DiagnosticSink;
use kermlc_intern::StringInterner;
use kermlc_resolve::{detect_specialization_cycles, emit_unresolved_errors, resolve_pass};
use kermlc_typeck::typecheck_pass;
use kermlc_validate::validate;

// Re-export engine types that drivers and tests need.
pub use kermlc_hir::{
    DefId, DefKind, FeatureDirection, MembershipId, MultBound, NameRef, ResolutionState,
    SemanticModel, StdlibDefs, Visibility,
};
pub use kermlc_serial_json::serialize_to_json;

/// Maximum fixpoint iterations for the resolve/typecheck loop.
const MAX_ITERATIONS: usize = 100;

/// Run the kernel compilation engine over an already-lowered `SemanticModel`.
///
/// Performs, in order:
/// 1. Interleaved resolve/typecheck fixpoint loop (up to `MAX_ITERATIONS`).
/// 2. `emit_unresolved_errors` — emit diagnostics for anything still unresolved.
/// 3. `detect_specialization_cycles` — report circular specialization chains.
/// 4. `validate` — semantic validation rules (e.g., multiplicity bounds).
///
/// Args:
///     model: Mutable semantic model populated by the front-end lowering pass.
///     interner: Shared string interner used during compilation.
///     sink: Diagnostic collector; check `sink.has_errors()` after returning.
pub fn compile(
    model: &mut SemanticModel,
    interner: &StringInterner,
    sink: &mut DiagnosticSink,
) {
    for _ in 0..MAX_ITERATIONS {
        let names_changed = resolve_pass(model, interner, sink);
        let types_changed = typecheck_pass(model, interner, sink);
        if !names_changed && !types_changed {
            break;
        }
    }
    emit_unresolved_errors(model, interner, sink);
    detect_specialization_cycles(model, interner, sink);
    validate(model, interner, sink);
}
