# A1c: Inline Conjugated Type Refs

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `feature f : ~T;` syntax — a feature typed by an anonymous type that conjugates `T`.

**Architecture:** Introduce `TypeExpr` enum in the AST to represent typed-by references (named, conjugated, or chain). The parser wraps qualified names in `TypeExpr::Named` or `TypeExpr::Conjugated` after `:`. During HIR lowering, `Conjugated` triggers synthesis of an anonymous type def with its `conjugation` field set. The existing fixpoint resolve/typeck loop handles the rest without modification.

**Tech Stack:** Rust, Cargo workspace (kermlc_ast, kermlc_parser, kermlc_hir, kermlc)

---

## Design (reference)

Date: 2026-03-11

### TypeExpr Abstraction

```rust
enum TypeExpr {
    Named(QualifiedName),               // T
    Conjugated(QualifiedName, Span),    // ~T
    Chain(FeatureChain),                // a.b (future)
}
```

Replaces `FeatureDecl.type_ref: Option<QualifiedName>` with `Option<TypeExpr>`.
Feature-level conjugation (`conjugation` field) and chaining (`chain` field)
stay separate — KerML grammar treats them as distinct productions.

### Data Flow

```
Source: feature f : ~T;
  → Parser: TypeExpr::Conjugated(QualifiedName("T"), span)
  → Lowering:
      1. Synthesize Def { name: "~T", kind: Type, conjugation: NameRef("T", Unresolved) }
      2. Feature f: type_ref = NameRef([], Resolved(anon_def_id))
  → Resolution: resolves anon type's conjugation NameRef("T") → DefId of T
  → Typeck: check_type on anon type → inherits T's features with flipped directions
  → Feature f is typed by the anonymous conjugated type
```

---

## Task 1: Add TypeExpr to AST and update FeatureDecl

**Files:**
- Modify: `crates/kermlc_ast/src/nodes.rs`

**Step 1: Add `TypeExpr` enum and update `FeatureDecl.type_ref`**

In `crates/kermlc_ast/src/nodes.rs`, add this enum after the `FeatureChain` struct (line 66):

```rust
/// A type reference in a typing (`:`) position.
/// Maps to KerML's GeneralType production + conjugation extension.
#[derive(Clone, Debug)]
pub enum TypeExpr {
    /// Plain named reference: `T` or `A::B`
    Named(QualifiedName),
    /// Conjugated type reference: `~T`
    Conjugated(QualifiedName, Span),
    /// Feature chain used as type: `a.b` (future)
    Chain(FeatureChain),
}
```

Then change `FeatureDecl.type_ref` from `Option<QualifiedName>` to `Option<TypeExpr>`:

```rust
pub struct FeatureDecl {
    pub name: SymbolId,
    pub span: Span,
    pub direction: Option<FeatureDirection>,
    pub type_ref: Option<TypeExpr>,           // was Option<QualifiedName>
    pub conjugation: Option<QualifiedName>,   // unchanged
    pub chain: Option<FeatureChain>,          // unchanged
    pub multiplicity: Option<Multiplicity>,
}
```

**Step 2: Build to verify — expect compile errors in downstream crates**

Run: `cargo build -p kermlc_ast 2>&1 | head -5`
Expected: kermlc_ast builds OK. Downstream crates (parser, hir) will fail until updated.

**Step 3: Commit**

```bash
git add crates/kermlc_ast/src/nodes.rs
git commit -m "feat(ast): add TypeExpr enum, update FeatureDecl.type_ref"
```

---

## Task 2: Update parser to produce TypeExpr

**Files:**
- Modify: `crates/kermlc_parser/src/parser.rs`

**Step 1: Write failing test for `feature f : ~T;`**

Add this test at the end of the `#[cfg(test)] mod tests` block in `parser.rs`:

```rust
#[test]
fn parse_inline_conjugated_type_ref() {
    let (result, interner, sink) =
        parse("package P { type T { feature f : ~T; } }");
    assert!(!sink.has_errors(), "errors: {:?}", sink.diagnostics());
    let pkg = &result.packages[result.source_file.packages[0]];
    let Member::Type(ty_id) = &pkg.members[0] else {
        panic!("expected type member");
    };
    let ty = &result.types[*ty_id];
    let Member::Feature(feat_id) = &ty.members[0] else {
        panic!("expected feature member");
    };
    let feat = &result.features[*feat_id];
    assert_eq!(interner.resolve(feat.name), "f");
    let type_ref = feat.type_ref.as_ref().expect("should have type_ref");
    match type_ref {
        TypeExpr::Conjugated(qn, _) => {
            assert_eq!(qn.segments.len(), 1);
            assert_eq!(interner.resolve(qn.segments[0]), "T");
        }
        _ => panic!("expected TypeExpr::Conjugated, got {:?}", type_ref),
    }
    assert!(feat.conjugation.is_none(), "conjugation field should be None");
}
```

