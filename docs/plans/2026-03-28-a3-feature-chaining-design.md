# A3: Feature Chaining Resolution — Type-Directed Chain Walking

## Scope

Only the `chains` keyword context: `feature x chains a.b.c;`. Dot-chains in
subsetting/specialization contexts (`subset g.g subsets b.f.a;`) and expression
contexts (`feature g = f.a;`) are deferred to B3 (grow grammar).

## Problem

The current implementation resolves each chain segment independently via
scope-based name resolution. This is incorrect — segment `[i]` must be resolved
as a member of the type of segment `[i-1]`, not in the enclosing scope.

Example:

```kerml
package P {
    type Vehicle {
        feature engine : Engine;
    }
    type Engine {
        feature cylinders : Cylinder;
    }
    type Cylinder {}
    type Fleet {
        feature vehicles : Vehicle;
        feature v_cyl chains vehicles.engine.cylinders;
    }
}
```

`engine` is not in Fleet's scope — it is a member of `Vehicle`. The resolver
must walk the type chain: `vehicles` (scope-based) -> type `Vehicle` ->
`engine` (member of Vehicle) -> type `Engine` -> `cylinders` (member of Engine).

## Approach: Resolve-local scope function with eager walk

### New function: `find_member` in `scope.rs`

```rust
/// Find a member (direct child or inherited feature) of a type.
/// No parent walking, no imports — strict member lookup only.
pub fn find_member(
    model: &SemanticModel,
    type_def_id: DefId,
    name: SymbolId,
) -> Option<DefId>
```

Searches:
1. Direct children of `type_def_id` by name
2. `inherited_features` of `type_def_id` (if typeck has populated them)

No parent walking, no imports — chain semantics require strict member lookup.
This function is reusable for future B3 dot-chain contexts.

### Modified `resolve_chains_for` in `resolve.rs` — eager walk

```
resolve_chains_for(def_id):
  for i in 0..chain_segments.len():
    if segment[i] already resolved -> continue

    if i == 0:
      resolve scope-based (as today)
    else:
      prev = resolved DefId of segment[i-1]
      prev_type_ref = model.defs[prev].type_ref
      if prev_type_ref unresolved -> break  (wait for typeck)
      type_def = prev_type_ref.resolved DefId
      find_member(model, type_def, segment[i].name):
        found       -> set Resolved, changed = true
        not found AND type is type_checked ->
            set Error, emit "not a member of type T"
        not found AND type NOT type_checked ->
            break  (inherited features may not be populated yet)

  if all segments resolved:
    def.chain_result = Some(last resolved DefId)
    changed = true

  return changed
```

Key properties:
- Resolves as many segments as possible in a single call (eager walk)
- A chain `a.b.c.d.e` where all types are known resolves in one pass
- When blocked (type unknown), stops and returns progress so far
- Distinguishes "not found because type not yet checked" (defer) from
  "not found because member does not exist" (error)

### New field on `Def`: `chain_result`

```rust
pub chain_result: Option<DefId>
```

Set when the entire chain resolves successfully — the DefId of the last
segment. Downstream code (typeck, serialize) uses this to determine the
resultant type of the chain.

## Data flow in fixpoint loop

```
Iteration 1: resolve segment[0] scope-based        -> OK
             resolve segment[1] via find_member     -> OK (type known)
             resolve segment[2]: type_ref unknown   -> break
             typeck populates type_ref for [1]

Iteration 2: resolve segment[2] via find_member     -> OK
             all segments resolved -> chain_result set
             no more progress needed
```

Worst case: 1 fixpoint iteration per segment (when each type depends on
the previous typeck pass). Best case: all segments in 1 iteration.

## Error handling

- "chain segment `x` is not a member of type `T`" — emitted when
  find_member returns None and the type is fully type-checked (no more
  inherited features expected)
- Unresolved segments after fixpoint exhaustion fall through to the
  existing `emit_unresolved` machinery for chain_segments

## Serialization

`chain_result` is used in `kermlc_serial_json` to emit `chainingFeature`
references in JSON-LD output.

## Tests

1. Basic two-step chain: `feature x chains a.b`
2. Three-step chain: `feature x chains a.b.c`
3. Chain with inherited feature (typeck must populate before resolve finds it)
4. Unresolved chain segment -> error diagnostic
5. Chain resolves across fixpoint iterations (intermediate type initially unknown)

## Future work (not in A3)

- **B3-B**: Dot-chains in subsetting/specialization contexts
  (`subset g.g subsets b.f.a;`)
- **B3-C**: Dot-chains in expression contexts (`feature g = f.a;`)

These use the same `find_member` function but require parser changes to
recognize dot notation in those positions.
