# A3: Feature Chaining Resolution — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement type-directed chain walking so `feature x chains a.b.c;`
resolves each segment as a member of the previous segment's type.

**Architecture:** Add `find_member()` to `scope.rs` for strict member lookup
(children + inherited features), rewrite `resolve_chains_for()` with eager
walk, add `chain_result` field to `Def` for downstream use.

**Tech Stack:** Rust, kermlc workspace (kermlc_hir, kermlc_resolve, kermlc_typeck,
kermlc_serial_json, kermlc integration tests)

---

### Task 1: Add `chain_result` field to `Def`

**Files:**
- Modify: `crates/kermlc_hir/src/types.rs:125-126` (add field after chain_segments)
- Modify: `crates/kermlc_hir/src/types.rs:152` (add default in Def::new)

**Step 1: Add the field to `Def` struct**

In `crates/kermlc_hir/src/types.rs`, after line 125 (`chain_segments`), add:

```rust
    /// For features with chains: the final resolved def of the chain
    pub chain_result: Option<DefId>,
```

**Step 2: Add default initialization in `Def::new()`**

In `crates/kermlc_hir/src/types.rs`, after line 152 (`chain_segments: Vec::new(),`), add:

```rust
            chain_result: None,
```

**Step 3: Verify it compiles**

Run: `cargo build 2>&1 | head -20`
Expected: compiles without errors

**Step 4: Commit**

```bash
git add crates/kermlc_hir/src/types.rs
git commit -m "feat(hir): add chain_result field to Def for chain walking"
```

---

### Task 2: Add `find_member()` to `scope.rs`

