# Visibility + Membership Layer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace bare `Vec<DefId>` parent-child links with arena-allocated `Membership` entities carrying visibility (`public`/`protected`/`private`) and kind (`Owning`/`Feature`/`Member`), enabling inherited membership filtering for A4.

**Architecture:** Membership is an arena-allocated struct (`MembershipId = Idx<Membership>`) stored in `SemanticModel.memberships`. `Def.children` becomes `Def.owned_memberships: Vec<MembershipId>`. `InheritedFeature` is replaced by `inherited_memberships: Vec<MembershipId>`. Direction for conjugation is computed on-demand via `direction_of()` instead of stored. `TypeInfo` is deleted entirely.

**Tech Stack:** Rust, Cargo workspace (11 crates), `kermlc_intern::Arena`/`Idx<T>` for arena allocation.

**Design doc:** `docs/plans/2026-03-28-visibility-membership-design.md`

---

### Task 1: Add lexer keywords

**Files:**
- Modify: `crates/kermlc_lexer/src/token.rs:4-12` (add variants)
- Modify: `crates/kermlc_lexer/src/lexer.rs:138-151` (add keyword match arms)
- Test: `crates/kermlc_lexer/src/lexer.rs:174-318` (existing tests module)

**Step 1: Write the failing test**

Add to `crates/kermlc_lexer/src/lexer.rs` tests module:

```rust
#[test]
fn lex_visibility_keywords() {
    let tokens = lex("public private protected member");
    assert_eq!(
        tokens,
        vec![
            (TokenKind::Public, "public"),
            (TokenKind::Private, "private"),
            (TokenKind::Protected, "protected"),
            (TokenKind::Member, "member"),
        ]
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_lexer -- lex_visibility_keywords`
Expected: FAIL — `Public` variant not found on `TokenKind`

**Step 3: Add token variants**

In `crates/kermlc_lexer/src/token.rs`, add after `InOut` (line 17):

```rust
    Public,
    Private,
    Protected,
    Member,
```

**Step 4: Add keyword matching**

In `crates/kermlc_lexer/src/lexer.rs`, add to the `match text` block (after `"inout"` arm, line 150):

```rust
                    "public" => TokenKind::Public,
                    "private" => TokenKind::Private,
                    "protected" => TokenKind::Protected,
                    "member" => TokenKind::Member,
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p kermlc_lexer`
Expected: ALL PASS

**Step 6: Commit**

```bash
git add crates/kermlc_lexer/
git commit -m "feat(lexer): add public/private/protected/member keywords"
```

---

### Task 2: Add Visibility and MembershipKind to AST

**Files:**
- Modify: `crates/kermlc_ast/src/nodes.rs` (add `Visibility`, `MembershipKind`, `MemberEntry`)

**Step 1: Write the failing test**

Add to `crates/kermlc_ast/src/nodes.rs` tests module:

```rust
#[test]
fn member_entry_default_visibility() {
    let entry = MemberEntry {
        visibility: None,
        is_member_only: false,
        member: Member::Type(Idx::from_raw(0)),
        span: Span::dummy(),
    };
    assert!(entry.visibility.is_none());
    assert!(!entry.is_member_only);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_ast -- member_entry_default_visibility`
Expected: FAIL — `MemberEntry` not found

**Step 3: Add types to AST**

Add before the `Member` enum in `crates/kermlc_ast/src/nodes.rs`:

```rust
/// Visibility of a membership (spec VisibilityKind).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

/// A member entry: the member plus its MemberPrefix metadata.
/// Wraps `Member` with visibility and `member` keyword info.
#[derive(Clone, Debug)]
pub struct MemberEntry {
    pub visibility: Option<Visibility>,
    pub is_member_only: bool,
    pub member: Member,
    pub span: Span,
}
```

**Step 4: Update containers to use `MemberEntry`**

Change `Vec<Member>` to `Vec<MemberEntry>` in:
- `PackageDecl.members` (line 32)
- `TypeDecl.members` (line 50)
- `SourceFile.members` (line 143)

**Step 5: Run test to verify it passes**

Run: `cargo test -p kermlc_ast`
Expected: PASS (existing tests will fail — fix in next step)

**Step 6: Fix existing AST tests**

Update `build_simple_ast` test — it doesn't use `members` directly so should still compile. If not, update the `PackageDecl` initialization to use empty `Vec<MemberEntry>`.