Add a second test for qualified conjugated ref:

```rust
#[test]
fn parse_inline_conjugated_type_ref_qualified() {
    let (result, interner, sink) =
        parse("package P { type T { feature f : ~A::B; } }");
    assert!(!sink.has_errors(), "errors: {:?}", sink.diagnostics());
    let pkg = &result.packages[result.source_file.packages[0]];
    let Member::Type(ty_id) = &pkg.members[0] else {
        panic!("expected type member");
    };
    let ty = &result.types[*ty_id];
    let Member::Feature(feat_id) = &ty.members[0] else {
        panic!("expected feature member");
    };
    let feat = &result.features[*feat_id];
    match feat.type_ref.as_ref().unwrap() {
        TypeExpr::Conjugated(qn, _) => {
            assert_eq!(qn.segments.len(), 2);
            assert_eq!(interner.resolve(qn.segments[0]), "A");
            assert_eq!(interner.resolve(qn.segments[1]), "B");
        }
        _ => panic!("expected TypeExpr::Conjugated"),
    }
}
```

**Step 2: Update `parse_feature` to produce `TypeExpr`**

In `parse_feature()` (around line 477-489), replace the typing/conjugation block:

```rust
// Parse typing `:` or conjugation `~`/`conjugates`
if self.at(TokenKind::Colon) {
    self.bump();
    if self.at(TokenKind::Tilde) || self.at(TokenKind::Conjugates) {
        let conj_start = self.current_span();
        self.bump();
        if let Some(qn) = self.parse_qualified_name() {
            let span = Span::new(
                conj_start.file,
                conj_start.start,
                qn.span.end,
            );
            type_ref = Some(TypeExpr::Conjugated(qn, span));
        }
    } else if let Some(qn) = self.parse_qualified_name() {
        type_ref = Some(TypeExpr::Named(qn));
    }
} else if self.at(TokenKind::Tilde) || self.at(TokenKind::Conjugates) {
    self.bump();
    conjugation = self.parse_qualified_name();
}
```

**Step 3: Fix existing parser tests that check `type_ref`**

The test `parse_feature_conjugation_tilde` (line 760) asserts
`feat.type_ref.is_none()` — this still holds since `feature g ~ T;`
(without `:`) sets `conjugation`, not `type_ref`. No change needed.

Any test that pattern-matches on `feat.type_ref` as a `QualifiedName`
now needs to match `TypeExpr::Named(qn)` instead. The only one is
implicit in the existing test assertions that just call `.is_none()` or
`.is_some()` — these still work on `Option<TypeExpr>`.

**Step 4: Add `use` for `TypeExpr` in test module**

At the top of the test module in `parser.rs`, ensure `TypeExpr` is in scope.
It should be already via `use super::*` since `parser.rs` re-exports
from `kermlc_ast`. If not, add: `use kermlc_ast::TypeExpr;`

**Step 5: Run parser tests**

Run: `cargo test -p kermlc_parser -- --nocapture 2>&1 | tail -20`
Expected: All tests pass including the two new ones.

**Step 6: Commit**

```bash
git add crates/kermlc_parser/src/parser.rs
git commit -m "feat(parser): parse inline conjugated type refs (: ~T)"
```

---

## Task 3: Update HIR lowering for TypeExpr + anonymous type synthesis

**Files:**
- Modify: `crates/kermlc_hir/src/lower.rs`

**Step 1: Write failing test for conjugated type lowering**

Add this test in `lower.rs`'s test module:

```rust
#[test]
fn lower_inline_conjugated_type_ref() {
    let (model, interner, sink) =
        lower("package P { type T { in feature f : T; } type U { feature g : ~T; } }");
    assert!(!sink.has_errors(), "errors: {:?}", sink.diagnostics());

    let pkg = &model.defs[model.roots[0]];
    let u_id = pkg.children[1];
    let g_id = model.defs[u_id].children[0];
    let g = &model.defs[g_id];

    assert_eq!(interner.resolve(g.name), "g");
    assert!(g.type_ref.is_some(), "g should have type_ref");
    let type_ref = g.type_ref.as_ref().unwrap();
    assert!(
        type_ref.is_resolved(),
        "type_ref should be pre-resolved to anonymous type"
    );

    let anon_id = type_ref.resolved_def().unwrap();
    let anon = &model.defs[anon_id];
    assert_eq!(anon.kind, DefKind::Type);
    assert_eq!(interner.resolve(anon.name), "~T");
    assert!(
        anon.conjugation.is_some(),
        "anonymous type should have conjugation"
    );
    assert_eq!(
        anon.conjugation.as_ref().unwrap().resolution,
        ResolutionState::Unresolved,
        "conjugation target should be unresolved at lowering time"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_hir -- lower_inline_conjugated_type_ref --nocapture 2>&1 | tail -10`
