# A4: Diamond Inheritance + Redefinition + Subsetting — Design

**Date:** 2026-03-28
**Status:** Approved
**Depends on:** A5 (Visibility + Membership layer)
**Spec reference:** KerML 1.0 Beta 2, §8.3.3.1 (Types), §8.2.4.3.3 (Subsetting), §8.2.4.3.4 (Redefinition)

## Problem

The compiler lacks:
1. **Diamond inheritance dedup** — when a type inherits the same feature via multiple paths, `removeRedefinedFeatures` must filter duplicates ordering-independently
2. **Redefinition** — `redefines`/`:>>` syntax (inline + standalone declarations)
3. **Subsetting** — `subsets`/`:>`, `references`/`::>`, `crosses` syntax (all feature relationship variants)

## Scope

### In scope (A4)
- Parse all four feature relationship kinds: `subsets`, `redefines`, `references`, `crosses`
- Both inline (`feature f :>> g;`) and standalone (`redefinition r1 redefines X;`) declarations
- Unified `FeatureRel` storage in HIR
- Resolve simple (non-chain) targets
- Implicit redefinition: owned member with same name as inherited member = shadow
- `removeRedefinedFeatures` algorithm in typeck
- Diamond inheritance ordering-independent dedup via MembershipId
- Basic validation (target exists, target is feature)
- JSON-LD serialization (`ownedRedefinition`, `ownedSubsetting`)

### Deferred
- **A4-D1:** Deep type-compatibility validation for redefinition (redefining must subtype redefined)
- **A4-D2:** Multiplicity compatibility validation for subsetting
- **A4-D3:** Cross-subsetting semantics beyond parsing + storage
- **A4-D4:** Reference subsetting semantics beyond parsing + storage
- **B3:** Chain resolution for targets like `b.f.a` (parsed and stored, not resolved)

## Data Model

### AST Layer (`kermlc_ast/src/nodes.rs`)

New enum for relationship targets (simple name vs feature chain):

```rust
pub enum RelTarget {
    Name(QualifiedName),
    Chain(FeatureChain),
}
```

New fields on `FeatureDecl`:

```rust
pub struct FeatureDecl {
    // ... existing fields ...
    pub subsettings: Vec<RelTarget>,
    pub redefinitions: Vec<RelTarget>,
    pub references: Vec<RelTarget>,
    pub crosses: Vec<RelTarget>,
}
```

New standalone declaration nodes:

```rust
pub struct SubsettingDecl {
    pub name: Option<Ident>,
    pub specific: RelTarget,
    pub general: RelTarget,
    pub span: Span,
}

pub struct RedefinitionDecl {
    pub name: Option<Ident>,
    pub specific: RelTarget,
    pub general: RelTarget,
    pub span: Span,
}
```

### HIR Layer (`kermlc_hir/src/types.rs`)

Unified feature relationship:

```rust
pub enum FeatureRelKind {
    Subsetting,
    Redefinition,
    ReferenceSubsetting,
    CrossSubsetting,
}

pub struct FeatureRel {
    pub kind: FeatureRelKind,
    pub target: NameRef,
    pub chain: Vec<NameRef>,  // empty for simple targets
    pub span: Span,
}
```

New field on `Def`:

```rust
pub struct Def {
    // ... existing fields ...
    pub feature_relationships: Vec<FeatureRel>,
}
```

### Design rationale

- **Unified `FeatureRel`** over separate vectors: all four relationship kinds share identical structure (kind + target). Single vector is simpler to iterate in resolve/typeck/validate/serialize.
- **`chain: Vec<NameRef>`** follows existing `chain_segments` pattern. Empty for simple targets, populated for chains like `b.f.a`. Chain resolution deferred to B3.
- **No new `DefKind`** for Subsetting/Redefinition. They are relationships on features, not standalone definitions. Standalone declarations attach to the specific feature during lowering.

## Token Disambiguation: `:>`

The `:>` token has context-dependent meaning per spec:

| Context | `:>` means | BNF rule |
|---------|-----------|----------|
| Type declaration: `type A :> B` | `specializes` | SPECIALIZES |
| Feature declaration: `feature f :> g` | `subsets` | SUBSETS |

This is handled naturally since `parse_type_decl()` and `parse_feature_decl()` are separate functions. Test coverage required for both contexts.

## Parser Changes

### Inline feature relationships

After existing type_ref and specialization parsing in `parse_feature_decl()`:

```
loop {
    match peek():
        'subsets' | ':>'   → parse comma-separated targets → subsettings
        'redefines' | ':>>' → parse comma-separated targets → redefinitions
        'references' | '::>' → parse comma-separated targets → references
        'crosses'          → parse comma-separated targets → crosses
        _                  → break
}
```

Each target parsed as `RelTarget`:
- Try `parse_qualified_name()`
- If followed by `.`, continue as `parse_feature_chain()`

### Standalone declarations

```kerml
subset X subsets Y;
specialization s1 subset X subsets Y;
redefinition SpecificFeature redefines GeneralFeature;
specialization s2 redefinition SpecificFeature redefines GeneralFeature;
```

New `parse_subsetting_decl()` and `parse_redefinition_decl()` functions, dispatched from `parse_non_feature_element()` when seeing `subset`/`redefinition` keywords (or `specialization` followed by these).

## Lowering (AST → HIR)

### Inline

Map each `RelTarget` to `FeatureRel`:

```rust
RelTarget::Name(qn) → FeatureRel {
    kind: ...,
    target: NameRef::unresolved(qn.segments, qn.span),
    chain: vec![],
    span: qn.span,
}

RelTarget::Chain(ch) → FeatureRel {
    kind: ...,
    target: NameRef::unresolved(vec![], ch.span),
    chain: ch.segments.iter()
        .map(|s| NameRef::unresolved(s.segments, s.span))
        .collect(),
    span: ch.span,
}
```

### Standalone

Standalone declarations identify the specific feature by name/chain and attach the relationship to it:
- Simple specific (`subset X subsets Y`): find feature `X` in scope, add `FeatureRel` to it
- Chain specific (`subset g.g subsets b.f.a`): store as unresolved, resolve in resolve pass (B3)

For A4: standalone with simple specific supported. Chain specific parsed and stored but not resolved.

## Resolve Pass

Add resolution for `feature_relationships` targets, same pattern as specializations:

```rust
for rel in &mut def.feature_relationships {
    if rel.chain.is_empty() {
        // Simple target: resolve like specialization NameRef
        resolve_name_ref(&mut rel.target, scope, interner, sink);
    }
    // Chain targets: skip (B3 scope)
}
```

No new resolution strategies needed. Uses existing 5-strategy resolution.

## Typeck: `removeRedefinedFeatures`

Called in `check_type()` after MembershipId dedup, before storing `inherited_memberships`.

```rust
fn remove_redefined_features(
    model: &SemanticModel,
    interner: &StringInterner,
    owner_id: DefId,
    inherited: &mut Vec<MembershipId>,
) {
    // 1. Collect owned member names for implicit redefinition
    let owned_names: HashSet<SymbolId> = model.defs[owner_id]
        .owned_memberships.iter()
        .map(|mid| model.defs[model.memberships[*mid].member_def].name)
        .collect();

    // 2. Collect explicit redefinition targets (resolved DefIds)
    let explicit_redefined: HashSet<DefId> = model.defs[owner_id]
        .feature_relationships.iter()
        .filter(|r| matches!(r.kind, FeatureRelKind::Redefinition))
        .filter_map(|r| r.target.resolved_def())
        .collect();

    // 3. Filter inherited memberships
    inherited.retain(|mid| {
        let member_def = model.memberships[*mid].member_def;
        let member_name = model.defs[member_def].name;

        // Explicit: redefines target matches inherited member
        if explicit_redefined.contains(&member_def) {
            return false;
        }

        // Implicit: owned member with same name shadows inherited
        if owned_names.contains(&member_name) {
            return false;
        }

        true
    });
}
```

### Diamond inheritance example

