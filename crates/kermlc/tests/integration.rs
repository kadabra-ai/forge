use kermlc_diagnostics::{DiagnosticSink, SourceMap};
use kermlc_hir::{add_implicit_specializations, load_stdlib, lower_ast, SemanticModel};
use kermlc_intern::StringInterner;
use kermlc_resolve::{detect_specialization_cycles, emit_unresolved_errors, resolve_pass};
use kermlc_serial_json::serialize_to_json;
use kermlc_typeck::typecheck_pass;
use kermlc_validate::validate;

/// Compile a KerML source string through the full pipeline.
fn compile_source(source: &str) -> CompileResult {
    let mut interner = StringInterner::new();
    let mut source_map = SourceMap::new();
    let mut sink = DiagnosticSink::new();
    let file_id = source_map.add_file("test.kerml".into(), source.into());

    let parse = kermlc_parser::Parser::parse(source, file_id, &mut interner, &mut sink);
    let mut model = lower_ast(&parse, &interner, &mut sink);
    let stdlib = load_stdlib(&mut model, &mut interner);
    add_implicit_specializations(&mut model, &stdlib);

    // Fixpoint loop
    for _ in 0..100 {
        let r = resolve_pass(&mut model, &interner, &mut sink);
        let t = typecheck_pass(&mut model, &interner, &mut sink);
        if !r && !t {
            break;
        }
    }
    emit_unresolved_errors(&model, &interner, &mut sink);
    detect_specialization_cycles(&model, &interner, &mut sink);
    validate(&model, &interner, &mut sink);

    CompileResult {
        model,
        interner,
        sink,
    }
}

/// Get the fixtures directory based on CARGO_MANIFEST_DIR.
fn fixtures_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(manifest_dir).join("tests").join("fixtures")
}

/// Compile a KerML file from disk through the full pipeline.
fn compile_file(path: &std::path::Path) -> CompileResult {
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e));
    compile_source(&source)
}

struct CompileResult {
    model: SemanticModel,
    interner: StringInterner,
    sink: DiagnosticSink,
}

// ── Valid fixture tests ──────────────────────────────────────────────

#[test]
fn valid_simple_package() {
    let result = compile_file(&fixtures_dir().join("valid/simple_package.kerml"));
    assert!(
        !result.sink.has_errors(),
        "Errors in simple_package.kerml: {:?}",
        result.sink.diagnostics()
    );
}

#[test]
fn valid_specialization() {
    let result = compile_file(&fixtures_dir().join("valid/specialization.kerml"));
    assert!(
        !result.sink.has_errors(),
        "Errors in specialization.kerml: {:?}",
        result.sink.diagnostics()
    );
}

#[test]
fn valid_conjugation() {
    let result = compile_file(&fixtures_dir().join("valid/conjugation.kerml"));
    assert!(
        !result.sink.has_errors(),
        "Errors in conjugation.kerml: {:?}",
        result.sink.diagnostics()
    );
}

#[test]
fn valid_feature_chain() {
    let result = compile_file(&fixtures_dir().join("valid/feature_chain.kerml"));
    assert!(
        !result.sink.has_errors(),
        "Errors in feature_chain.kerml: {:?}",
        result.sink.diagnostics()
    );
}

#[test]
fn valid_imports() {
    let result = compile_file(&fixtures_dir().join("valid/imports.kerml"));
    assert!(
        !result.sink.has_errors(),
        "Errors in imports.kerml: {:?}",
        result.sink.diagnostics()
    );
}

// ── Invalid fixture tests ────────────────────────────────────────────

#[test]
fn invalid_unresolved_type() {
    let result = compile_file(&fixtures_dir().join("invalid/unresolved_type.kerml"));
    assert!(
        result.sink.has_errors(),
        "Expected errors in unresolved_type.kerml"
    );
}

#[test]
fn invalid_missing_brace() {
    let result = compile_file(&fixtures_dir().join("invalid/missing_brace.kerml"));
    assert!(
        result.sink.has_errors(),
        "Expected errors in missing_brace.kerml"
    );
}

// ── Pipeline behavior tests ─────────────────────────────────────────

#[test]
fn json_serialization_produces_valid_json() {
    let result = compile_source("package Foo { type Bar {} }");
    assert!(!result.sink.has_errors());

    let json = serialize_to_json(&result.model, &result.interner);
    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&json);
    assert!(parsed.is_ok(), "Invalid JSON output: {}", json);
    let elements = parsed.unwrap();
    assert!(!elements.is_empty());
}

#[test]
fn cross_package_qualified_resolution() {
    let result = compile_source(
        r#"
        package A { type X {} }
        package B { type Y :> A::X {} }
        "#,
    );
    assert!(
        !result.sink.has_errors(),
        "Cross-package qualified name should resolve: {:?}",
        result.sink.diagnostics()
    );
}

#[test]
fn specialization_chain_inherits_features() {
    let result = compile_source(
        r#"
        package P {
            type A { feature x : A; }
            type B :> A {}
            type C :> B {}
        }
        "#,
    );
    assert!(!result.sink.has_errors());

    // Verify C inherits from the chain
    let pkg = result.model.roots[0];
    let c_id = result.model.defs[pkg].children[2];
    assert!(
        !result.model.defs[c_id].inherited_features.is_empty(),
        "C should inherit features through specialization chain A -> B -> C"
    );
}

#[test]
fn validation_catches_bad_multiplicity() {
    let result = compile_source(
        "package P { type T { feature x : T [5..2]; } }",
    );
    assert!(
        result.sink.has_errors(),
        "Should catch lower > upper multiplicity"
    );
}
