use kermlc_diagnostics::DiagnosticSink;
use kermlc_hir::SemanticModel;
use kermlc_intern::StringInterner;
use kermlc_resolve::{detect_specialization_cycles, emit_unresolved_errors, resolve_pass};
use kermlc_typeck::typecheck_pass;

const MAX_ITERATIONS: usize = 100;

/// Runs the interleaved fixpoint loop of name resolution and type checking.
/// Iterates until neither pass makes progress, then emits diagnostics
/// for anything still unresolved.
pub fn resolve_and_typecheck(
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
    // Emit diagnostics for anything still unresolved
    emit_unresolved_errors(model, interner, sink);
    // Detect circular specialization chains
    detect_specialization_cycles(model, interner, sink);
}

#[cfg(test)]
mod tests {
    use super::*;
    use kermlc_diagnostics::SourceMap;
    use kermlc_hir::lower_ast;
    use kermlc_intern::StringInterner;
    use kermlc_parser::Parser;

    fn compile(input: &str) -> (SemanticModel, StringInterner, DiagnosticSink) {
        let mut interner = StringInterner::new();
        let mut source_map = SourceMap::new();
        let mut sink = DiagnosticSink::new();
        let file_id = source_map.add_file("test.kerml".into(), input.into());
        let parse = Parser::parse(input, file_id, &mut interner, &mut sink);
        let mut model = lower_ast(&parse, &mut interner, &mut sink);
        resolve_and_typecheck(&mut model, &interner, &mut sink);
        (model, interner, sink)
    }

    #[test]
    fn fixpoint_resolves_cross_references() {
        let (model, _interner, sink) = compile(
            r#"
            package P {
                type A { feature x : B; }
                type B { feature y : A; }
            }
            "#,
        );
        assert!(!sink.has_errors());

        let pkg = model.roots[0];
        let a_id = model.defs[pkg].children[0];
        let b_id = model.defs[pkg].children[1];

        // A's feature x should have type ref resolved to B
        let x_id = model.defs[a_id].children[0];
        assert!(model.defs[x_id].type_ref.as_ref().unwrap().is_resolved());
        assert_eq!(
            model.defs[x_id].type_ref.as_ref().unwrap().resolved_def(),
            Some(b_id)
        );

        // B's feature y should have type ref resolved to A
        let y_id = model.defs[b_id].children[0];
        assert!(model.defs[y_id].type_ref.as_ref().unwrap().is_resolved());
        assert_eq!(
            model.defs[y_id].type_ref.as_ref().unwrap().resolved_def(),
            Some(a_id)
        );
    }

    #[test]
    fn fixpoint_resolves_specialization_chain() {
        let (model, interner, sink) = compile(
            r#"
            package P {
                type A { feature x : A; }
                type B :> A {}
                type C :> B {}
            }
            "#,
        );
        assert!(!sink.has_errors());

        let pkg = model.roots[0];
        let c_id = model.defs[pkg].children[2];
        assert_eq!(interner.resolve(model.defs[c_id].name), "C");
        assert!(model.defs[c_id].type_checked);

        // C should inherit feature x from A (through B)
        assert!(
            !model.defs[c_id].inherited_features.is_empty(),
            "C should inherit features from B which inherits from A"
        );
    }

    #[test]
    fn fixpoint_with_imports() {
        let (model, interner, sink) = compile(
            r#"
            package Lib { type Base { feature id : Base; } }
            package App {
                import Lib::*;
                type Widget :> Base {}
            }
            "#,
        );
        assert!(!sink.has_errors());

        let app_pkg = model.roots[1];
        let widget_id = model.defs[app_pkg].children[0];
        assert_eq!(interner.resolve(model.defs[widget_id].name), "Widget");
        assert!(model.defs[widget_id].specializations[0].is_resolved());
        assert!(model.defs[widget_id].type_checked);
    }

    #[test]
    fn fixpoint_unresolved_emits_error() {
        let (_model, _interner, sink) = compile("package P { type A :> DoesNotExist {} }");
        assert!(sink.has_errors());
    }
}
