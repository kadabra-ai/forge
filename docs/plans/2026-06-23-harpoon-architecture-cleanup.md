# harpoon architecture cleanup — deep `compile()`, engine split, rename

Date: 2026-06-23
Status: Ready to implement
Related: ADR-0001, ADR-0002, `CONTEXT.md`

## Goal

Turn the leaked, re-stitched compile orchestration into one deep `harpoon::compile()` on the
`SemanticModel` seam, extract KerML lowering from the engine, and rename the engine crates so the
"one shared engine, two drivers" structure (ADR-0001) is real. Each step is behavior-neutral:
`cargo test` + `cargo clippy --all-targets -- -D warnings` stay green between steps.

## Locked design decisions (from grilling)

1. **Seam on `SemanticModel`.** `compile()` takes an already-lowered model; the front-end runs upstream.
2. **Facade crate `harpoon`** exposes `compile()` and re-exports the engine types/functions drivers need.
3. **Lowering split.** `lower_ast` moves out of the engine into a new front-end crate `kermlc_lower`.
   `harpoon_hir` keeps only the language-agnostic model (`types.rs` + `stdlib.rs`) and *owns*
   `FeatureDirection` + `Visibility` (moved out of `kermlc_ast`).
4. **`harpoon::compile(&mut SemanticModel, &StringInterner, &mut DiagnosticSink)`** runs phases 3–6:
   fixpoint loop → `emit_unresolved_errors` → `detect_specialization_cycles` → `validate`.
   Prelude (`load_stdlib` + `add_implicit_specializations`) stays upstream in the front-end;
   serialization stays downstream in the driver. Success is read via `sink.has_errors()`.
5. **`kermlc/src/lib.rs`** exposes `compile_source(src) -> CompiledModel`
   (parse → `kermlc_lower` → prelude → `harpoon::compile`); `main.rs` becomes a thin CLI shell.
   The resolve↔typeck dev-dep cycle is broken (see Step A).

Consequence: `emit_unresolved_errors` and `detect_specialization_cycles` are called only inside
`harpoon::compile()` now — drivers stop choreographing them (resolves candidate #2's ordering bug
without touching resolve internals).

## Crate layout (target)

```
forge                       build tool / package manager  (= cargo)   [future]
  └─ kermlc / sysmlc        driver binaries (sysmlc future)
       └─ harpoon           engine facade  →  harpoon::compile()
            ├─ harpoon_hir          types.rs + stdlib.rs + FeatureDirection/Visibility
            ├─ harpoon_resolve
            ├─ harpoon_typeck
            ├─ harpoon_validate
            ├─ harpoon_serial_json
            ├─ harpoon_intern       (foundation)
            └─ harpoon_diagnostics  (foundation)
  front-end (KerML, keep kermlc_*):  kermlc_lexer · kermlc_parser · kermlc_ast · kermlc_lower
```

Rename map (Step C): `kermlc_{intern,diagnostics,hir,resolve,typeck,validate,serial_json}` →
`harpoon_*`. Keep `kermlc_{lexer,parser,ast,lower}` and the `kermlc` binary.

## Step 0 — Safety net

- `cargo test` (whole workspace) — record the green baseline and the current test count.
- `cargo clippy --all-targets --all-features -- -D warnings` — clean baseline.

## Step A — Deep `compile()` (candidate #1), no rename yet

Files: new `crates/harpoon/`, `crates/kermlc/src/{lib.rs,main.rs}`,
`crates/kermlc_resolve/Cargo.toml`, `crates/kermlc_resolve/src/resolve.rs` (move tests out),
`crates/kermlc/tests/integration.rs`, workspace `Cargo.toml`.

1. Create `crates/harpoon` facade crate depending on `kermlc_hir`, `kermlc_resolve`,
   `kermlc_typeck`, `kermlc_validate`, `kermlc_serial_json`, `kermlc_intern`, `kermlc_diagnostics`.
2. Implement `harpoon::compile(&mut SemanticModel, &StringInterner, &mut DiagnosticSink)` =
   the 100-iteration fixpoint + `emit_unresolved_errors` + `detect_specialization_cycles` +
   `validate`. Re-export the engine types drivers need (`SemanticModel`, `serialize_to_json`, …).
