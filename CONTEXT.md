# Domain glossary — forge / kermlc

Ubiquitous language for the KerML / SysML v2 toolchain. Architecture vocabulary
(module, seam, depth, leverage, locality, adapter) lives in the `/codebase-design`
skill; this file names the **domain**.

## Languages & artifacts

- **KerML** — the kernel modeling language; serves as the semantic IR for the whole toolchain.
- **SysML v2** — systems modeling language whose abstract syntax is defined as specializations
  of KerML. It is *lowered* to KerML, not compiled independently.
- **Kernel model** (a.k.a. `SemanticModel`, "compiled model") — the in-memory resolved + typed
  model. The **primary artifact** and the shared seam both front-ends target. Source of truth
  for simulation. See ADR-0002.
- **Serialization backend** — emits the kernel model to an external format (JSON-LD today,
  XMI planned). An **output adapter**, not the handoff to the simulator.

## Components

- **harpoon** — the shared, language-agnostic **kernel engine**: the kernel-model data type +
  resolve + typeck + validate + serialization. The facade crate `harpoon` exposes `compile()`;
  engine crates carry the `harpoon_*` prefix (`harpoon_hir`, `harpoon_resolve`, `harpoon_typeck`,
  `harpoon_validate`, `harpoon_serial_json`, plus foundation `harpoon_intern`,
  `harpoon_diagnostics`). See ADR-0001.
- **Front-end** — language-specific lexer + parser + AST + lowering that produces a kernel model.
  KerML front-end crates keep the `kermlc_*` prefix; SysML adds `sysml_*`.
- **Driver** — a thin compiler binary over harpoon. `kermlc` (KerML), `sysmlc` (SysML).
  Mirrors `rustc`: one engine, two drivers.
- **forge** — build tool + package manager over the drivers (the `cargo` of this toolchain).

## Lowering

- **Lowering** — surface AST → kernel model. Lives in a front-end crate (`kermlc_lower`, later
  `sysml_lower`), never in the engine. KerML lowering is ~1:1; SysML lowering is **desugaring**:
  one surface element → several kernel elements, with implicit specializations to standard libraries.

## Lookup

- **Include path (`-I`)** — user library lookup folders (gcc/cc convention).
- **Sysroot** — root of the standard libraries (Kernel Semantic Library, Systems Library),
  available implicitly like Rust's `std`. Distinct from `-I`.