```kerml
type A { feature f : T; }
type B :> A { }
type C :> A { }
type D :> B, C { feature f :>> A::f; }
```

Pipeline for D:
1. Collect inherited from B: `[A::f (mid=3)]`
2. Collect inherited from C: `[A::f (mid=3)]`
3. Combined: `[mid=3, mid=3]`
4. MembershipId dedup: `[mid=3]`
5. `removeRedefinedFeatures`: D owns `f` (implicit) + explicit `redefines A::f` → remove mid=3
6. Result: `[]` — D has only its own `f`

## Validation

Basic checks for A4:

| Check | Error |
|-------|-------|
| Redefinition/subsetting target unresolved | "cannot resolve redefinition target `X`" |
| Target is not a feature | "redefinition target `X` is not a feature" |
| Duplicate redefinition of same target | warning: "feature `f` redefines `g` multiple times" |

Deep validation (A4-D1/D2) deferred.

## Serialization

New JSON-LD elements following existing `ownedSpecialization` pattern:

```json
{
  "@type": "Feature",
  "@id": "feat-f",
  "ownedRedefinition": [{
    "@type": "Redefinition",
    "redefiningFeature": { "@id": "feat-f" },
    "redefinedFeature": { "@id": "feat-g" }
  }],
  "ownedSubsetting": [{
    "@type": "Subsetting",
    "subsettingFeature": { "@id": "feat-f" },
    "subsettedFeature": { "@id": "feat-h" }
  }]
}
```

Chain targets (B3): serialized as nested feature chain references when resolved.

## Test Plan

### Parser tests
- Inline: `feature f subsets g;`
- Inline: `feature f :> g;` (in feature context = subsets)
- Inline: `feature f redefines g;`
- Inline: `feature f :>> g;`
- Inline: `feature f references g;`
- Inline: `feature f crosses g;`
- Inline combined: `feature f : T subsets g redefines h;`
- Inline multiple: `feature f subsets g, h;`
- Inline chain target: `feature f subsets a.b.c;` (parsed, chain stored)
- Standalone: `subset X subsets Y;`
- Standalone: `redefinition X redefines Y;`
- Standalone named: `specialization s1 subset X subsets Y;`
- Context disambiguation: `:>` as specializes in type vs subsets in feature

### Integration tests (fixtures)
- `valid/diamond_basic.kerml` — simple diamond, no redefinition
- `valid/diamond_redefine.kerml` — diamond with explicit redefinition
- `valid/implicit_redefine.kerml` — name-based shadowing without explicit redefines
- `valid/subsetting_basic.kerml` — feature subsets another feature
- `valid/redefinition_standalone.kerml` — standalone redefinition declaration
- `invalid/redefine_nonexistent.kerml` — target does not exist
- `invalid/redefine_non_feature.kerml` — target is a type, not a feature

### Property: ordering independence
Diamond dedup must produce identical `inherited_memberships` regardless of supertype declaration order (`D :> B, C` vs `D :> C, B`).

## Files Changed

| File | Change |
|------|--------|
| `kermlc_ast/src/nodes.rs` | `RelTarget`, fields on `FeatureDecl`, `SubsettingDecl`, `RedefinitionDecl` |
| `kermlc_lexer/src/lib.rs` | New tokens: `Redefines`, `ColonColonGt`, `Crosses`, `References` (if not present) |
| `kermlc_parser/src/lib.rs` | Parse inline relationships, standalone declarations, `:>` disambiguation |
| `kermlc_hir/src/types.rs` | `FeatureRelKind`, `FeatureRel`, `feature_relationships` on `Def` |
| `kermlc_hir/src/lower.rs` | Lower inline + standalone to `FeatureRel` |
| `kermlc_resolve/src/resolve.rs` | Resolve simple feature relationship targets |
| `kermlc_typeck/src/check.rs` | `remove_redefined_features()` after dedup |
| `kermlc_validate/src/lib.rs` | Target existence + kind validation |
| `kermlc_serial_json/src/lib.rs` | `ownedRedefinition`, `ownedSubsetting` emission |
| `kermlc/tests/` | New fixture files |
