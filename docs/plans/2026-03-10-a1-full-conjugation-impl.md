# A1: Full Conjugation — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement type-level conjugation with `in`/`out`/`inout` feature direction parsing and direction flipping when inheriting through conjugation.

**Architecture:** Two vertical slices. Slice 1 adds feature direction support (lex → parse → AST → HIR → serialize). Slice 2 adds conjugation direction flipping with inheritance tagging (typeck → validate → serialize). Both slices follow the existing arena-based pipeline.

**Tech Stack:** Rust, cargo test, existing kermlc workspace crates.

---

## Slice 1: Feature Directions

### Task 1: Add `FeatureDirection` enum to HIR

**Files:**
- Modify: `crates/kermlc_hir/src/types.rs:11-16` (near existing enums)
- Modify: `crates/kermlc_hir/src/types.rs:72-94` (Def struct)

**Step 1: Add FeatureDirection enum and direction field to Def**

Add after the `Bound` enum (line 69):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureDirection {
    In,
    Out,
    InOut,
}
```

Add field to `Def` struct after `multiplicity` (line 88):

```rust
pub direction: Option<FeatureDirection>,
```

Initialize it in `Def::new()` (around line 98):

```rust
direction: None,
```

**Step 2: Run `cargo check -p kermlc_hir`**

Expected: PASS (no consumers use the new field yet).

**Step 3: Commit**

```bash
git add crates/kermlc_hir/src/types.rs
git commit -m "feat(hir): add FeatureDirection enum and direction field to Def"
```

---

### Task 2: Add direction to AST FeatureDecl

**Files:**
- Modify: `crates/kermlc_ast/src/nodes.rs:67-74` (FeatureDecl struct)

**Step 1: Add direction field to FeatureDecl**

Add after `name` field (line 69) in FeatureDecl:

```rust
pub direction: Option<kermlc_hir::FeatureDirection>,
```

Note: Reuse `FeatureDirection` from HIR to avoid duplication. Add `kermlc_hir` as a dependency of `kermlc_ast` in Cargo.toml OR define the enum in a shared location. Since both crates are in the workspace, check if `kermlc_ast` already depends on `kermlc_hir`. If circular dependency would result, define `FeatureDirection` in `kermlc_ast` and re-export from `kermlc_hir`.

**Alternative (preferred):** Define `FeatureDirection` in `kermlc_ast/src/nodes.rs` and import it in `kermlc_hir`. This avoids circular deps since `kermlc_hir` already depends on `kermlc_ast`.

Move the enum to `kermlc_ast/src/nodes.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureDirection {
    In,
    Out,
    InOut,
}
```

Add to `FeatureDecl`:

```rust
pub struct FeatureDecl {
    pub name: SymbolId,
    pub direction: Option<FeatureDirection>,
    pub span: Span,
    pub type_ref: Option<QualifiedName>,
    pub chain: Option<FeatureChain>,
    pub multiplicity: Option<Multiplicity>,
}
```

In `kermlc_hir/src/types.rs`, import and re-export:

```rust
pub use kermlc_ast::FeatureDirection;
```

And change the `Def` field to use it:

```rust
pub direction: Option<FeatureDirection>,
```

**Step 2: Run `cargo check -p kermlc_ast -p kermlc_hir`**

Expected: PASS.

**Step 3: Commit**

```bash
git add crates/kermlc_ast/src/nodes.rs crates/kermlc_hir/src/types.rs
git commit -m "feat(ast): add FeatureDirection enum and direction field to FeatureDecl"
```

---

### Task 3: Add `In`, `Out`, `InOut` tokens to lexer

**Files:**
- Modify: `crates/kermlc_lexer/src/token.rs:6-12` (keyword variants)
- Modify: `crates/kermlc_lexer/src/lexer.rs` (keyword matching)
- Test: `crates/kermlc_lexer/src/lexer.rs` (or `tests/` if separate)

**Step 1: Write failing test**

Add test in lexer test module:

```rust
#[test]
fn lex_direction_keywords() {
    let source = "in out inout";
    let tokens = lex_all(source);
    assert_eq!(tokens[0].kind, TokenKind::In);
    assert_eq!(tokens[1].kind, TokenKind::Out);
    assert_eq!(tokens[2].kind, TokenKind::InOut);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_lexer -- lex_direction_keywords`
Expected: FAIL — `TokenKind::In` does not exist.

**Step 3: Add token variants**

In `token.rs`, add after `Chains` (line 12):

```rust
In,
Out,
InOut,
```

In `lexer.rs`, in the keyword matching section (where identifiers are classified), add:

```rust
"in" => TokenKind::In,
"out" => TokenKind::Out,
"inout" => TokenKind::InOut,
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p kermlc_lexer -- lex_direction_keywords`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/kermlc_lexer/
git commit -m "feat(lexer): add In, Out, InOut keyword tokens"
```

---

### Task 4: Parse direction modifiers on features

**Files:**
- Modify: `crates/kermlc_parser/src/parser.rs:382` (`parse_feature_decl`)
- Test: parser tests

**Step 1: Write failing test**

Add parser test:

```rust
#[test]
fn parse_in_feature() {
    let source = "package P { type T { in feature f : T; } }";
    let result = parse(source);
    // Navigate to the feature and check direction
    let feature = get_first_feature(&result);
    assert_eq!(feature.direction, Some(FeatureDirection::In));
}

#[test]
fn parse_out_feature() {
    let source = "package P { type T { out feature g : T; } }";
    let result = parse(source);
    let feature = get_first_feature(&result);
    assert_eq!(feature.direction, Some(FeatureDirection::Out));
}

#[test]
fn parse_inout_feature() {
    let source = "package P { type T { inout feature h : T; } }";
    let result = parse(source);
    let feature = get_first_feature(&result);
    assert_eq!(feature.direction, Some(FeatureDirection::InOut));
}

#[test]
fn parse_undirected_feature() {
    let source = "package P { type T { feature x : T; } }";
    let result = parse(source);
    let feature = get_first_feature(&result);
    assert_eq!(feature.direction, None);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p kermlc_parser -- parse_in_feature parse_out_feature parse_inout_feature parse_undirected_feature`
Expected: FAIL.

**Step 3: Implement direction parsing**

In `parse_feature_decl()` (line 382), before consuming the `feature` keyword, check for direction:

```rust
fn parse_feature_decl(&mut self) -> Option<FeatureDeclId> {
    let start = self.current_span();

    // Parse optional direction
    let direction = match self.peek() {
        TokenKind::In => {
            self.bump();
            Some(FeatureDirection::In)
        }
        TokenKind::Out => {
            self.bump();
            Some(FeatureDirection::Out)
        }
        TokenKind::InOut => {
            self.bump();
            Some(FeatureDirection::InOut)
        }
        _ => None,
    };

    // Expect 'feature' keyword
    self.expect(TokenKind::Feature)?;

    // ... rest of existing parsing ...

    let id = self.features.alloc(FeatureDecl {
        name,
        direction,
        span: /* ... */,
        type_ref,
        chain,
        multiplicity,
    });
    Some(id)
}
```

Also update the caller that decides whether to parse a feature — currently it checks `self.at(TokenKind::Feature)`. Now it also needs to check for `In`, `Out`, `InOut` followed by `Feature`. Look at where `parse_feature_decl` is called (likely in `parse_package` body or type body parsing around lines 195-265) and update the condition:

```rust
// Where members are parsed, add direction tokens as feature-start indicators
TokenKind::Feature | TokenKind::In | TokenKind::Out | TokenKind::InOut => {
    if let Some(id) = self.parse_feature_decl() {
        members.push(Member::Feature(id));
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p kermlc_parser -- parse_in_feature parse_out_feature parse_inout_feature parse_undirected_feature`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/kermlc_parser/
git commit -m "feat(parser): parse in/out/inout direction modifiers on features"
```

---

### Task 5: Lower direction from AST to HIR

**Files:**
- Modify: `crates/kermlc_hir/src/lower.rs:100` (`lower_feature`)

**Step 1: Write failing test**

Add test in lowering tests:

```rust
#[test]
fn lower_feature_direction() {
    let source = "package P { type T { in feature f : T; } }";
    let model = lower(source);
    let f = find_def_by_name(&model, "f");
    assert_eq!(model.defs[f].direction, Some(FeatureDirection::In));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_hir -- lower_feature_direction`
Expected: FAIL — direction is always None.

**Step 3: Add direction lowering**

In `lower_feature()` (around line 100), after creating the Def, copy the direction:

```rust
fn lower_feature(&mut self, id: FeatureDeclId) -> DefId {
    let feat = &self.parse.features[id];
    let mut def = Def::new(feat.name, DefKind::Feature, feat.span);
    def.direction = feat.direction;
    // ... rest of existing lowering ...
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p kermlc_hir -- lower_feature_direction`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/kermlc_hir/src/lower.rs
git commit -m "feat(hir): lower feature direction from AST to HIR"
```

---

### Task 6: Serialize feature direction to JSON-LD

**Files:**
- Modify: `crates/kermlc_serial_json/src/serialize.rs:91-101` (feature serialization area)

**Step 1: Write failing test**

Add serialization test:

```rust
#[test]
fn serialize_feature_direction() {
    let source = "package P { type T { in feature f : T; out feature g : T; } }";
    let json = compile_to_json(source);
    let elements: Vec<Value> = serde_json::from_str(&json).unwrap();
    let f = find_element(&elements, "f");
    assert_eq!(f["direction"], "in");
    let g = find_element(&elements, "g");
    assert_eq!(g["direction"], "out");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_serial_json -- serialize_feature_direction`
Expected: FAIL — no `direction` field in output.

**Step 3: Add direction serialization**

In `build_elements()`, in the feature section (around lines 91-101), add direction output:

```rust
// Add direction for features
if def.kind == DefKind::Feature {
    if let Some(dir) = &def.direction {
        element["direction"] = json!(match dir {
            FeatureDirection::In => "in",
            FeatureDirection::Out => "out",
            FeatureDirection::InOut => "inout",
        });
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p kermlc_serial_json -- serialize_feature_direction`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/kermlc_serial_json/
git commit -m "feat(serial): serialize feature direction to JSON-LD"
```

---

### Task 7: Integration test for feature directions

**Files:**
- Create: `crates/kermlc/tests/fixtures/valid/direction.kerml`
- Modify: `crates/kermlc/tests/integration.rs` (add test case)

**Step 1: Create fixture file**

```kerml
package Directions {
    type Sensor {
        in feature reading : Sensor;
        out feature signal : Sensor;
        inout feature control : Sensor;
        feature data : Sensor;
    }
}
```

**Step 2: Write integration test**

```rust
#[test]
fn direction_fixture() {
    let result = compile_fixture("valid/direction.kerml");
    assert!(result.diagnostics.is_empty(), "unexpected errors: {:?}", result.diagnostics);
}
```

**Step 3: Run test**

Run: `cargo test -p kermlc -- direction_fixture`
Expected: PASS.

**Step 4: Commit**

```bash
git add crates/kermlc/tests/
git commit -m "test: integration test for feature direction parsing"
```

---

## Slice 2: Conjugation Direction Flipping

### Task 8: Add `InheritanceKind` and `InheritedFeature` to HIR

**Files:**
- Modify: `crates/kermlc_hir/src/types.rs` (add types, change `inherited_features` field)

**Step 1: Add types**

Add after `FeatureDirection` (or its re-export):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InheritanceKind {
    Specialization,
    Conjugation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritedFeature {
    pub def_id: DefId,
    pub kind: InheritanceKind,
    pub direction_override: Option<FeatureDirection>,
}
```

**Step 2: Change `Def.inherited_features` type**

Change line 92 from:

```rust
pub inherited_features: Vec<DefId>,
```

to:

```rust
pub inherited_features: Vec<InheritedFeature>,
```

**Step 3: Fix all compilation errors**

This will break `kermlc_typeck`, `kermlc_validate`, `kermlc_serial_json`, and `kermlc_resolve` wherever `inherited_features` is accessed. Update each site:

- In **typeck** (`check.rs`): where features are pushed to `inherited_features`, wrap in `InheritedFeature { def_id, kind: InheritanceKind::Specialization, direction_override: None }`
- In **validate** (`validate.rs`): where `inherited_features` is iterated, access `.def_id`
- In **serial** (`serialize.rs`): where `inherited_features` is iterated, access `.def_id`
- In **resolve** (`resolve.rs`): if `inherited_features` is accessed, update similarly

**Step 4: Run `cargo check --workspace`**

Expected: PASS (all sites updated).

**Step 5: Run `cargo test --workspace`**

Expected: All existing tests PASS (behavior unchanged, just wrapped in structs).

**Step 6: Commit**

```bash
git add crates/
git commit -m "refactor(hir): replace Vec<DefId> with Vec<InheritedFeature> for inheritance tagging"
```

---

### Task 9: Add `conjugate_direction` function

**Files:**
- Modify: `crates/kermlc_hir/src/types.rs` (add function)

**Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conjugate_in_becomes_out() {
        assert_eq!(
            conjugate_direction(Some(FeatureDirection::In)),
            Some(FeatureDirection::Out),
        );
    }

    #[test]
    fn conjugate_out_becomes_in() {
        assert_eq!(
            conjugate_direction(Some(FeatureDirection::Out)),
            Some(FeatureDirection::In),
        );
    }

    #[test]
    fn conjugate_inout_stays_inout() {
        assert_eq!(
            conjugate_direction(Some(FeatureDirection::InOut)),
            Some(FeatureDirection::InOut),
        );
    }

    #[test]
    fn conjugate_none_stays_none() {
        assert_eq!(conjugate_direction(None), None);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p kermlc_hir -- conjugate_`
Expected: FAIL — function does not exist.

**Step 3: Implement**

```rust
pub fn conjugate_direction(
    dir: Option<FeatureDirection>,
) -> Option<FeatureDirection> {
    match dir {
        Some(FeatureDirection::In) => Some(FeatureDirection::Out),
        Some(FeatureDirection::Out) => Some(FeatureDirection::In),
        Some(FeatureDirection::InOut) => Some(FeatureDirection::InOut),
        None => None,
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p kermlc_hir -- conjugate_`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/kermlc_hir/src/types.rs
git commit -m "feat(hir): add conjugate_direction function"
```

---

### Task 10: Implement direction flipping in type checking

**Files:**
- Modify: `crates/kermlc_typeck/src/check.rs:92-116` (conjugation section)

**Step 1: Write failing test**

```rust
#[test]
fn conjugation_flips_directions() {
    let source = r#"
        package P {
            type A {
                in feature f : A;
                out feature g : A;
                inout feature h : A;
                feature x : A;
            }
            type B ~ A {}
        }
    "#;
    let model = compile_and_check(source);
    let b = find_def_by_name(&model, "B");
    let inherited = &model.defs[b].inherited_features;

    let f_inherited = inherited.iter().find(|i| {
        model.defs[i.def_id].name == intern("f")
    }).unwrap();
    assert_eq!(f_inherited.kind, InheritanceKind::Conjugation);
    assert_eq!(f_inherited.direction_override, Some(FeatureDirection::Out));

    let g_inherited = inherited.iter().find(|i| {
        model.defs[i.def_id].name == intern("g")
    }).unwrap();
    assert_eq!(g_inherited.direction_override, Some(FeatureDirection::In));

    let h_inherited = inherited.iter().find(|i| {
        model.defs[i.def_id].name == intern("h")
    }).unwrap();
    assert_eq!(h_inherited.direction_override, Some(FeatureDirection::InOut));

    let x_inherited = inherited.iter().find(|i| {
        model.defs[i.def_id].name == intern("x")
    }).unwrap();
    assert_eq!(x_inherited.direction_override, None);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_typeck -- conjugation_flips_directions`
Expected: FAIL — direction_override is None, kind is Specialization.

**Step 3: Update conjugation handling in `check_type`**

Replace the conjugation section (lines 92-116) with:

```rust
// Handle conjugation: flip feature directions
if let Some(conj) = &model.defs[def_id].conjugation.clone() {
    if let Some(conj_target) = conj.resolved_def() {
        // Collect direct features of conjugated type
        let direct_features: Vec<DefId> = model.defs[conj_target]
            .children
            .iter()
            .filter(|c| model.defs[**c].kind == DefKind::Feature)
            .copied()
            .collect();

        // Collect inherited features of conjugated type
        let inherited_features: Vec<InheritedFeature> = model.defs[conj_target]
            .inherited_features
            .clone();

        // Add direct features with flipped directions
        for feat_id in &direct_features {
            let dir = model.defs[*feat_id].direction;
            model.defs[def_id].inherited_features.push(InheritedFeature {
                def_id: *feat_id,
                kind: InheritanceKind::Conjugation,
                direction_override: conjugate_direction(dir),
            });
        }

        // Add inherited features with flipped effective directions
        for inherited in &inherited_features {
            let effective_dir = inherited.direction_override
                .or(model.defs[inherited.def_id].direction);
            model.defs[def_id].inherited_features.push(InheritedFeature {
                def_id: inherited.def_id,
                kind: InheritanceKind::Conjugation,
                direction_override: conjugate_direction(effective_dir),
            });
        }

        // Populate TypeInfo::conjugate_of
        if let Some(type_id) = model.def_to_type
            .get(def_id.raw() as usize)
            .and_then(|t| *t)
        {
            model.type_infos[type_id].conjugate_of = Some(conj_target);
        }

        changed = true;
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p kermlc_typeck -- conjugation_flips_directions`
Expected: PASS.

**Step 5: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests PASS (existing conjugation test may need updating if it checks inherited_features structure).

**Step 6: Commit**

```bash
git add crates/kermlc_typeck/
git commit -m "feat(typeck): flip feature directions during conjugation inheritance"
```

---

### Task 11: Update serialization for direction overrides

**Files:**
- Modify: `crates/kermlc_serial_json/src/serialize.rs`

**Step 1: Write failing test**

```rust
#[test]
fn serialize_conjugation_flipped_directions() {
    let source = r#"
        package P {
            type A { in feature f : A; }
            type B ~ A {}
        }
    "#;
    let json = compile_to_json(source);
    let elements: Vec<Value> = serde_json::from_str(&json).unwrap();
    let b = find_element(&elements, "B");
    // B's inherited feature f should show direction "out" (flipped)
    let inherited = &b["inheritedFeature"];
    assert!(inherited.is_array());
    let f_ref = &inherited[0];
    assert_eq!(f_ref["direction"], "out");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_serial_json -- serialize_conjugation_flipped`
Expected: FAIL.

**Step 3: Add inherited feature serialization with direction overrides**

In `build_elements()`, add inherited feature output for types:

```rust
// Add inherited features
if !def.inherited_features.is_empty() {
    let inherited_refs: Vec<Value> = def.inherited_features.iter().map(|inh| {
        let mut obj = json!({
            "@id": format!("feature-{}", inh.def_id.raw()),
            "inheritanceKind": match inh.kind {
                InheritanceKind::Specialization => "specialization",
                InheritanceKind::Conjugation => "conjugation",
            },
        });
        let effective_dir = inh.direction_override
            .or(model.defs[inh.def_id].direction);
        if let Some(dir) = effective_dir {
            obj["direction"] = json!(match dir {
                FeatureDirection::In => "in",
                FeatureDirection::Out => "out",
                FeatureDirection::InOut => "inout",
            });
        }
        obj
    }).collect();
    element["inheritedFeature"] = json!(inherited_refs);
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p kermlc_serial_json -- serialize_conjugation_flipped`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/kermlc_serial_json/
git commit -m "feat(serial): serialize inherited features with direction overrides"
```

---

### Task 12: Add validation for direction consistency

**Files:**
- Modify: `crates/kermlc_validate/src/validate.rs`

**Step 1: Write failing test**

```rust
#[test]
fn validate_conjugation_target_no_features_warning() {
    let source = r#"
        package P {
            type Empty {}
            type B ~ Empty {}
        }
    "#;
    let diagnostics = compile_and_validate(source);
    assert!(diagnostics.iter().any(|d| {
        d.severity == Severity::Warning
            && d.message.contains("no features")
    }));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p kermlc_validate -- validate_conjugation_target_no_features`
Expected: FAIL — no such warning emitted.

**Step 3: Add validation**

In `validate_type()`, after the existing conjugation target check (lines 58-73), add:

```rust
// Warn if conjugation target has no features
if let Some(conj) = &def.conjugation {
    if let Some(target_id) = conj.resolved_def() {
        let target = &model.defs[target_id];
        let has_features = target.children.iter().any(|c| {
            model.defs[*c].kind == DefKind::Feature
        });
        if !has_features {
            let name = interner.resolve(target.name);
            sink.emit(
                Diagnostic::warning(format!(
                    "conjugation target `{name}` has no features; \
                     conjugation has no effect"
                ))
                .with_label(Label::primary(
                    conj.span,
                    "this type has no features to flip",
                )),
            );
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p kermlc_validate -- validate_conjugation_target_no_features`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/kermlc_validate/
git commit -m "feat(validate): warn when conjugation target has no features"
```

---

### Task 13: Update conjugation integration fixture

**Files:**
- Modify: `crates/kermlc/tests/fixtures/valid/conjugation.kerml`
- Modify: `crates/kermlc/tests/integration.rs`

**Step 1: Update fixture with directions**

```kerml
package Connectors {
    type Source {
        in feature input : Source;
        out feature output : Source;
        inout feature control : Source;
        feature data : Source;
    }
    type Sink ~ Source {}
}
```

**Step 2: Update or add integration test**

```rust
#[test]
fn conjugation_fixture() {
    let result = compile_fixture("valid/conjugation.kerml");
    assert!(result.diagnostics.is_empty(), "unexpected errors: {:?}", result.diagnostics);

    // Verify Sink inherits features with flipped directions
    let sink = find_def_by_name(&result.model, "Sink");
    let inherited = &result.model.defs[sink].inherited_features;
    assert_eq!(inherited.len(), 4);

    // in -> out
    let input_inh = find_inherited(&result.model, inherited, "input");
    assert_eq!(input_inh.kind, InheritanceKind::Conjugation);
    assert_eq!(input_inh.direction_override, Some(FeatureDirection::Out));

    // out -> in
    let output_inh = find_inherited(&result.model, inherited, "output");
    assert_eq!(output_inh.direction_override, Some(FeatureDirection::In));

    // inout -> inout
    let control_inh = find_inherited(&result.model, inherited, "control");
    assert_eq!(control_inh.direction_override, Some(FeatureDirection::InOut));

    // none -> none
    let data_inh = find_inherited(&result.model, inherited, "data");
    assert_eq!(data_inh.direction_override, None);
}
```

**Step 3: Run test**

Run: `cargo test -p kermlc -- conjugation_fixture`
Expected: PASS.

**Step 4: Commit**

```bash
git add crates/kermlc/tests/
git commit -m "test: update conjugation fixture with direction flipping verification"
```

---

### Task 14: Chained conjugation test

**Files:**
- Create: `crates/kermlc/tests/fixtures/valid/conjugation_chained.kerml`
- Modify: `crates/kermlc/tests/integration.rs`

**Step 1: Create fixture**

```kerml
package ChainedConjugation {
    type A {
        in feature f : A;
        out feature g : A;
    }
    type B ~ A {}
    type C ~ B {}
}
```

**Step 2: Write test**

```rust
#[test]
fn chained_conjugation_double_flip() {
    let result = compile_fixture("valid/conjugation_chained.kerml");
    assert!(result.diagnostics.is_empty());

    // B ~ A: in->out, out->in
    let b = find_def_by_name(&result.model, "B");
    let b_f = find_inherited(&result.model, &result.model.defs[b].inherited_features, "f");
    assert_eq!(b_f.direction_override, Some(FeatureDirection::Out));

    // C ~ B: out->in (double flip = back to original)
    let c = find_def_by_name(&result.model, "C");
    let c_f = find_inherited(&result.model, &result.model.defs[c].inherited_features, "f");
    assert_eq!(c_f.direction_override, Some(FeatureDirection::In));
}
```

**Step 3: Run test**

Run: `cargo test -p kermlc -- chained_conjugation`
Expected: PASS.

**Step 4: Commit**

```bash
git add crates/kermlc/tests/
git commit -m "test: chained conjugation double-flip verification"
```

---

### Task 15: Run full suite + clippy + final commit

**Step 1: Run full test suite**

Run: `cargo test --workspace`
Expected: All PASS.

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: No warnings.

**Step 3: Run fmt**

Run: `cargo fmt --check`
Expected: No formatting issues.

**Step 4: Final commit if any fixups needed**

```bash
git add -A
git commit -m "chore: clippy and fmt fixes for A1 conjugation"
```
