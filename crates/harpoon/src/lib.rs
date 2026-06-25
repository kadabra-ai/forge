use harpoon_diagnostics::DiagnosticSink;
use harpoon_intern::StringInterner;
use harpoon_resolve::{finalize_resolution, resolve_pass};
use harpoon_typeck::typecheck_pass;
use harpoon_validate::validate;

// Re-export engine types that drivers and tests need.
pub use harpoon_hir::{
    DefId, DefKind, FeatureDirection, MembershipId, MultBound, NameRef, ResolutionState,
    SemanticModel, StdlibDefs, Visibility,
};
pub use harpoon_serial_json::serialize_to_json;

/// Maximum fixpoint iterations for the resolve/typecheck loop.
const MAX_ITERATIONS: usize = 100;

/// Run the kernel compilation engine over an already-lowered `SemanticModel`.
///
/// Performs, in order:
/// 1. Interleaved resolve/typecheck fixpoint loop (up to `MAX_ITERATIONS`).
/// 2. `finalize_resolution` — emit unresolved-name diagnostics, then report
///    circular specialization chains.
/// 3. `validate` — semantic validation rules (e.g., multiplicity bounds).
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
    finalize_resolution(model, interner, sink);
    validate(model, interner, sink);
}
