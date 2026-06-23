# ADR-0002 — In-memory kernel model is the primary artifact; serialization is an output

Status: Accepted — 2026-06-23

## Context

The end goal is running simulations on SysML/KerML models. Simulation needs an *evaluable*
in-memory model (expressions, behaviors, occurrences, concrete values), not a static document.
JSON-LD was being treated as the pipeline's product.

## Decision

- The in-memory **kernel model** is the primary compiled artifact and the handoff to the simulator.
- JSON-LD (and later XMI) are **serialization outputs** produced *from* that model — backend
  adapters behind a seam, not the source of truth.

## Consequences

- Serialization stays a swappable backend (JSON-LD now, XMI later → two adapters = a real seam).
- The simulator consumes the in-memory model directly; no round-trip through JSON-LD.

## Open decisions (revisit when relevant)

- **Incremental build** — whether the engine supports incremental / partial recompilation.
  Likely desirable for editor and simulation loops; deferred until the toolchain matures.
- **Standard library lookup** — `-I` for user libraries vs an implicit **sysroot** for the
  kernel / systems libraries (ties to B2 stdlib-from-files). Lean: sysroot for stdlib, `-I` for user libs.
