use harpoon_diagnostics::Span;
use harpoon_intern::{Arena, Idx, SymbolId};

/// Direction modifier for a feature (in, out, inout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureDirection {
    In,
    Out,
    InOut,
}

/// Visibility modifier for a member or import.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

/// A definition ID — typed index into the def arena.
pub type DefId = Idx<Def>;

/// A membership ID — typed index into the membership arena.
pub type MembershipId = Idx<Membership>;

/// What kind of membership relationship this represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MembershipKind {
    Owning,
    Feature,
    Member,
}

/// A membership relationship between a namespace and a member.
#[derive(Clone, Debug)]
pub struct Membership {
    pub visibility: Visibility,
    pub kind: MembershipKind,
    pub member_def: DefId,
    pub owning_namespace: DefId,
    pub span: Span,
}

/// What kind of definition this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefKind {
    Package,
    Type,
    Feature,
    Conjugation,
}

/// Resolution state for a name reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolutionState {
    Unresolved,
    InProgress,
    Resolved(DefId),
    Error,
}

/// A reference to a name that may or may not be resolved yet.
#[derive(Clone, Debug)]
pub struct NameRef {
    pub segments: Vec<SymbolId>,
    pub span: Span,
    pub resolution: ResolutionState,
}

impl NameRef {
    pub fn unresolved(segments: Vec<SymbolId>, span: Span) -> Self {
        Self {
            segments,
            span,
            resolution: ResolutionState::Unresolved,
        }
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self.resolution, ResolutionState::Resolved(_))
    }

    pub fn resolved_def(&self) -> Option<DefId> {
        match self.resolution {
            ResolutionState::Resolved(id) => Some(id),
            _ => None,
        }
    }
}

/// A multiplicity bound: concrete value, unbounded (*), or symbolic feature reference.
#[derive(Clone, Debug)]
pub enum MultBound {
    Exact(u64),
    Unbounded,
    Ref(NameRef),
}

impl MultBound {
    pub fn as_name_ref_mut(&mut self) -> Option<&mut NameRef> {
        if let MultBound::Ref(r) = self {
            Some(r)
        } else {
            None
        }
    }
}

/// Multiplicity bounds in the HIR.
#[derive(Clone, Debug)]
pub struct HirMultiplicity {
    pub lower: MultBound,
    pub upper: MultBound,
    pub span: Span,
}

/// A definition in the semantic model.
#[derive(Clone, Debug)]
pub struct Def {
    pub name: SymbolId,
    pub kind: DefKind,
    pub span: Span,
    pub parent: Option<DefId>,
    pub owned_memberships: Vec<MembershipId>,
    /// For types: specialization targets (resolved or not)
    pub specializations: Vec<NameRef>,
    /// For types: conjugation target
    pub conjugation: Option<NameRef>,
    /// For features: typing reference
    pub type_ref: Option<NameRef>,
    /// For features: chain segments
    pub chain_segments: Vec<NameRef>,
    /// For features with chains: the final resolved def of the chain
    pub chain_result: Option<DefId>,
    /// For features: multiplicity
    pub multiplicity: Option<HirMultiplicity>,
    /// For features: direction modifier (in, out, inout)
    pub direction: Option<FeatureDirection>,
    /// For conjugation declarations: (conjugated_type, original_type)
    pub conjugation_decl: Option<(NameRef, NameRef)>,
    /// Imports visible from this def's scope
    pub imports: Vec<Import>,
    /// Inherited memberships (populated by type checking)
    pub inherited_memberships: Vec<MembershipId>,
    /// Whether this def has been fully type-checked
    pub type_checked: bool,
}

impl Def {
    pub fn new(name: SymbolId, kind: DefKind, span: Span) -> Self {
        Self {
            name,
            kind,
            span,
            parent: None,
            owned_memberships: Vec::new(),
            specializations: Vec::new(),
            conjugation: None,
            conjugation_decl: None,
            type_ref: None,
            chain_segments: Vec::new(),
            chain_result: None,
            multiplicity: None,
            direction: None,
            imports: Vec::new(),
            inherited_memberships: Vec::new(),
            type_checked: false,
        }
    }
}

/// An import declaration in the HIR.
#[derive(Clone, Debug)]
pub struct Import {
    pub path: NameRef,
    pub is_wildcard: bool,
    pub visibility: Visibility,
    pub span: Span,
}

/// The top-level semantic model.
pub struct SemanticModel {
    pub defs: Arena<Def>,
    pub memberships: Arena<Membership>,
    /// Root defs (packages and top-level members)
    pub roots: Vec<DefId>,
}

impl SemanticModel {
    pub fn new() -> Self {
        Self {
            defs: Arena::new(),
            memberships: Arena::new(),
            roots: Vec::new(),
        }
    }

    pub fn alloc_def(&mut self, def: Def) -> DefId {
        self.defs.alloc(def)
    }

    /// Create a membership linking parent to child.
    pub fn add_member(
        &mut self,
        parent: DefId,
        child: DefId,
        visibility: Visibility,
        kind: MembershipKind,
        span: Span,
    ) -> MembershipId {
        let mid = self.memberships.alloc(Membership {
            visibility,
            kind,
            member_def: child,
            owning_namespace: parent,
            span,
        });
        self.defs[parent].owned_memberships.push(mid);
        self.defs[child].parent = Some(parent);
        mid
    }