Expected: FAIL — `lower_feature` doesn't handle `TypeExpr` yet.

**Step 3: Change `lower_ast` signature to take `&mut StringInterner`**

In `lower.rs`, change:

```rust
pub fn lower_ast(
    parse: &kermlc_parser::ParseResult,
    interner: &mut StringInterner,    // was &StringInterner
    _sink: &mut DiagnosticSink,
) -> SemanticModel {
```

Also update `LowerCtx`:

```rust
struct LowerCtx<'a> {
    model: &'a mut SemanticModel,
    parse: &'a kermlc_parser::ParseResult,
    interner: &'a mut StringInterner,    // was &'a StringInterner, remove #[allow(dead_code)]
}
```

**Step 4: Update `lower_feature` to handle `TypeExpr`**

Replace the type_ref lowering block (lines 108-114) with:

```rust
match &feat.type_ref {
    Some(kermlc_ast::TypeExpr::Named(qn)) => {
        def.type_ref = Some(NameRef::unresolved(
            qn.segments.clone(),
            qn.span,
        ));
    }
    Some(kermlc_ast::TypeExpr::Conjugated(qn, span)) => {
        let anon_id = self.synthesize_conjugated_type(qn, *span);
        def.type_ref = Some(NameRef {
            segments: vec![],
            span: *span,
            resolution: ResolutionState::Resolved(anon_id),
        });
    }
    Some(kermlc_ast::TypeExpr::Chain(_)) => {
        // Future: chain-as-type lowering
    }
    None => {}
}
```

**Step 5: Add `synthesize_conjugated_type` method to `LowerCtx`**

Add this method to `impl<'a> LowerCtx<'a>`:

```rust
fn synthesize_conjugated_type(
    &mut self,
    original: &kermlc_ast::QualifiedName,
    span: Span,
) -> DefId {
    let last_seg = *original.segments.last().expect("empty qualified name");
    let orig_name = self.interner.resolve(last_seg);
    let synth_name = self.interner.intern(&format!("~{orig_name}"));

    let mut anon_def = Def::new(synth_name, DefKind::Type, span);
    anon_def.conjugation = Some(NameRef::unresolved(
        original.segments.clone(),
        original.span,
    ));
    self.model.alloc_def(anon_def)
}
```

**Step 6: Update test helper `lower()` to pass `&mut interner`**

In the test module, the `lower` function already has `let mut interner`.
Change the call from `lower_ast(&parse, &interner, &mut sink)` to
`lower_ast(&parse, &mut interner, &mut sink)`.

**Step 7: Run lowering tests**

Run: `cargo test -p kermlc_hir -- --nocapture 2>&1 | tail -20`
Expected: All tests pass including `lower_inline_conjugated_type_ref`.

**Step 8: Commit**

```bash
git add crates/kermlc_hir/src/lower.rs
git commit -m "feat(hir): synthesize anonymous conjugated types from TypeExpr::Conjugated"
```

---

## Task 4: Fix all callers of `lower_ast` (signature change)

**Files:**
- Modify: `crates/kermlc/src/main.rs` (lines 86, 139)
- Modify: `crates/kermlc/src/pipeline.rs` (line 44)
- Modify: `crates/kermlc/tests/integration.rs` (line 20)
- Modify: `crates/kermlc_resolve/src/resolve.rs` (line 372)
- Modify: `crates/kermlc_typeck/src/check.rs` (line 238)
- Modify: `crates/kermlc_validate/src/validate.rs` (line 354)
- Modify: `crates/kermlc_serial_json/src/serialize.rs` (line 212)

**Step 1: Update every `lower_ast(&parse, &interner,` to `lower_ast(&parse, &mut interner,`**

Each call site already has `let mut interner = StringInterner::new()` (or receives
`&mut StringInterner`), so the change is mechanical: replace `&interner` with
`&mut interner` in the second argument of every `lower_ast` call.

Files and approximate line numbers:
- `crates/kermlc/src/main.rs:86` — `run_check`
- `crates/kermlc/src/main.rs:139` — `run_compile`
- `crates/kermlc/src/pipeline.rs:44` — `compile` test helper
- `crates/kermlc/tests/integration.rs:20` — `compile_source`
- `crates/kermlc_resolve/src/resolve.rs:372` — `parse_and_lower` test helper
- `crates/kermlc_typeck/src/check.rs:238` — `compile_to_model` test helper
- `crates/kermlc_validate/src/validate.rs:354` — test helper
- `crates/kermlc_serial_json/src/serialize.rs:212` — `compile_and_serialize` test helper