**Step 7: Run full AST tests**

Run: `cargo test -p kermlc_ast`
Expected: ALL PASS

**Step 8: Commit**

```bash
git add crates/kermlc_ast/
git commit -m "feat(ast): add Visibility, MemberEntry wrapper for members"
```

---

### Task 3: Parse MemberPrefix

**Files:**
- Modify: `crates/kermlc_parser/src/parser.rs` (add `parse_member_prefix`, update all member push sites)

**Step 1: Write the failing test**

Add to parser tests:

```rust
#[test]
fn parse_visibility_on_feature() {
    let (result, _, sink) = parse("package P { type T { private feature x : T; } }");
    assert!(!sink.has_errors(), "{:?}", sink.diagnostics());
    let pkg = &result.packages[result.source_file.packages[0]];
    let ty_entry = &pkg.members[0];
    let Member::Type(ty_id) = &ty_entry.member else { panic!("expected type") };
    let ty = &result.types[*ty_id];
    let feat_entry = &ty.members[0];
    assert_eq!(feat_entry.visibility, Some(Visibility::Private));
    let Member::Feature(_) = &feat_entry.member else { panic!("expected feature") };
}

#[test]
fn parse_member_keyword() {
    let (result, _, sink) = parse("package P { type T { member feature x : T; } }");
    assert!(!sink.has_errors(), "{:?}", sink.diagnostics());
    let pkg = &result.packages[result.source_file.packages[0]];
    let ty_entry = &pkg.members[0];
    let Member::Type(ty_id) = &ty_entry.member else { panic!("expected type") };
    let ty = &result.types[*ty_id];
    let feat_entry = &ty.members[0];
    assert!(feat_entry.is_member_only);
}

#[test]
fn parse_public_import() {
    let (result, _, sink) =
        parse("package P { public import A::*; }");
    assert!(!sink.has_errors(), "{:?}", sink.diagnostics());
}

#[test]
fn parse_default_visibility_is_none() {
    let (result, _, sink) = parse("package P { type T {} }");
    assert!(!sink.has_errors());
    let pkg = &result.packages[result.source_file.packages[0]];
    let entry = &pkg.members[0];
    assert_eq!(entry.visibility, None);
    assert!(!entry.is_member_only);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p kermlc_parser -- parse_visibility`
Expected: FAIL — compilation errors (Member → MemberEntry)

**Step 3: Add `parse_member_prefix` helper**

Add to parser impl:

```rust
/// Parse optional MemberPrefix: visibility? 'member'?
/// Returns (visibility, is_member_only, prefix_span).
fn parse_member_prefix(&mut self) -> (Option<Visibility>, bool, Span) {
    let start = self.current_span();
    let visibility = match self.peek() {
        TokenKind::Public => { self.bump(); Some(Visibility::Public) }
        TokenKind::Private => { self.bump(); Some(Visibility::Private) }
        TokenKind::Protected => { self.bump(); Some(Visibility::Protected) }
        _ => None,
    };
    let is_member_only = if self.at(TokenKind::Member) {
        self.bump();
        true
    } else {
        false
    };
    (visibility, is_member_only, start)
}
```

**Step 4: Update all member dispatch sites**

Three locations build `Member::*` and push to `members`:
1. Top-level parse loop (line 67-107)
2. Package body (line 229-269)
3. Type body (line ~355-380)

At each site, before the `match self.peek()`, call:
```rust
let (vis, is_member, prefix_span) = self.parse_member_prefix();
```

Then wrap each `members.push(Member::X(id))` as:
```rust
members.push(MemberEntry {
    visibility: vis,
    is_member_only: is_member,
    member: Member::X(id),
    span: Span::new(prefix_span.file, prefix_span.start, self.current_span().end),
});
```

Also add `TokenKind::Public | TokenKind::Private | TokenKind::Protected | TokenKind::Member` to the synchronize points so error recovery works.

**Step 5: Update import parsing for visibility**

In `parse_import`, add visibility parameter or parse it before calling. The `ImportDecl` AST node gets a `visibility: Option<Visibility>` field.

**Step 6: Fix all existing parser tests**

Every test that does `let Member::Type(ty_id) = &pkg.members[0]` becomes `let Member::Type(ty_id) = &pkg.members[0].member`. Mechanical replacement across ~20 test sites.