    /// Iterate over direct children of a def.
    pub fn children(&self, def: DefId) -> impl Iterator<Item = DefId> + '_ {
        self.defs[def]
            .owned_memberships
            .iter()
            .map(|&mid| self.memberships[mid].member_def)
    }

    /// Find a direct child of `parent` with the given name.
    pub fn find_child(&self, parent: DefId, name: SymbolId) -> Option<DefId> {
        self.children(parent)
            .find(|&child| self.defs[child].name == name)
    }

    /// Find a root def with the given name.
    pub fn find_root(&self, name: SymbolId) -> Option<DefId> {
        self.roots
            .iter()
            .find(|&&id| self.defs[id].name == name)
            .copied()
    }

    /// Compute the direction of a feature within a type,
    /// following KerML `directionOfExcluding()` semantics.
    pub fn direction_of(&self, feature: DefId, in_type: DefId) -> Option<FeatureDirection> {
        self.direction_of_excluding(feature, in_type, &mut Vec::new())
    }

    fn direction_of_excluding(
        &self,
        feature: DefId,
        in_type: DefId,
        excluded: &mut Vec<DefId>,
    ) -> Option<FeatureDirection> {
        if self.defs[feature].parent == Some(in_type) {
            let dir = self.defs[feature].direction;
            if self.defs[in_type]
                .conjugation
                .as_ref()
                .and_then(|c| c.resolved_def())
                .is_some()
            {
                return dir.map(Self::conjugate_dir);
            }
            return dir;
        }

        excluded.push(in_type);

        for spec in &self.defs[in_type].specializations {
            if let Some(super_id) = spec.resolved_def() {
                if excluded.contains(&super_id) {
                    continue;
                }
                if let Some(dir) = self.direction_of_excluding(feature, super_id, excluded) {
                    if self.defs[in_type]
                        .conjugation
                        .as_ref()
                        .and_then(|c| c.resolved_def())
                        .is_some()
                    {
                        return Some(Self::conjugate_dir(dir));
                    }
                    return Some(dir);
                }
            }
        }

        if let Some(conj) = &self.defs[in_type].conjugation {
            if let Some(conj_id) = conj.resolved_def() {
                if !excluded.contains(&conj_id) {
                    if let Some(dir) = self.direction_of_excluding(feature, conj_id, excluded) {
                        return Some(Self::conjugate_dir(dir));
                    }
                }
            }
        }

        None
    }

    fn conjugate_dir(dir: FeatureDirection) -> FeatureDirection {
        match dir {
            FeatureDirection::In => FeatureDirection::Out,
            FeatureDirection::Out => FeatureDirection::In,
            FeatureDirection::InOut => FeatureDirection::InOut,
        }
    }
}

impl Default for SemanticModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use harpoon_intern::StringInterner;

    #[test]
    fn alloc_membership() {
        let mut model = SemanticModel::new();
        let mut interner = StringInterner::new();
        let parent = model.alloc_def(Def::new(
            interner.intern("P"),
            DefKind::Package,
            Span::dummy(),
        ));
        let child = model.alloc_def(Def::new(interner.intern("T"), DefKind::Type, Span::dummy()));
        let mid = model.add_member(
            parent,
            child,
            Visibility::Public,
            MembershipKind::Owning,
            Span::dummy(),
        );
        assert_eq!(model.memberships[mid].member_def, child);
        assert_eq!(model.memberships[mid].visibility, Visibility::Public,);
        assert_eq!(model.defs[child].parent, Some(parent));
        let children: Vec<DefId> = model.children(parent).collect();
        assert_eq!(children, vec![child]);
    }

    #[test]
    fn find_child_by_name() {
        let mut model = SemanticModel::new();
        let mut interner = StringInterner::new();
        let parent = model.alloc_def(Def::new(
            interner.intern("P"),
            DefKind::Package,
            Span::dummy(),
        ));
        let child = model.alloc_def(Def::new(interner.intern("T"), DefKind::Type, Span::dummy()));
        let name = interner.intern("T");
        model.add_member(
            parent,
            child,
            Visibility::Public,
            MembershipKind::Owning,
            Span::dummy(),
        );
        assert_eq!(model.find_child(parent, name), Some(child));
    }

    #[test]
    fn direction_of_direct_feature() {
        let mut model = SemanticModel::new();
        let mut interner = StringInterner::new();
        let ty = model.alloc_def(Def::new(interner.intern("T"), DefKind::Type, Span::dummy()));
        let mut feat_def = Def::new(interner.intern("f"), DefKind::Feature, Span::dummy());
        feat_def.direction = Some(FeatureDirection::In);
        let feat = model.alloc_def(feat_def);
        model.add_member(
            ty,
            feat,
            Visibility::Public,
            MembershipKind::Feature,
            Span::dummy(),
        );
        assert_eq!(model.direction_of(feat, ty), Some(FeatureDirection::In),);
    }

    #[test]
    fn mult_bound_ref_is_not_copy() {
        let nr = NameRef::unresolved(vec![], Span::new(harpoon_diagnostics::FileId(0), 0, 0));
        let bound = MultBound::Ref(nr);
        let _cloned = bound.clone();
    }
}
