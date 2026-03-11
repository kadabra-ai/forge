use crate::types::*;
use kermlc_diagnostics::Span;
use kermlc_intern::StringInterner;

/// Load the minimal Kernel Semantic Library base types into the model.
///
/// Creates hardcoded Def entries for:
/// - Anything (root of all types)
/// - Object (structures)
/// - DataValue (data types)
/// - Occurrence (occurrences)
/// - Performance (behaviors)
/// - Link (associations)
///
/// All types implicitly specialize Anything (except Anything itself).
pub fn load_stdlib(model: &mut SemanticModel, interner: &mut StringInterner) -> StdlibDefs {
    let anything_id = model.alloc_def(Def::new(
        interner.intern("Anything"),
        DefKind::Type,
        Span::dummy(),
    ));

    let object_id = {
        let mut def = Def::new(interner.intern("Object"), DefKind::Type, Span::dummy());
        def.specializations.push(NameRef {
            segments: vec![interner.intern("Anything")],
            span: Span::dummy(),
            resolution: ResolutionState::Resolved(anything_id),
        });
        model.alloc_def(def)
    };

    let data_value_id = {
        let mut def = Def::new(interner.intern("DataValue"), DefKind::Type, Span::dummy());
        def.specializations.push(NameRef {
            segments: vec![interner.intern("Anything")],
            span: Span::dummy(),
            resolution: ResolutionState::Resolved(anything_id),
        });
        model.alloc_def(def)
    };

    let occurrence_id = {
        let mut def = Def::new(interner.intern("Occurrence"), DefKind::Type, Span::dummy());
        def.specializations.push(NameRef {
            segments: vec![interner.intern("Anything")],
            span: Span::dummy(),
            resolution: ResolutionState::Resolved(anything_id),
        });
        model.alloc_def(def)
    };

    let performance_id = {
        let mut def = Def::new(interner.intern("Performance"), DefKind::Type, Span::dummy());
        def.specializations.push(NameRef {
            segments: vec![interner.intern("Anything")],
            span: Span::dummy(),
            resolution: ResolutionState::Resolved(anything_id),
        });
        model.alloc_def(def)
    };

    let link_id = {
        let mut def = Def::new(interner.intern("Link"), DefKind::Type, Span::dummy());
        def.specializations.push(NameRef {
            segments: vec![interner.intern("Anything")],
            span: Span::dummy(),
            resolution: ResolutionState::Resolved(anything_id),
        });
        model.alloc_def(def)
    };

    // Add stdlib types as roots
    model.roots.push(anything_id);
    model.roots.push(object_id);
    model.roots.push(data_value_id);
    model.roots.push(occurrence_id);
    model.roots.push(performance_id);
    model.roots.push(link_id);

    // Mark all stdlib types as type-checked (they are pre-resolved)
    model.defs[anything_id].type_checked = true;
    model.defs[object_id].type_checked = true;
    model.defs[data_value_id].type_checked = true;
    model.defs[occurrence_id].type_checked = true;
    model.defs[performance_id].type_checked = true;
    model.defs[link_id].type_checked = true;

    StdlibDefs {
        anything: anything_id,
        object: object_id,
        data_value: data_value_id,
        occurrence: occurrence_id,
        performance: performance_id,
        link: link_id,
    }
}

/// Add implicit `specializes Anything` to every user-defined Type
/// that has no explicit specializations.
///
/// Must be called after `load_stdlib` and `lower_ast`.
/// Skips stdlib types, packages, and features.
pub fn add_implicit_specializations(model: &mut SemanticModel, stdlib: &StdlibDefs) {
    let stdlib_ids: Vec<DefId> = vec![
        stdlib.anything,
        stdlib.object,
        stdlib.data_value,
        stdlib.occurrence,
        stdlib.performance,
        stdlib.link,
    ];

    let all_defs: Vec<DefId> = model.defs.iter().map(|(id, _)| id).collect();

    for def_id in all_defs {
        if stdlib_ids.contains(&def_id) {
            continue;
        }
        let def = &model.defs[def_id];
        if def.kind != DefKind::Type {
            continue;
        }
        if !def.specializations.is_empty() {
            continue;
        }
        model.defs[def_id].specializations.push(NameRef {
            segments: vec![],
            span: Span::dummy(),
            resolution: ResolutionState::Resolved(stdlib.anything),
        });
    }
}