**Step 7: Run all parser tests**

Run: `cargo test -p kermlc_parser`
Expected: ALL PASS

**Step 8: Commit**

```bash
git add crates/kermlc_parser/ crates/kermlc_ast/
git commit -m "feat(parser): parse MemberPrefix (visibility, member keyword)"
```

---

### Task 4: Add Membership arena to HIR

**Files:**
- Modify: `crates/kermlc_hir/src/types.rs` (add `Membership` struct, `MembershipId`, `MembershipKind`, modify `Def`, modify `SemanticModel`, delete `InheritedFeature`, `InheritanceKind`, `TypeInfo`)

**Step 1: Write the failing test**

Add to `crates/kermlc_hir/src/types.rs` tests:

```rust
#[test]
fn alloc_membership() {
    let mut model = SemanticModel::new();
    let mut interner = StringInterner::new();
    let parent = model.alloc_def(
        Def::new(interner.intern("P"), DefKind::Package, Span::dummy()),
    );
    let child = model.alloc_def(
        Def::new(interner.intern("T"), DefKind::Type, Span::dummy()),
    );
    let mid = model.add_member(
        parent, child,
        Visibility::Public, MembershipKind::Owning, Span::dummy(),
    );
    assert_eq!(model.memberships[mid].member_def, child);
    assert_eq!(model.memberships[mid].visibility, Visibility::Public);
    assert_eq!(model.defs[child].parent, Some(parent));
    // children() convenience
    let children: Vec<DefId> = model.children(parent).collect();
    assert_eq!(children, vec![child]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_hir -- alloc_membership`
Expected: FAIL — `Membership` not defined

**Step 3: Add new types and modify existing**

In `crates/kermlc_hir/src/types.rs`:

a) Re-export `Visibility` from AST (or define it here and remove from AST — prefer defining in HIR and re-exporting from AST to avoid circular deps). Since AST doesn't depend on HIR, define `Visibility` in `kermlc_ast` and re-export in HIR:

```rust
pub use kermlc_ast::Visibility;
```

b) Add `MembershipKind` and `Membership`:

```rust
pub type MembershipId = Idx<Membership>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MembershipKind {
    Owning,
    Feature,
    Member,
}

#[derive(Clone, Debug)]
pub struct Membership {
    pub visibility: Visibility,
    pub kind: MembershipKind,
    pub member_def: DefId,
    pub owning_namespace: DefId,
    pub span: Span,
}
```

c) Modify `Def`:
- Replace `pub children: Vec<DefId>` with `pub owned_memberships: Vec<MembershipId>`
- Replace `pub inherited_features: Vec<InheritedFeature>` with `pub inherited_memberships: Vec<MembershipId>`
- Update `Def::new()` accordingly

d) Delete:
- `InheritedFeature` struct
- `InheritanceKind` enum
- `conjugate_direction()` function
- `TypeInfo` struct
- `type_infos: Arena<TypeInfo>` from `SemanticModel`
- `def_to_type: Vec<Option<TypeId>>` from `SemanticModel`
- `TypeId` type alias

e) Add `memberships: Arena<Membership>` to `SemanticModel`

f) Replace `add_child` with `add_member`:

```rust
pub fn add_member(
    &mut self, parent: DefId, child: DefId,
    visibility: Visibility, kind: MembershipKind, span: Span,
) -> MembershipId {
    let mid = self.memberships.alloc(Membership {
        visibility, kind, member_def: child,
        owning_namespace: parent, span,
    });
    self.defs[parent].owned_memberships.push(mid);
    self.defs[child].parent = Some(parent);
    mid
}
```

g) Add convenience methods:

```rust
pub fn children(&self, def: DefId) -> impl Iterator<Item = DefId> + '_ {
    self.defs[def].owned_memberships.iter()
        .map(|&mid| self.memberships[mid].member_def)
}

pub fn find_child(&self, parent: DefId, name: SymbolId) -> Option<DefId> {
    self.children(parent)
        .find(|&child| self.defs[child].name == name)
}
```

h) Add `direction_of`:

```rust
pub fn direction_of(&self, feature: DefId, in_type: DefId) -> Option<FeatureDirection> {
    self.direction_of_excluding(feature, in_type, &mut Vec::new())
}

fn direction_of_excluding(
    &self, feature: DefId, in_type: DefId, excluded: &mut Vec<DefId>,
) -> Option<FeatureDirection> {
    // 1. Check if feature is directly owned by in_type
    if self.defs[feature].parent == Some(in_type) {
        let dir = self.defs[feature].direction;
        return if self.defs[in_type].conjugation.is_some() {
            dir.map(|d| conjugate_dir(d))
        } else {
            dir
        };
    }
    // 2. Walk supertypes
    excluded.push(in_type);
    for spec in &self.defs[in_type].specializations {
        if let Some(super_id) = spec.resolved_def() {
            if excluded.contains(&super_id) { continue; }
            if let Some(dir) = self.direction_of_excluding(
                feature, super_id, excluded,
            ) {
                return if self.defs[in_type].conjugation.is_some() {
                    Some(conjugate_dir(dir))
                } else {
                    Some(dir)
                };
            }
        }
    }
    // 3. Check conjugation target
    if let Some(conj) = &self.defs[in_type].conjugation {
        if let Some(conj_id) = conj.resolved_def() {
            if !excluded.contains(&conj_id) {
                if let Some(dir) = self.direction_of_excluding(
                    feature, conj_id, excluded,
                ) {
                    return Some(conjugate_dir(dir));
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
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p kermlc_hir -- alloc_membership`
Expected: PASS

**Step 5: Fix remaining HIR tests**

Update all tests that reference `children`, `inherited_features`, `TypeInfo`, etc. The `conjugate_direction` tests become `direction_of` tests.

**Step 6: Run all HIR tests**

Run: `cargo test -p kermlc_hir`
Expected: ALL PASS

**Step 7: Commit**

```bash
git add crates/kermlc_hir/
git commit -m "feat(hir): add Membership arena, replace children/InheritedFeature"
```

---

### Task 5: Update lowering to create Memberships

**Files:**
- Modify: `crates/kermlc_hir/src/lower.rs` (use `add_member` instead of `add_child`, pass visibility from AST)
- Modify: `crates/kermlc_hir/src/stdlib.rs` (stdlib types use `add_member`)

**Step 1: Write the failing test**

Add to `crates/kermlc_hir/src/lower.rs` tests:

```rust
#[test]
fn lower_private_feature() {
    let (model, interner, sink) =
        lower("package P { type T { private feature x : T; } }");
    assert!(!sink.has_errors(), "{:?}", sink.diagnostics());

    let pkg = model.roots[0];
    let ty = model.children(pkg).next().unwrap();
    let feat_mid = model.defs[ty].owned_memberships[0];
    let membership = &model.memberships[feat_mid];
    assert_eq!(membership.visibility, Visibility::Private);
    assert_eq!(membership.kind, MembershipKind::Feature);
    assert_eq!(interner.resolve(model.defs[membership.member_def].name), "x");
}

#[test]
fn lower_member_keyword() {
    let (model, _interner, sink) =
        lower("package P { type T { member feature x : T; } }");
    assert!(!sink.has_errors(), "{:?}", sink.diagnostics());

    let pkg = model.roots[0];
    let ty = model.children(pkg).next().unwrap();
    let feat_mid = model.defs[ty].owned_memberships[0];
    assert_eq!(model.memberships[feat_mid].kind, MembershipKind::Member);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_hir -- lower_private`
Expected: FAIL

**Step 3: Update `lower_member`**

Change signature to accept `MemberEntry`:

```rust
fn lower_member(&mut self, entry: &kermlc_ast::MemberEntry, parent: DefId) {
    let visibility = entry.visibility.unwrap_or(Visibility::Public);
    let defs = match &entry.member {
        kermlc_ast::Member::Package(id) => vec![self.lower_package(*id)],
        kermlc_ast::Member::Type(id) => vec![self.lower_type(*id)],
        kermlc_ast::Member::Feature(id) => self.lower_feature(*id),
        kermlc_ast::Member::Conjugation(id) => vec![self.lower_conjugation_decl(*id)],
    };
    for def_id in defs {
        let kind = if entry.is_member_only {
            MembershipKind::Member
        } else {
            match &entry.member {
                kermlc_ast::Member::Feature(_) => MembershipKind::Feature,
                _ => MembershipKind::Owning,
            }
        };
        self.model.add_member(parent, def_id, visibility, kind, entry.span);
    }
}
```

