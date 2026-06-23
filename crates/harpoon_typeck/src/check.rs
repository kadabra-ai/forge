use harpoon_diagnostics::DiagnosticSink;
use harpoon_hir::{DefId, DefKind, MembershipId, MembershipKind, SemanticModel, Visibility};
use harpoon_intern::StringInterner;

/// Run one pass of type checking over the model.
/// Processes resolved specializations to compute inherited features.
/// Returns `true` if any type information changed (for the fixpoint loop).
pub fn typecheck_pass(
    model: &mut SemanticModel,
    _interner: &StringInterner,
    _sink: &mut DiagnosticSink,
) -> bool {
    let mut changed = false;

    let all_defs: Vec<DefId> = model.defs.iter().map(|(id, _)| id).collect();

    for def_id in all_defs {
        if model.defs[def_id].type_checked {
            continue;
        }

        match model.defs[def_id].kind {
            DefKind::Type => {
                changed |= check_type(model, def_id);
            }
            DefKind::Feature => {
                changed |= check_feature(model, def_id);
            }
            DefKind::Conjugation => {
                changed |= check_conjugation_decl(model, def_id);
            }
            DefKind::Package => {}
        }
    }

    changed
}

/// Check a type definition: verify specializations are resolved
/// and compute inherited memberships.
fn check_type(model: &mut SemanticModel, def_id: DefId) -> bool {
    let mut changed = false;

    let all_specs_resolved = model.defs[def_id]
        .specializations
        .iter()
        .all(|s| s.is_resolved());
    if !all_specs_resolved {
        return false;
    }

    if let Some(conj) = &model.defs[def_id].conjugation {
        if !conj.is_resolved() {
            return false;
        }
    }

    // Collect inherited memberships from supertypes
    let supertype_ids: Vec<DefId> = model.defs[def_id]
        .specializations
        .iter()
        .filter_map(|s| s.resolved_def())
        .collect();

    let mut inherited: Vec<MembershipId> = Vec::new();

    for &super_id in &supertype_ids {
        for &mid in &model.defs[super_id].owned_memberships {
            let m = &model.memberships[mid];
            if m.visibility != Visibility::Private
                && (m.kind == MembershipKind::Feature || m.kind == MembershipKind::Member)
            {
                inherited.push(mid);
            }
        }
        let super_inherited = model.defs[super_id].inherited_memberships.clone();
        inherited.extend(super_inherited);
    }

    // Collect from conjugation target
    if let Some(conj) = &model.defs[def_id].conjugation {
        if let Some(conj_id) = conj.resolved_def() {
            for &mid in &model.defs[conj_id].owned_memberships {
                let m = &model.memberships[mid];
                if m.visibility != Visibility::Private
                    && (m.kind == MembershipKind::Feature || m.kind == MembershipKind::Member)
                {
                    inherited.push(mid);
                }
            }
            let conj_inherited = model.defs[conj_id].inherited_memberships.clone();
            inherited.extend(conj_inherited);
        }
    }

    // Dedup by MembershipId
    inherited.sort_by_key(|mid| mid.raw());
    inherited.dedup();

    if model.defs[def_id].inherited_memberships != inherited {
        model.defs[def_id].inherited_memberships = inherited;
        changed = true;
    }

    model.defs[def_id].type_checked = true;
    changed
}

/// Check a named conjugation declaration: apply conjugation
/// effect to the conjugated type.
fn check_conjugation_decl(model: &mut SemanticModel, def_id: DefId) -> bool {
    let (conj_ref, orig_ref) = match &model.defs[def_id].conjugation_decl {
        Some((c, o)) => (c.clone(), o.clone()),
        None => return false,
    };

    if !conj_ref.is_resolved() || !orig_ref.is_resolved() {
        return false;
    }

    let conjugated_id = conj_ref.resolved_def().unwrap();
    let original_id = orig_ref.resolved_def().unwrap();

    // Only apply if the conjugated type doesn't already have
    // a conjugation set (from inline ~ or a prior declaration)
    if model.defs[conjugated_id].conjugation.is_some() {
        model.defs[def_id].type_checked = true;
        return false;
    }

    // Set the conjugation on the conjugated type so check_type
    // picks it up and applies direction flipping
    model.defs[conjugated_id].conjugation = Some(harpoon_hir::NameRef {
        segments: orig_ref.segments,
        span: orig_ref.span,
        resolution: harpoon_hir::ResolutionState::Resolved(original_id),
    });

    // Reset type_checked on the conjugated type so it gets
    // re-processed with the new conjugation
    model.defs[conjugated_id].type_checked = false;

    model.defs[def_id].type_checked = true;
    true
}

