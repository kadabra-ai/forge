# A2: Expression Evaluation — Symbolic Multiplicity Bounds

## Problem

Multiplicity bounds currently eagerly evaluate to concrete `u64` values during
AST→HIR lowering. `eval_const_expr` returns `0` for anything other than
`IntLiteral`, so feature references in bounds like `[1..portCount]` silently
produce wrong values. The information is lost before resolve, validation, or
serialization can use it.

## Scope

KerML spec (Clause 8.2.5.11) restricts multiplicity bounds to:

- `LiteralExpression` — integer, real, boolean, string, infinity (`*`)
- `FeatureReferenceExpression` — reference to a feature by qualified name

No binary operators in multiplicity. BinOp belongs to the general expression
system (derived features, constraints) — future B3 scope.

This design covers: `IntLiteral`, `Star`, and `Name` (feature reference) in
multiplicity bounds.

## Approach: NameRef in MultBound

Use the existing `NameRef` pattern (same as specializations, type_ref,
conjugation). Feature references in multiplicity are stored as unresolved
`NameRef`s during lowering, resolved during the resolve pass, and preserved
symbolically for validation and serialization.

## Design

### 1. HIR Types (`kermlc_hir/src/types.rs`)

Replace `Bound` enum with `MultBound`:

```rust
#[derive(Clone, Debug)]
pub enum MultBound {
    Exact(u64),
    Unbounded,
    Ref(NameRef),
}

#[derive(Clone, Debug)]
pub struct HirMultiplicity {
    pub lower: MultBound,
    pub upper: MultBound,
    pub span: Span,
}
```

Delete old `Bound` enum. Update all references.

### 2. Parser (`kermlc_parser/src/parser.rs`)

Simplify `parse_multiplicity` to delegate to `parse_expr_atom` for all bound
positions instead of explicitly checking token kinds:

```rust
fn parse_multiplicity(&mut self) -> Option<Multiplicity> {
    // ...
    let first = self.parse_expr_atom()?;
    if self.at(TokenKind::DotDot) {
        self.bump();
        lower = Some(first);
        upper = self.parse_expr_atom();
    } else {
        upper = Some(first);
    }
    // ...
}
```

This enables `[name]`, `[name..*]`, `[1..name]`, `[a..b]`, `[Pkg::count]`.

### 3. Lowering (`kermlc_hir/src/lower.rs`)

Replace `eval_const_expr` with `lower_expr_to_bound`:

```rust
fn lower_expr_to_bound(expr: &Expr, interner: &StringInterner) -> MultBound {
    match expr {
        Expr::IntLiteral { value, .. } => MultBound::Exact(*value),
        Expr::Star { .. } => MultBound::Unbounded,
        Expr::Name { name } => {
            // Convert QualifiedName segments to SymbolIds
            MultBound::Ref(NameRef::unresolved(segments, span))
        }
        Expr::BinOp { .. } => MultBound::Exact(0), // unreachable
    }
}
```

Default when lower is missing: `MultBound::Exact(0)`.
Default when upper is missing: clone of lower.

### 4. Resolve (`kermlc_resolve/src/resolve.rs`)

New `resolve_multiplicity_refs_for()` — same pattern as
`resolve_specializations_for()`. Resolves `MultBound::Ref` NameRefs using
`try_resolve_name`.

Called from main `resolve_pass` loop:

```rust
changed |= resolve_multiplicity_refs_for(model, def_id);
```

`emit_unresolved_errors` extended to check multiplicity NameRefs.

### 5. Validation (`kermlc_validate/src/validate.rs`)

Bounds check only when both are `Exact`:

```rust
if let (MultBound::Exact(lower), MultBound::Exact(upper)) = (&mult.lower, &mult.upper) {
    if lower > upper { /* emit error */ }
}
```

Symbolic bounds (`Ref`) — defer to runtime. Redefinition validation same rule.

### 6. JSON-LD Serialization (`kermlc_serial_json/src/serialize.rs`)

Helper `mult_bound_to_json`:

- `Exact(n)` → `n`
- `Unbounded` → `"*"`
- `Ref(name_ref)` → `{ "@type": "FeatureReferenceExpression", "reference": name }`

### 7. Tests

**Parser:** `[n]`, `[n..*]`, `[1..n]`, `[a..b]`, `[Pkg::count]`

**Integration (valid):** `multiplicity_feature_ref.kerml` — feature ref in
multiplicity resolves without errors.

**Integration (invalid):** `multiplicity_unresolved_ref.kerml` — unresolved
feature ref emits diagnostic.

**Lowering:** `Expr::Name` → `MultBound::Ref`, `IntLiteral` → `Exact`, `Star` → `Unbounded`

**Resolve:** feature ref resolves to DefId; missing ref stays Unresolved.

**Validation:** `[5..2]` errors; `[n..2]` defers; `[5..*]` passes.

## Files Changed

| File | Change |
|------|--------|
| `kermlc_hir/src/types.rs` | `MultBound` enum, updated `HirMultiplicity`, delete `Bound` |
| `kermlc_parser/src/parser.rs` | Simplify `parse_multiplicity` to use `parse_expr_atom` |
| `kermlc_hir/src/lower.rs` | `lower_expr_to_bound` replaces `eval_const_expr` |
| `kermlc_resolve/src/resolve.rs` | `resolve_multiplicity_refs_for`, extend `emit_unresolved_errors` |
| `kermlc_validate/src/validate.rs` | Update bounds check for `MultBound` |
| `kermlc_serial_json/src/serialize.rs` | `mult_bound_to_json` helper |
| `kermlc/tests/fixtures/valid/` | `multiplicity_feature_ref.kerml` |
| `kermlc/tests/fixtures/invalid/` | `multiplicity_unresolved_ref.kerml` |