Update all call sites (`lower_package`, `lower_type`, top-level loop) to pass `parent` DefId and use `lower_member(entry, parent)` instead of `lower_member(member)` + `add_child`.

**Step 4: Update `load_stdlib`**

Replace `model.roots.push(...)` with proper membership. Since stdlib types are roots (no parent package), they stay as `model.roots.push(id)` without membership. No change needed — stdlib types are root-level, not children of a namespace.

**Step 5: Fix all existing lowering tests**

Replace `model.defs[pkg].children[0]` with `model.children(pkg).nth(0).unwrap()` or collect into Vec first. Mechanical replacement.

**Step 6: Run all tests**

Run: `cargo test -p kermlc_hir`
Expected: ALL PASS

**Step 7: Commit**

```bash
git add crates/kermlc_hir/
git commit -m "feat(lower): create Membership per member, propagate visibility"
```

---

### Task 6: Update resolve pass for Membership

**Files:**
- Modify: `crates/kermlc_resolve/src/scope.rs` (update `find_member`, `resolve_via_imports`)
- Modify: `crates/kermlc_resolve/src/resolve.rs` (update `.children` references)

**Step 1: Write the failing test**

Add to `crates/kermlc_resolve/src/scope.rs` tests:

```rust
#[test]
fn find_member_respects_private() {
    let (mut model, mut interner, mut sink) =
        parse_and_lower(
            "package P { type A { private feature secret : A; feature visible : A; } type B :> A {} }"
        );
    for _ in 0..10 {
        let r = crate::resolve_pass(&mut model, &interner, &mut sink);
        let t = kermlc_typeck::typecheck_pass(&mut model, &interner, &mut sink);
        if !r && !t { break; }
    }

    let pkg = model.roots[0];
    let b_id = model.children(pkg).nth(1).unwrap();

    // B should NOT inherit private feature "secret"
    let secret_name = interner.intern("secret");
    let found = find_member(&model, b_id, secret_name);
    assert_eq!(found, None, "private feature should not be inherited");

    // B should inherit "visible"
    let visible_name = interner.intern("visible");
    let found = find_member(&model, b_id, visible_name);
    assert!(found.is_some(), "public feature should be inherited");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_resolve -- find_member_respects_private`
Expected: FAIL (compilation errors from `.children` usage)

**Step 3: Update scope.rs**

Replace all `model.defs[x].children` with `model.children(x)` calls. Update `find_member` to use `inherited_memberships`:

```rust
pub fn find_member(
    model: &SemanticModel, type_def_id: DefId, name: SymbolId,
) -> Option<DefId> {
    // 1. Direct children (owned memberships)
    if let Some(found) = model.find_child(type_def_id, name) {
        return Some(found);
    }
    // 2. Inherited memberships
    for &mid in &model.defs[type_def_id].inherited_memberships {
        let m = &model.memberships[mid];
        if model.defs[m.member_def].name == name {
            return Some(m.member_def);
        }
    }
    None
}
```

**Step 4: Update resolve.rs**

Replace all `model.defs[x].children[n]` in tests with `model.children(x).nth(n).unwrap()`.

**Step 5: Run all resolve tests**

Run: `cargo test -p kermlc_resolve`
Expected: ALL PASS

**Step 6: Commit**

```bash
git add crates/kermlc_resolve/
git commit -m "refactor(resolve): use Membership-based children/find_member"
```

---

### Task 7: Update typeck to collect inherited Memberships

**Files:**
- Modify: `crates/kermlc_typeck/src/check.rs` (replace `InheritedFeature` logic with `MembershipId` collection)

**Step 1: Write the failing test**

Add to `crates/kermlc_typeck/src/check.rs` tests:

```rust
#[test]
fn private_features_not_inherited() {
    let (model, interner, sink) = compile_to_model(
        "package P { type A { private feature secret : A; feature visible : A; } type B :> A {} }"
    );
    assert!(!sink.has_errors(), "{:?}", sink.diagnostics());

    let pkg = model.roots[0];
    let b_id = model.children(pkg).nth(1).unwrap();
    assert_eq!(interner.resolve(model.defs[b_id].name), "B");

    // B should only inherit "visible", not "secret"
    assert_eq!(
        model.defs[b_id].inherited_memberships.len(), 1,
        "B should inherit only public feature, not private"
    );
    let mid = model.defs[b_id].inherited_memberships[0];
    let name = interner.resolve(model.defs[model.memberships[mid].member_def].name);
    assert_eq!(name, "visible");
}

#[test]
fn protected_features_inherited() {
    let (model, interner, sink) = compile_to_model(
        "package P { type A { protected feature prot : A; } type B :> A {} }"
    );
    assert!(!sink.has_errors());

    let pkg = model.roots[0];
    let b_id = model.children(pkg).nth(1).unwrap();
    assert_eq!(model.defs[b_id].inherited_memberships.len(), 1);
}

#[test]
fn direction_of_direct_feature() {
    let (model, _interner, _sink) = compile_to_model(
        "package P { type A { in feature f : A; } }"
    );
    let pkg = model.roots[0];
    let a_id = model.children(pkg).next().unwrap();
    let f_id = model.children(a_id).next().unwrap();
    assert_eq!(
        model.direction_of(f_id, a_id),
        Some(FeatureDirection::In),
    );
}

#[test]
fn direction_of_conjugated() {
    let (model, interner, _sink) = compile_to_model(
        "package P { type A { in feature f : A; } type B ~ A {} }"
    );
    let pkg = model.roots[0];
    let a_id = model.children(pkg).nth(0).unwrap();
    let f_id = model.children(a_id).next().unwrap();
    let b_id = model.children(pkg).nth(1).unwrap();
    assert_eq!(interner.resolve(model.defs[b_id].name), "B");
    assert_eq!(
        model.direction_of(f_id, b_id),
        Some(FeatureDirection::Out),
        "conjugation should flip in→out"
    );
}
```

**Step 2: Run test to verify they fail**

Run: `cargo test -p kermlc_typeck -- private_features`
Expected: FAIL

**Step 3: Rewrite `check_type`**

```rust
fn check_type(model: &mut SemanticModel, def_id: DefId) -> bool {
    let mut changed = false;

    let all_specs_resolved = model.defs[def_id]
        .specializations.iter().all(|s| s.is_resolved());
    if !all_specs_resolved { return false; }

    if let Some(conj) = &model.defs[def_id].conjugation {
        if !conj.is_resolved() { return false; }
    }

    // Collect inherited memberships from supertypes
    let supertype_ids: Vec<DefId> = model.defs[def_id]
        .specializations.iter()
        .filter_map(|s| s.resolved_def()).collect();

    let mut inherited: Vec<MembershipId> = Vec::new();
    for &super_id in &supertype_ids {
        // Own feature memberships of supertype (non-private)
        for &mid in &model.defs[super_id].owned_memberships {
            let m = &model.memberships[mid];
            if m.visibility != Visibility::Private
                && m.kind == MembershipKind::Feature
            {
                inherited.push(mid);
            }
        }
        // Transitively inherited memberships of supertype
        let super_inherited = model.defs[super_id]
            .inherited_memberships.clone();
        inherited.extend(super_inherited);
    }

    // Collect from conjugation target
    if let Some(conj) = &model.defs[def_id].conjugation {
        if let Some(conj_id) = conj.resolved_def() {
            for &mid in &model.defs[conj_id].owned_memberships {
                let m = &model.memberships[mid];
                if m.visibility != Visibility::Private
                    && m.kind == MembershipKind::Feature
                {
                    inherited.push(mid);
                }
            }
            let conj_inherited = model.defs[conj_id]
                .inherited_memberships.clone();
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
```

**Step 4: Update `check_conjugation_decl`**

Same logic but references `owned_memberships` instead of `children`.

**Step 5: Fix existing typeck tests**

Replace `inherited_features` references with `inherited_memberships` + membership lookup. Replace `InheritanceKind` checks with membership queries. Replace `direction_override` checks with `model.direction_of()` calls.

**Step 6: Run all typeck tests**

Run: `cargo test -p kermlc_typeck`
Expected: ALL PASS

**Step 7: Commit**

```bash
git add crates/kermlc_typeck/
git commit -m "feat(typeck): inherit MembershipIds, filter private, direction_of()"
```

---

### Task 8: Update validate