**Step 2: Build the entire workspace**

Run: `cargo build 2>&1 | tail -10`
Expected: Clean build, no errors.

**Step 3: Run full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: All existing tests pass.

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: update all lower_ast callers for &mut StringInterner"
```

---

## Task 5: Add integration test fixture and end-to-end test

**Files:**
- Create: `crates/kermlc/tests/fixtures/valid/inline_conjugation.kerml`
- Modify: `crates/kermlc/tests/integration.rs`

**Step 1: Create the test fixture**

Create `crates/kermlc/tests/fixtures/valid/inline_conjugation.kerml`:

```kerml
package InlineConj {
    type Source {
        in feature input : Source;
        out feature output : Source;
        inout feature control : Source;
        feature data : Source;
    }

    type Wrapper {
        feature port : ~Source;
    }
}
```

**Step 2: Write the integration test**

Add to `crates/kermlc/tests/integration.rs`:

```rust
#[test]
fn valid_inline_conjugation() {
    let result = compile_file(
        &fixtures_dir().join("valid/inline_conjugation.kerml"),
    );
    assert!(
        !result.sink.has_errors(),
        "Errors in inline_conjugation.kerml: {:?}",
        result.sink.diagnostics()
    );

    let pkg = result.model.roots[0];
    let children = &result.model.defs[pkg].children;

    // Find Wrapper type
    let wrapper_id = children
        .iter()
        .find(|&&c| {
            result.interner.resolve(result.model.defs[c].name) == "Wrapper"
        })
        .copied()
        .expect("Wrapper not found");

    // Find feature port inside Wrapper
    let port_id = result.model.defs[wrapper_id]
        .children
        .iter()
        .find(|&&c| {
            result.interner.resolve(result.model.defs[c].name) == "port"
        })
        .copied()
        .expect("port feature not found");

    let port = &result.model.defs[port_id];
    assert!(port.type_ref.is_some(), "port should have type_ref");
    let type_ref = port.type_ref.as_ref().unwrap();
    assert!(type_ref.is_resolved(), "port type_ref should be resolved");

    // The anonymous type should have conjugation-flipped features
    let anon_id = type_ref.resolved_def().unwrap();
    let anon = &result.model.defs[anon_id];
    assert_eq!(anon.kind, kermlc_hir::DefKind::Type);
    assert!(
        result.interner.resolve(anon.name).starts_with('~'),
        "anonymous type name should start with ~"
    );

    // Anonymous type should have inherited features with flipped directions
    assert_eq!(
        anon.inherited_features.len(),
        4,
        "~Source should inherit 4 features"
    );

    for inh in &anon.inherited_features {
        assert_eq!(inh.kind, InheritanceKind::Conjugation);
        let feat_name =
            result.interner.resolve(result.model.defs[inh.def_id].name);
        match feat_name {
            "input" => assert_eq!(
                inh.direction_override,
                Some(FeatureDirection::Out),
                "in should flip to out"
            ),
            "output" => assert_eq!(
                inh.direction_override,
                Some(FeatureDirection::In),
                "out should flip to in"
            ),
            "control" => assert_eq!(
                inh.direction_override,
                Some(FeatureDirection::InOut),
                "inout stays inout"
            ),
            "data" => assert_eq!(
                inh.direction_override,
                None,
                "no direction stays None"
            ),
            other => panic!("unexpected feature: {other}"),
        }
    }
}
```

**Step 3: Run the integration test**

Run: `cargo test -p kermlc -- valid_inline_conjugation --nocapture 2>&1 | tail -20`
Expected: PASS

**Step 4: Run the full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: All tests pass (no regressions).

**Step 5: Commit**

```bash
git add crates/kermlc/tests/fixtures/valid/inline_conjugation.kerml crates/kermlc/tests/integration.rs
git commit -m "feat: inline conjugated type refs (A1c)"
```

---

## Task 6: Update progress tracker

**Files:**
- Modify: `docs/plans/progress.md`

**Step 1: Mark A1c as complete**

Change line 34 from:
```
- [ ] A1c: Inline conjugated type refs — `feature port : ~FuelPort;` (anonymous type synthesis)
```
to:
```
- [x] A1c: Inline conjugated type refs — `feature port : ~FuelPort;` (anonymous type synthesis)
```

Update the "Last updated" line accordingly.

**Step 2: Commit**

```bash
git add docs/plans/progress.md
git commit -m "docs: mark A1c complete in progress tracker"
```
