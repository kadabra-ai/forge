# ADR-0001 — Two front-end drivers over one shared kernel engine

Status: Accepted — 2026-06-23

## Context

The toolchain must compile both KerML and SysML v2. SysML v2's abstract syntax is defined as
specializations of KerML; SysML constructs *are* KerML constructs semantically. The semantic
core (lowering target, name resolution, type checking, validation, serialization) already depends
only on `kermlc_hir` — verified: `kermlc_parser` is a `dev-dependency` of resolve/typeck/validate,
used only in `#[cfg(test)]`. The core is therefore already front-end-agnostic.

## Decision

- KerML is the semantic IR. SysML is a **front-end** that lowers to the same kernel model;
  it is not an independent compiler.
- Ship **two thin driver binaries**: `kermlc` (KerML surface) and `sysmlc` (SysML surface),
  mirroring `rustc`. Each = front-end + the shared engine.
- The engine is **one shared library** named **harpoon**. The facade crate `harpoon` exposes
  `compile()` (the deep entry point, see ADR/grilling) and re-exports the engine types drivers
  need. Engine crates are renamed `kermlc_* → harpoon_*`
  (`harpoon_hir`, `harpoon_resolve`, `harpoon_typeck`, `harpoon_validate`, `harpoon_serial_json`).
  Shared-foundation crates follow suit (`harpoon_intern`, `harpoon_diagnostics`).
- **Front-end crates keep the `kermlc_*` prefix** (`kermlc_lexer`, `kermlc_parser`, `kermlc_ast`),
  with future `sysml_*` counterparts — they are language-specific.
- **`forge` keeps its name**: it is the build tool / package manager over the drivers
  (the `cargo` of this toolchain).

Crate layering:

```
forge                         build tool / package manager  (= cargo)
  └─ kermlc / sysmlc          driver binaries, language-specific
       └─ harpoon             engine facade  →  harpoon::compile()
            └─ harpoon_hir / _resolve / _typeck / _validate / _serial_json
```

### Lowering split (forced by the rename)

`lower_ast` consumes the KerML AST, so it cannot live in `harpoon_hir` without the engine
depending on `kermlc_ast`. Therefore:

- `harpoon_hir` (engine) holds only the language-agnostic data model: `types.rs` + `stdlib.rs`.
  The `FeatureDirection` and `Visibility` enums are **defined here**; `kermlc_ast` reuses them.
- KerML lowering moves to a new front-end crate **`kermlc_lower`** (`lower_ast`), depending on
  `harpoon_hir` + `kermlc_ast` + `kermlc_parser`. SysML will add a parallel `sysml_lower`.

## Consequences

- The kernel model (`SemanticModel`, in `harpoon_hir`) is the load-bearing seam: both front-ends
  target it; the engine never reaches into a parser.
- Adding SysML = new front-end crates (`sysml_lexer` / `sysml_parser` / `sysml_ast` /
  `sysml_lower`) reusing the engine unchanged.
- `add_implicit_specializations` and stdlib lookup must be parameterizable by language — KerML
  and SysML specialize to different standard libraries.
- Risk avoided: duplicating resolve/typeck/validate per language and letting their semantics diverge.