/// Holds DefIds for the standard library types for easy reference.
#[derive(Clone, Debug)]
pub struct StdlibDefs {
    pub anything: DefId,
    pub object: DefId,
    pub data_value: DefId,
    pub occurrence: DefId,
    pub performance: DefId,
    pub link: DefId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kermlc_intern::StringInterner;

    #[test]
    fn stdlib_creates_six_types() {
        let mut model = SemanticModel::new();
        let mut interner = StringInterner::new();
        let stdlib = load_stdlib(&mut model, &mut interner);

        assert_eq!(model.roots.len(), 6);
        assert_eq!(model.defs[stdlib.anything].kind, DefKind::Type);
        assert_eq!(
            interner.resolve(model.defs[stdlib.anything].name),
            "Anything"
        );

        // Object specializes Anything
        let object = &model.defs[stdlib.object];
        assert_eq!(object.specializations.len(), 1);
        assert_eq!(
            object.specializations[0].resolution,
            ResolutionState::Resolved(stdlib.anything)
        );
    }

    #[test]
    fn implicit_specialization_added_for_type_without_explicit() {
        let mut model = SemanticModel::new();
        let mut interner = StringInterner::new();
        let stdlib = load_stdlib(&mut model, &mut interner);

        // Create a user-defined type with no explicit specialization
        let user_type = model.alloc_def(Def::new(
            interner.intern("Vehicle"),
            DefKind::Type,
            Span::dummy(),
        ));
        model.roots.push(user_type);

        add_implicit_specializations(&mut model, &stdlib);

        // Vehicle should now implicitly specialize Anything
        let vehicle = &model.defs[user_type];
        assert_eq!(vehicle.specializations.len(), 1);
        assert_eq!(
            vehicle.specializations[0].resolution,
            ResolutionState::Resolved(stdlib.anything)
        );
    }

    #[test]
    fn implicit_specialization_not_added_when_explicit_exists() {
        let mut model = SemanticModel::new();
        let mut interner = StringInterner::new();
        let stdlib = load_stdlib(&mut model, &mut interner);

        // Create a type that already has an explicit specialization
        let base = model.alloc_def(Def::new(
            interner.intern("Base"),
            DefKind::Type,
            Span::dummy(),
        ));
        model.roots.push(base);

        let mut child_def = Def::new(interner.intern("Child"), DefKind::Type, Span::dummy());
        child_def.specializations.push(NameRef {
            segments: vec![interner.intern("Base")],
            span: Span::dummy(),
            resolution: ResolutionState::Resolved(base),
        });
        let child = model.alloc_def(child_def);
        model.roots.push(child);

        add_implicit_specializations(&mut model, &stdlib);

        // Child should still have only its explicit specialization
        assert_eq!(model.defs[child].specializations.len(), 1);
        assert_eq!(
            model.defs[child].specializations[0].resolution,
            ResolutionState::Resolved(base)
        );
    }

    #[test]
    fn implicit_specialization_not_added_to_stdlib_types() {
        let mut model = SemanticModel::new();
        let mut interner = StringInterner::new();
        let stdlib = load_stdlib(&mut model, &mut interner);

        add_implicit_specializations(&mut model, &stdlib);

        // Anything should have no specializations
        assert_eq!(model.defs[stdlib.anything].specializations.len(), 0);
        // Object should still have exactly one (explicit -> Anything)
        assert_eq!(model.defs[stdlib.object].specializations.len(), 1);
    }

    #[test]
    fn implicit_specialization_not_added_to_packages_or_features() {
        let mut model = SemanticModel::new();
        let mut interner = StringInterner::new();
        let stdlib = load_stdlib(&mut model, &mut interner);

        let pkg = model.alloc_def(Def::new(
            interner.intern("MyPkg"),
            DefKind::Package,
            Span::dummy(),
        ));
        model.roots.push(pkg);

        let feat = model.alloc_def(Def::new(
            interner.intern("myFeat"),
            DefKind::Feature,
            Span::dummy(),
        ));
        model.roots.push(feat);

        add_implicit_specializations(&mut model, &stdlib);

        assert_eq!(model.defs[pkg].specializations.len(), 0);
        assert_eq!(model.defs[feat].specializations.len(), 0);
    }
}