3. Add `crates/kermlc/src/lib.rs` with `compile_source(src) -> CompiledModel`
   (parse → `lower_ast` → `load_stdlib` → `add_implicit_specializations` → `harpoon::compile`).
   Reduce `main.rs` to CLI parsing + `compile_source` + diagnostics render + (compile) serialize.
   Delete `crates/kermlc/src/pipeline.rs` (folded into `harpoon` + `lib.rs`).
4. Break the dev-dep cycle:
   - Move the ~4–5 fixpoint tests out of `kermlc_resolve` (resolve.rs:~665/705/751/781 and
     scope.rs:163) into `kermlc` integration tests using `compile_source`.
   - Remove `kermlc_typeck` from `kermlc_resolve` `[dev-dependencies]`.
   - `kermlc_resolve` tests that remain operate without `typecheck_pass` (resolution in isolation;
     keep `kermlc_parser`/future `kermlc_lower` dev-dep only).
5. Replace the duplicated test helpers (`compile_to_model`, `compile_and_validate`,
   `compile_and_serialize`) with calls to a shared helper or `compile_source` where a full
   pipeline is needed; keep pass-isolated tests local.
6. QA: build + clippy + test green.

## Step B — Extract `kermlc_lower`

Files: new `crates/kermlc_lower/`, `crates/kermlc_hir/src/{lib.rs,lower.rs,types.rs}`,
`crates/kermlc_ast/src/*` (enum move), dependents' `use` paths.

1. Create `crates/kermlc_lower`; move `crates/kermlc_hir/src/lower.rs` into it as the crate body.
   Depends on `kermlc_hir` + `kermlc_ast` + `kermlc_parser`. Public: `lower_ast`.
2. Move `FeatureDirection` and `Visibility` enum *definitions* from `kermlc_ast` into `kermlc_hir`
   (`types.rs`); have `kermlc_ast` re-use them from `kermlc_hir` (front-end → engine direction).
3. Remove `kermlc_ast` + `kermlc_parser` from `kermlc_hir`'s `[dependencies]` (engine is now
   AST-free). `kermlc_hir` keeps `types.rs` + `stdlib.rs` only.
4. Update `kermlc/src/lib.rs` and any test helper to import `lower_ast` from `kermlc_lower`.
5. QA: build + clippy + test green. Verify `harpoon_hir` (still `kermlc_hir` here) no longer
   compiles against `kermlc_ast`.

## Step C — Rename `kermlc_* → harpoon_*` (after A + B merged)

1. Rename the 7 engine/foundation crate directories + `package.name` in their `Cargo.toml`.
2. Update every `Cargo.toml` dependency key and every `use kermlc_<engine>::` path across the
   workspace (front-end crates and the `kermlc` binary included).
3. Keep `kermlc_{lexer,parser,ast,lower}` and the `kermlc` binary names unchanged.
4. QA: build + clippy + test green. Grep for stray `kermlc_{intern,diagnostics,hir,resolve,
   typeck,validate,serial_json}` references — must be zero.

## Agent / worktree split

- **Agent A** (own worktree): Step A.
- **Agent B** (own worktree): Step B.
  A and B touch mostly disjoint files; the only shared file is the workspace `Cargo.toml`
  (each adds one crate) — reconcile at merge.
- Land order: A → B → **C**. Step C runs after A + B are merged, as a single agent on the merged
  tree (a rename in parallel with edits would conflict everywhere).
- **Integral QA agent**: after C, full-workspace `cargo test` + `clippy -D warnings` + a smoke run
  of `kermlc check` / `kermlc compile` on a fixture.

Each agent: `set -euo pipefail` discipline, its own QA before handing back. Squash-merge per step.

## Reporting requirement

The final integral-QA report must list **every** ignored / skipped / disabled test with the
reason (no silent `#[ignore]`). Also report: baseline vs final test count, any test moved (from →
to), and confirmation the resolve↔typeck dev-dep cycle is gone (`cargo tree -i kermlc_typeck` /
`harpoon_typeck` shows no resolve edge).

## Out of scope (explicitly deferred)

- Candidates #3 (validation rules as modules), #4 (diagnostics catalog), #5 (model mutation API).
- B2 stdlib-from-files, `-I`/sysroot lookup, incremental build, `sysml_*` front-end.
- `forge` build-tool work (it keeps its name; no changes here).
