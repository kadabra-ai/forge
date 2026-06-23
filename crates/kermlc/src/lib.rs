use harpoon_diagnostics::{DiagnosticSink, SourceMap};
use harpoon_hir::{add_implicit_specializations, load_stdlib, SemanticModel};
use harpoon_intern::StringInterner;
use kermlc_lower::lower_ast;

/// The output of a full front-end + engine compilation pass.
///
/// Bundles the resolved/type-checked model together with the interner,
/// diagnostic sink, and source map so callers can inspect results and
/// render diagnostics.
pub struct CompiledModel {
    pub model: SemanticModel,
    pub interner: StringInterner,
    pub sink: DiagnosticSink,
    pub source_map: SourceMap,
}

/// Compile a KerML source string through the full pipeline.
///
/// Runs, in order: parse → lower to HIR → load stdlib → add implicit
/// specializations → `harpoon::compile` (resolve/typeck fixpoint,
/// emit_unresolved_errors, detect_specialization_cycles, validate).
///
/// The `file_name` parameter is used only for diagnostic messages.
///
/// Args:
///     source: KerML source text.
///     file_name: Display name for the source file used in diagnostics.
///
/// Returns:
///     A `CompiledModel` bundling model, interner, sink, and source map.
///     Check `compiled.sink.has_errors()` to determine success.
pub fn compile_source(source: &str, file_name: &str) -> CompiledModel {
    let mut interner = StringInterner::new();
    let mut source_map = SourceMap::new();
    let mut sink = DiagnosticSink::new();

    let file_id = source_map.add_file(file_name.to_string(), source.to_string());
    let parse = kermlc_parser::Parser::parse(source, file_id, &mut interner, &mut sink);
    let mut model = lower_ast(&parse, &mut interner, &mut sink);
    let stdlib = load_stdlib(&mut model, &mut interner);
    add_implicit_specializations(&mut model, &stdlib);

    harpoon::compile(&mut model, &interner, &mut sink);

    CompiledModel {
        model,
        interner,
        sink,
        source_map,
    }
}