**Files:**
- Modify: `crates/kermlc_resolve/src/scope.rs` (add function at end of file)
- Create: unit test in `crates/kermlc_resolve/src/scope.rs` (inline #[cfg(test)])

**Step 1: Write failing test**

Add at the end of `crates/kermlc_resolve/src/scope.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kermlc_diagnostics::{DiagnosticSink, SourceMap};
    use kermlc_hir::lower_ast;
    use kermlc_intern::StringInterner;
    use kermlc_parser::Parser;

    fn parse_and_lower(
        input: &str,
    ) -> (SemanticModel, StringInterner, DiagnosticSink) {
        let mut interner = StringInterner::new();
        let mut source_map = SourceMap::new();
        let mut sink = DiagnosticSink::new();
        let file_id =
            source_map.add_file("test.kerml".into(), input.into());
        let parse =
            Parser::parse(input, file_id, &mut interner, &mut sink);
        let model = lower_ast(&parse, &mut interner, &mut sink);
        (model, interner, sink)
    }

    #[test]
    fn find_member_direct_child() {
        let (mut model, interner, mut sink) = parse_and_lower(
            "package P { type T { feature f : T; } }",
        );
        // Resolve so type_ref is set
        crate::resolve_pass(&mut model, &interner, &mut sink);

        let pkg = model.roots[0];
        let t_id = model.defs[pkg].children[0];
        let f_id = model.defs[t_id].children[0];
        let f_name = model.defs[f_id].name;

        let found = find_member(&model, t_id, f_name);
        assert_eq!(found, Some(f_id));
    }

    #[test]
    fn find_member_not_found() {
        let (model, interner, _sink) = parse_and_lower(
            "package P { type T { feature f : T; } }",
        );

        let pkg = model.roots[0];
        let t_id = model.defs[pkg].children[0];
        // Use a name that doesn't exist
        let bad_name = interner.intern("nonexistent");

        let found = find_member(&model, t_id, bad_name);
        assert_eq!(found, None);
    }

    #[test]
    fn find_member_inherited_feature() {
        let (mut model, interner, mut sink) = parse_and_lower(
            "package P { type A { feature x : A; } type B :> A {} }",
        );
        // Run resolve + typecheck to populate inherited features
        for _ in 0..10 {
            let r = crate::resolve_pass(
                &mut model, &interner, &mut sink,
            );
            let t = kermlc_typeck::typecheck_pass(
                &mut model, &interner, &mut sink,
            );
            if !r && !t {
                break;
            }
        }

        let pkg = model.roots[0];
        let a_id = model.defs[pkg].children[0];
        let x_id = model.defs[a_id].children[0];
        let x_name = model.defs[x_id].name;
        let b_id = model.defs[pkg].children[1];

        // x should be found as inherited member of B
        let found = find_member(&model, b_id, x_name);
        assert_eq!(found, Some(x_id));
    }

    #[test]
    fn find_member_no_parent_walking() {
        let (model, interner, _sink) = parse_and_lower(
            "package P { type T {} feature outside : T; }",
        );

        let pkg = model.roots[0];
        let t_id = model.defs[pkg].children[0];
        let outside_name = interner.intern("outside");

        // find_member should NOT walk up to parent (package) scope
        let found = find_member(&model, t_id, outside_name);
        assert_eq!(found, None);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p kermlc_resolve -- scope::tests 2>&1 | tail -20`
Expected: FAIL — `find_member` not found

**Step 3: Implement `find_member`**

Add before the `#[cfg(test)]` block in `crates/kermlc_resolve/src/scope.rs`:

```rust
/// Find a member of a type by name.
/// Searches direct children, then inherited features.
/// No parent walking, no imports — strict member lookup only.
/// Used for type-directed chain resolution (A3).
pub fn find_member(
    model: &SemanticModel,
    type_def_id: DefId,
    name: SymbolId,
) -> Option<DefId> {
    // 1. Direct children
    if let Some(found) = model.find_child(type_def_id, name) {
        return Some(found);
    }

    // 2. Inherited features (populated by typeck)
    for inherited in &model.defs[type_def_id].inherited_features {
        if model.defs[inherited.def_id].name == name {
            return Some(inherited.def_id);
        }
    }

    None
}
```

**Step 4: Add `kermlc_typeck` to dev-dependencies**

In `crates/kermlc_resolve/Cargo.toml`, add under `[dev-dependencies]`:

```toml
kermlc_typeck = { workspace = true }
```

(Needed for the `find_member_inherited_feature` test that runs typeck.)

**Step 5: Run tests to verify they pass**

Run: `cargo test -p kermlc_resolve -- scope::tests -v 2>&1 | tail -20`
Expected: 4 tests PASS

**Step 6: Commit**

```bash
git add crates/kermlc_resolve/src/scope.rs crates/kermlc_resolve/Cargo.toml
git commit -m "feat(resolve): add find_member for strict member lookup"
```

---

### Task 3: Rewrite `resolve_chains_for` with eager walk

**Files:**
- Modify: `crates/kermlc_resolve/src/resolve.rs:295-302` (replace resolve_chains_for)

**Step 1: Write failing test in `resolve.rs`**

Add to existing `mod tests` block in `crates/kermlc_resolve/src/resolve.rs`:

```rust
    #[test]
    fn resolve_chain_type_directed() {
        let src = "\
            package P {\
                type Engine { feature cylinders : Engine; }\
                type Vehicle { feature engine : Engine; }\
                type Fleet {\
                    feature vehicles : Vehicle;\
                    feature v_eng chains vehicles.engine;\
                }\
            }";
        let (mut model, interner, mut sink) = parse_and_lower(src);
        let stdlib = kermlc_hir::load_stdlib(&mut model, &mut interner);
        kermlc_hir::add_implicit_specializations(&mut model, &stdlib);

        for _ in 0..100 {
            let r = resolve_pass(&mut model, &interner, &mut sink);
            let t = kermlc_typeck::typecheck_pass(
                &mut model, &interner, &mut sink,
            );
            if !r && !t {
                break;
            }
        }
        emit_unresolved_errors(&model, &interner, &mut sink);

        assert!(
            !sink.has_errors(),
            "chain should resolve: {:?}",
            sink.diagnostics()
        );

        // Find v_eng and verify chain_result is set
        let pkg = model.roots[0];
        let fleet = model.defs[pkg].children[2]; // Fleet
        let v_eng = model.defs[fleet].children[1]; // v_eng
        assert!(
            model.defs[v_eng].chain_result.is_some(),
            "chain_result should be set after resolution"
        );
    }

    #[test]
    fn resolve_chain_three_steps() {
        let src = "\
            package P {\
                type C {}\
                type B { feature c : C; }\
                type A { feature b : B; }\
                type Root {\
                    feature a : A;\
                    feature abc chains a.b.c;\
                }\
            }";
        let (mut model, interner, mut sink) = parse_and_lower(src);
        let stdlib = kermlc_hir::load_stdlib(&mut model, &mut interner);
        kermlc_hir::add_implicit_specializations(&mut model, &stdlib);

        for _ in 0..100 {
            let r = resolve_pass(&mut model, &interner, &mut sink);
            let t = kermlc_typeck::typecheck_pass(
                &mut model, &interner, &mut sink,
            );
            if !r && !t {
                break;
            }
        }
        emit_unresolved_errors(&model, &interner, &mut sink);

        assert!(
            !sink.has_errors(),
            "3-step chain should resolve: {:?}",
            sink.diagnostics()
        );

        let pkg = model.roots[0];
        let root = model.defs[pkg].children[3]; // Root
        let abc = model.defs[root].children[1]; // abc

        // All 3 segments should be resolved
        assert!(model.defs[abc].chain_segments.iter().all(|s| s.is_resolved()));
        assert!(model.defs[abc].chain_result.is_some());
    }

    #[test]
    fn resolve_chain_unresolved_member() {
        let src = "\
            package P {\
                type A { feature x : A; }\
                type Root {\
                    feature a : A;\
                    feature bad chains a.nonexistent;\
                }\
            }";
        let (mut model, interner, mut sink) = parse_and_lower(src);
        let stdlib = kermlc_hir::load_stdlib(&mut model, &mut interner);
        kermlc_hir::add_implicit_specializations(&mut model, &stdlib);

        for _ in 0..100 {
            let r = resolve_pass(&mut model, &interner, &mut sink);
            let t = kermlc_typeck::typecheck_pass(
                &mut model, &interner, &mut sink,
            );
            if !r && !t {
                break;
            }
        }
        emit_unresolved_errors(&model, &interner, &mut sink);

        assert!(
            sink.has_errors(),
            "unresolved chain member should produce error"
        );
    }
```

**Step 2: Add `kermlc_typeck` to dev-dependencies (if not already done)**

Check `crates/kermlc_resolve/Cargo.toml` `[dev-dependencies]` — add
`kermlc_typeck = { workspace = true }` if missing (should be added in Task 2).

**Step 3: Run tests to verify they fail**

Run: `cargo test -p kermlc_resolve -- resolve_chain 2>&1 | tail -20`
Expected: FAIL — current resolve does scope-based resolution, type-directed
walk is not implemented

**Step 4: Rewrite `resolve_chains_for`**

Replace the function at `crates/kermlc_resolve/src/resolve.rs:295-302` with:

```rust
fn resolve_chains_for(model: &mut SemanticModel, def_id: DefId) -> bool {
    let count = model.defs[def_id].chain_segments.len();
    if count == 0 {
        return false;
    }

    let mut changed = false;

    for i in 0..count {
        if model.defs[def_id].chain_segments[i].resolution
            != ResolutionState::Unresolved
        {
            continue;
        }

        if i == 0 {
            // First segment: scope-based resolution (as before)
            let segments =
                model.defs[def_id].chain_segments[0].segments.clone();
            if let Some(resolved) =
                try_resolve_name(model, def_id, &segments)
            {
                model.defs[def_id].chain_segments[0].resolution =
                    ResolutionState::Resolved(resolved);
                changed = true;
            }
        } else {
            // Subsequent segments: type-directed via find_member
            let prev_resolution =
                model.defs[def_id].chain_segments[i - 1].resolution;
            let prev_def = match prev_resolution {
                ResolutionState::Resolved(id) => id,
                _ => break, // previous not resolved yet, defer
            };

            // Get the type of the previous segment
            let prev_type_ref = match &model.defs[prev_def].type_ref {
                Some(tr) => tr.resolution,
                None => break, // no type ref, defer
            };
            let type_def = match prev_type_ref {
                ResolutionState::Resolved(id) => id,
                _ => break, // type not resolved yet, defer
            };

            // Look up this segment as a member of that type
            let seg_name =
                model.defs[def_id].chain_segments[i].segments[0];
            if let Some(found) =
                crate::scope::find_member(model, type_def, seg_name)
            {
                model.defs[def_id].chain_segments[i].resolution =
                    ResolutionState::Resolved(found);
                changed = true;
            } else if model.defs[type_def].type_checked {
                // Type is fully checked but member not found — error
                model.defs[def_id].chain_segments[i].resolution =
                    ResolutionState::Error;
                changed = true;
                break;
            } else {
                // Type not yet checked — inherited features may appear
                break;
            }
        }
    }

    // If all segments are resolved, set chain_result
    let all_resolved = (0..count).all(|i| {
        matches!(
            model.defs[def_id].chain_segments[i].resolution,
            ResolutionState::Resolved(_)
        )
    });
    if all_resolved && model.defs[def_id].chain_result.is_none() {
        if let ResolutionState::Resolved(last) =
            model.defs[def_id].chain_segments[count - 1].resolution
        {
            model.defs[def_id].chain_result = Some(last);
            changed = true;
        }
    }

    changed
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p kermlc_resolve -- resolve_chain -v 2>&1 | tail -30`
Expected: 3 new tests PASS

**Step 6: Run all resolve tests for regression**

Run: `cargo test -p kermlc_resolve 2>&1 | tail -10`
Expected: all tests PASS

**Step 7: Commit**

```bash
git add crates/kermlc_resolve/src/resolve.rs
git commit -m "feat(resolve): type-directed chain walking with eager walk"
```

---

### Task 4: Update integration test fixture

**Files:**
- Modify: `crates/kermlc/tests/fixtures/valid/feature_chain.kerml`
- Modify: `crates/kermlc/tests/integration.rs` (add chain-specific assertions)

**Step 1: Replace fixture with a real chain test**

Overwrite `crates/kermlc/tests/fixtures/valid/feature_chain.kerml`:

```kerml
package Models {
    type Engine {
        feature cylinders : Engine;
    }
    type Vehicle {
        feature engine : Engine;
    }
    type Fleet {
        feature vehicles : Vehicle;
        feature v_engine chains vehicles.engine;
        feature v_cyl chains vehicles.engine.cylinders;
    }
}
```

**Step 2: Run integration tests**

Run: `cargo test -p kermlc -- valid_feature_chain -v 2>&1`
Expected: PASS (no errors)

**Step 3: Add invalid fixture for chain errors**

Create `crates/kermlc/tests/fixtures/invalid/chain_bad_member.kerml`:

```kerml
package P {
    type A {
        feature x : A;
    }
    type B {
        feature a : A;
        feature bad chains a.nonexistent;
    }
}
```

**Step 4: Add integration test for invalid chain**

Add to `crates/kermlc/tests/integration.rs`:

```rust
#[test]
fn invalid_chain_bad_member() {
    let result = compile_file(
        &fixtures_dir().join("invalid/chain_bad_member.kerml"),
    );
    assert!(
        result.sink.has_errors(),
        "bad chain member should produce error"
    );
}
```

**Step 5: Run all integration tests**

Run: `cargo test -p kermlc 2>&1 | tail -15`
Expected: all PASS

**Step 6: Commit**

```bash
git add crates/kermlc/tests/fixtures/ crates/kermlc/tests/integration.rs
git commit -m "test: add integration tests for feature chain resolution"
```

---

### Task 5: Update serialization for chain_result

**Files:**
- Modify: `crates/kermlc_serial_json/src/serialize.rs:141-174` (add chain output)

**Step 1: Add chain serialization**

In `crates/kermlc_serial_json/src/serialize.rs`, inside the
`if def.kind == DefKind::Feature` block (after the direction section, before
the closing `}`), add:

```rust
            // Add chaining features
            if !def.chain_segments.is_empty() {
                let chaining: Vec<_> = def
                    .chain_segments
                    .iter()
                    .filter_map(|seg| seg.resolved_def())
                    .map(|id| {
                        json!({
                            "@type": "FeatureChaining",
                            "chainingFeature": {
                                "@id": format!("def-{}", id.raw()),
                            },
                        })
                    })
                    .collect();
                if !chaining.is_empty() {
                    element["ownedFeatureChaining"] = json!(chaining);
                }
            }
```

**Step 2: Run integration test for JSON output**

Run: `cargo test -p kermlc -- json_serialization -v 2>&1`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/kermlc_serial_json/src/serialize.rs
git commit -m "feat(serial): serialize feature chaining to JSON-LD"
```

---

### Task 6: Run full test suite and clippy

**Step 1: Run all tests**

Run: `cargo test 2>&1 | tail -20`
Expected: all PASS

**Step 2: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tail -20`
Expected: no warnings

**Step 3: Run fmt check**

Run: `cargo fmt --check 2>&1`
Expected: no formatting issues

**Step 4: Fix any issues found, then commit**

If issues found, fix and commit with:
```bash
git commit -m "style: fix clippy/fmt issues from A3 implementation"
```

---

### Task 7: Update progress tracker

**Files:**
- Modify: `docs/plans/progress.md` (mark A3 complete)

**Step 1: Mark A3 as done**

Change `- [ ] A3: Feature chaining resolution` to
`- [x] A3: Feature chaining resolution — type-directed chain walking (a.b.c)`

Update `Last updated:` line to current date.

**Step 2: Commit**

```bash
git add docs/plans/progress.md
git commit -m "docs: mark A3 feature chaining resolution as complete"
```