**Files:**
- Modify: `crates/kermlc_validate/src/validate.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn inherited_name_conflict_with_owned() {
    let (_model, sink) = compile_and_validate(
        "package P { type A { feature x : A; } type B :> A { feature x : A; } }"
    );
    // This should be valid — owned feature shadows inherited
    assert!(!sink.has_errors());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_validate -- inherited_name_conflict`
Expected: FAIL (compilation errors from `.children` / `inherited_features`)

**Step 3: Update validate functions**

Replace all `def.children.iter()` with `model.children(def_id)`. Replace `inherited_features` with `inherited_memberships` + membership lookup. The duplicate feature check iterates `owned_memberships` filtered by `MembershipKind::Feature`.

The conjugation warning that checks for "has no features":
```rust
let has_features = model.children(target_id)
    .any(|c| model.defs[c].kind == DefKind::Feature);
```

The redefinition multiplicity check uses `inherited_memberships` instead of `inherited_features`:
```rust
for &mid in &parent_def.inherited_memberships {
    let inherited_id = model.memberships[mid].member_def;
    let inherited = &model.defs[inherited_id];
    if inherited.name == def.name {
        validate_redefinition_multiplicity(model, interner, def_id, inherited_id, sink);
    }
}
```

**Step 4: Run all validate tests**

Run: `cargo test -p kermlc_validate`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add crates/kermlc_validate/
git commit -m "refactor(validate): use Membership-based children/inherited"
```

---

### Task 9: Update serialization

**Files:**
- Modify: `crates/kermlc_serial_json/src/serialize.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn serialize_visibility() {
    let json = compile_and_serialize(
        "package P { type T { private feature x : T; protected feature y : T; } }"
    );
    let value: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    let x_elem = value.iter().find(|e| e["name"] == "x").unwrap();
    // Membership wrapper should have visibility
    let t_elem = value.iter().find(|e| e["name"] == "T").unwrap();
    let members = t_elem["ownedMembership"].as_array().unwrap();
    let x_mem = members.iter().find(|m| {
        m.get("memberElement").and_then(|e| e.get("name"))
            .map(|n| n == "x").unwrap_or(false)
    });
    assert!(x_mem.is_some());
}
```

**Step 2: Run test to verify it fails**

Expected: FAIL (compilation + no visibility in output)

**Step 3: Update serialization**

Replace `def.children` iteration with `def.owned_memberships` iteration. Emit `ownedMembership` array with `Membership` objects including `visibility` and `@type` (`OwningMembership`/`FeatureMembership`).

For inherited features, use `inherited_memberships` + `model.direction_of()`:

```rust
if !def.inherited_memberships.is_empty() {
    let inherited_refs: Vec<Value> = def.inherited_memberships.iter()
        .map(|&mid| {
            let m = &model.memberships[mid];
            let feat_id = m.member_def;
            let mut obj = json!({
                "@id": format!("feature-{}", feat_id.raw()),
            });
            if let Some(dir) = model.direction_of(feat_id, def_id) {
                obj["direction"] = json!(match dir {
                    FeatureDirection::In => "in",
                    FeatureDirection::Out => "out",
                    FeatureDirection::InOut => "inout",
                });
            }
            obj
        })
        .collect();
    element["inheritedFeature"] = json!(inherited_refs);
}
```

**Step 4: Run all serialization tests**

Run: `cargo test -p kermlc_serial_json`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add crates/kermlc_serial_json/
git commit -m "feat(serial): emit Membership objects with visibility in JSON-LD"
```

---

### Task 10: Update integration tests and pipeline

**Files:**
- Modify: `crates/kermlc/tests/integration.rs`
- Modify: `crates/kermlc/src/pipeline.rs`
- Create: `crates/kermlc/tests/fixtures/valid/visibility.kerml`

**Step 1: Create visibility test fixture**

```kerml
// Tests visibility: public (default), protected, private
package Visibility {
    type Base {
        feature pub_feat : Base;
        protected feature prot_feat : Base;
        private feature priv_feat : Base;
    }

    type Sub :> Base {
        // Should inherit pub_feat and prot_feat, but NOT priv_feat
    }

    type SubSub :> Sub {
        // Should transitively inherit pub_feat and prot_feat
    }
}
```

**Step 2: Write integration test**