/// Check a feature definition: verify type ref is resolved.
fn check_feature(model: &mut SemanticModel, def_id: DefId) -> bool {
    // Check type ref
    if let Some(type_ref) = &model.defs[def_id].type_ref {
        if !type_ref.is_resolved() {
            return false; // Can't type-check yet
        }
    }

    // Check chain segments
    let all_chains_resolved = model.defs[def_id]
        .chain_segments
        .iter()
        .all(|s| s.is_resolved());

    if !all_chains_resolved {
        return false;
    }

    model.defs[def_id].type_checked = true;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use harpoon_diagnostics::{DiagnosticSink, SourceMap};
    use harpoon_hir::FeatureDirection;
    use kermlc_lower::lower_ast;
    use harpoon_intern::StringInterner;
    use kermlc_parser::Parser;
    use harpoon_resolve::resolve_pass;

    fn compile_to_model(input: &str) -> (SemanticModel, StringInterner, DiagnosticSink) {
        let mut interner = StringInterner::new();
        let mut source_map = SourceMap::new();
        let mut sink = DiagnosticSink::new();
        let file_id = source_map.add_file("test.kerml".into(), input.into());
        let parse = Parser::parse(input, file_id, &mut interner, &mut sink);
        let mut model = lower_ast(&parse, &mut interner, &mut sink);

        // Run fixpoint
        for _ in 0..10 {
            let r = resolve_pass(&mut model, &interner, &mut sink);
            let t = typecheck_pass(&mut model, &interner, &mut sink);
            if !r && !t {
                break;
            }
        }

        (model, interner, sink)
    }

    #[test]
    fn specialization_adds_inherited_features() {
        let (model, interner, sink) =
            compile_to_model("package P { type A { feature x : A; } type B :> A {} }");
        assert!(!sink.has_errors());

        let pkg = model.roots[0];
        let b_id = model.children(pkg).nth(1).unwrap();
        assert_eq!(interner.resolve(model.defs[b_id].name), "B");
        assert!(model.defs[b_id].type_checked);
        assert!(
            !model.defs[b_id].inherited_memberships.is_empty(),
            "B should inherit features from A"
        );
    }

    #[test]
    fn conjugation_inherits_features() {
        let (model, interner, sink) =
            compile_to_model("package P { type T { feature f : T; } type U ~ T {} }");
        assert!(!sink.has_errors());

        let pkg = model.roots[0];
        let u_id = model.children(pkg).nth(1).unwrap();
        assert_eq!(interner.resolve(model.defs[u_id].name), "U");
        assert!(model.defs[u_id].type_checked);
        assert!(
            !model.defs[u_id].inherited_memberships.is_empty(),
            "U should inherit features from conjugated T"
        );
    }

    #[test]
    fn conjugation_flips_directions() {
        let (model, interner, _sink) = compile_to_model(
            r#"package P {
            type A {
                in feature f : A;
                out feature g : A;
                inout feature h : A;
                feature x : A;
            }
            type B ~ A {}
        }"#,
        );

        let pkg = model.roots[0];
        let _a_id = model.children(pkg).next().unwrap();
        let b_id = model.children(pkg).nth(1).unwrap();
        assert_eq!(interner.resolve(model.defs[b_id].name), "B");

        let inherited = &model.defs[b_id].inherited_memberships;
        assert_eq!(inherited.len(), 4, "B should have 4 inherited memberships");

        for &mid in inherited {
            let feat_id = model.memberships[mid].member_def;
            let name = interner.resolve(model.defs[feat_id].name);
            let dir = model.direction_of(feat_id, b_id);
            match name {
                "f" => assert_eq!(dir, Some(FeatureDirection::Out)),
                "g" => assert_eq!(dir, Some(FeatureDirection::In)),
                "h" => assert_eq!(dir, Some(FeatureDirection::InOut)),
                "x" => assert_eq!(dir, None),
                other => panic!("unexpected feature: {other}"),
            }
        }
    }

    #[test]
    fn nested_types_not_inherited() {
        let (model, interner, sink) = compile_to_model(
            "package P { type A { type Inner {} feature f : A; } type B :> A {} }",
        );
        assert!(!sink.has_errors(), "{:?}", sink.diagnostics());

        let pkg = model.roots[0];
        let b_id = model.children(pkg).nth(1).unwrap();
        assert_eq!(interner.resolve(model.defs[b_id].name), "B");

        assert_eq!(
            model.defs[b_id].inherited_memberships.len(),
            1,
            "B should only inherit features, not nested types"
        );
        let mid = model.defs[b_id].inherited_memberships[0];
        let name = interner.resolve(model.defs[model.memberships[mid].member_def].name);
        assert_eq!(name, "f");
    }

    #[test]
    fn feature_type_ref_resolved() {
        let (model, interner, sink) =
            compile_to_model("package P { type A {} type B { feature x : A; } }");
        assert!(!sink.has_errors());

        let pkg = model.roots[0];
        let b_id = model.children(pkg).nth(1).unwrap();
        let x_id = model.children(b_id).next().unwrap();
        assert_eq!(interner.resolve(model.defs[x_id].name), "x");
        assert!(model.defs[x_id].type_ref.as_ref().unwrap().is_resolved());
        assert!(model.defs[x_id].type_checked);
    }
}