```rust
#[test]
fn valid_visibility() {
    let result = compile_file(&fixtures_dir().join("valid/visibility.kerml"));
    assert!(
        !result.sink.has_errors(),
        "Errors in visibility.kerml: {:?}",
        result.sink.diagnostics()
    );

    let pkg = result.model.roots[0];
    let sub_id = result.model.children(pkg).nth(1).unwrap();
    assert_eq!(result.interner.resolve(result.model.defs[sub_id].name), "Sub");

    // Sub should inherit pub_feat and prot_feat but not priv_feat
    assert_eq!(
        result.model.defs[sub_id].inherited_memberships.len(), 2,
        "Sub should inherit 2 features (public + protected), not private"
    );

    let sub_sub_id = result.model.children(pkg).nth(2).unwrap();
    assert_eq!(
        result.model.defs[sub_sub_id].inherited_memberships.len(), 2,
        "SubSub should transitively inherit 2 features"
    );
}
```

**Step 3: Fix all existing integration tests**

Replace all `.children[n]` with `model.children(x).nth(n).unwrap()`. Replace `inherited_features` with `inherited_memberships` + membership access. Replace `InheritanceKind` and `direction_override` checks with `model.direction_of()`.

The conjugation integration tests (`valid_conjugation`, `valid_conjugation_chained`, etc.) change from checking `inh.direction_override` to checking `model.direction_of(feat_id, type_id)`.

**Step 4: Fix pipeline.rs**

Replace `.children` with `model.children()` in tests.

**Step 5: Run full test suite**

Run: `cargo test`
Expected: ALL PASS

**Step 6: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: No warnings

**Step 7: Run fmt**

Run: `cargo fmt --check`
Expected: Clean

**Step 8: Commit**

```bash
git add crates/kermlc/ crates/kermlc_hir/ crates/kermlc_typeck/
git commit -m "feat: add visibility fixture, update integration tests for Membership model"
```

---

### Task 11: Update import visibility

**Files:**
- Modify: `crates/kermlc_hir/src/types.rs` (Import struct)
- Modify: `crates/kermlc_hir/src/lower.rs` (pass import visibility)
- Modify: `crates/kermlc_resolve/src/scope.rs` (filter by import visibility)

**Step 1: Write the failing test**

Add to resolve tests:

```rust
#[test]
fn public_import_makes_members_visible() {
    let (mut model, interner, mut sink) = parse_and_lower(
        "package A { type X {} } package B { public import A::*; } package C { import B::*; type Y :> X {} }"
    );
    for _ in 0..10 {
        let r = crate::resolve_pass(&mut model, &interner, &mut sink);
        if !r { break; }
    }
    // X should be visible in C through B's public import
    let c_pkg = model.roots[2];
    let y_id = model.children(c_pkg).next().unwrap();
    assert!(model.defs[y_id].specializations[0].is_resolved());
}
```

**Step 2: Run test**

Expected: FAIL

**Step 3: Add visibility to Import HIR struct**

```rust
pub struct Import {
    pub path: NameRef,
    pub is_wildcard: bool,
    pub visibility: Visibility,
    pub span: Span,
}
```

Default in lowering: `Visibility::Private`.

**Step 4: Update lowering**

Pass `import.visibility.unwrap_or(Visibility::Private)` from AST `ImportDecl` to HIR `Import`.

**Step 5: Update resolve_via_imports**

Filter: when resolving through imports from outside the namespace, only consider imports whose visibility allows it. For now, all imports are visible within the namespace (the import visibility affects re-export, not internal use). The filtering happens when another namespace tries to see imported members.

**Step 6: Run tests**

Run: `cargo test -p kermlc_resolve`
Expected: ALL PASS

**Step 7: Commit**

```bash
git add crates/kermlc_hir/ crates/kermlc_resolve/
git commit -m "feat(resolve): import visibility (default private)"
```

---

### Task 12: Clean up and update progress

**Files:**
- Modify: `docs/plans/progress.md`

**Step 1: Verify no dead code**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: Clean

**Step 2: Verify formatting**

Run: `cargo fmt --check`
Expected: Clean

**Step 3: Run full test suite one final time**

Run: `cargo test`
Expected: ALL PASS

**Step 4: Update progress tracker**

Mark A5 as complete in `docs/plans/progress.md`. Add note about Membership layer.

**Step 5: Commit**

```bash
git add docs/plans/progress.md
git commit -m "docs: mark A5 visibility + membership layer as complete"
```
