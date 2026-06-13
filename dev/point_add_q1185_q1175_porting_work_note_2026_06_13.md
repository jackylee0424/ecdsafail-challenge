# point_add q1185 Clean Port and q1175 Porting Formulation

Date: 2026-06-13

Scope: refactor research and dev tooling only. This note does not claim
submission readiness.

## Purpose

Use the clean q1185 route as the stable behavior-preserving baseline, then
formulate the q1175 route as a reviewable overlay with local formal obligations
before considering any broader port into `src/point_add`.

The important distinction is:

- q1185 porting is route-surface cleanup. It should preserve the accepted route
  behavior.
- q1175 porting is a structural experiment. It has clean empirical evidence and
  formalized local mechanisms, but it is not a score beat and is not a
  proof-complete route.

## Clean q1185 Port Recipe

The q1185 route is the default clean state. Current measured evidence:

```text
route: default q1185
commit label in results.tsv: cf310ec
qubits: 1185
avg executed Toffoli: 1418586.774
avg executed Clifford: 5788577.207
emitted ops: 10186572
correctness: 0 classical / 0 phase / 0 ancilla
score: 1681025595
```

The clean port strategy is to turn
`configure_ecdsafail_submission_route()` into a small preset selector while
moving route entries into typed data:

- `set_default_env(name, value)` becomes `EnvVar::new(name, value)`.
- `std::env::set_var(name, value)` becomes `EnvMutation::Set(name, value)`.
- `std::env::remove_var(name)` becomes `EnvMutation::Remove(name)`.
- Applying a submission route means defaults first, forced pins second, removes
  exactly where declared.
- Caller overrides survive for defaults.
- Forced pins override caller env.
- Remove pins delete caller env.

The default route should stay:

```rust
PointAddRoutePreset::accepted_cf310ec().apply_for_submission()
```

That gives a behavior-preserving route-surface refactor: the large env block is
reviewable as typed data without changing the default build target.

## q1185 Review Gates

A clean q1185 route port is not ready just because it compiles. The minimum
review gates are:

```powershell
cargo fmt --check
cargo test
cargo run -- --dump
cargo run -- --formal-check
cargo check
```

For a purely organizational refactor, also compare emitted ops against a
reference artifact:

```powershell
cargo run -- --compare-ops <reference-ops.bin> <candidate-ops.bin>
```

If the goal is byte-level behavior preservation, the ops artifact should match
byte-for-byte. If the change is structural, byte match is not expected; the
route must instead pass trusted `build_circuit` plus `eval_circuit`.

## q1175 Formulation

The q1175 route is expressed as an overlay on top of the clean q1185 preset, not
as the default route:

```text
POINT_ADD_ROUTE_PRESET=q1175_repaired_clean_91794252
```

Current measured evidence:

```text
route: q1175_repaired_clean_91794252
commit label in results.tsv: cf310ec
qubits: 1175
avg executed Toffoli: 1545824.522
avg executed Clifford: 6166104.828
emitted ops: 11005540
correctness: 0 classical / 0 phase / 0 ancilla
score: 1816344375
```

This saves 10 qubits relative to q1185, but gives back about 127238 average
executed Toffoli. It is clean, but it is not a score beat.

The overlay pins are:

```text
DIALOG_GCD_BOUNDARY_REPLAY_BORROW_CLEANED=1
DIALOG_GCD_APPLY_ALL_DIRTY_QOFFSET=1
DIALOG_GCD_FOLD_STREAM_CONTROLS=1
DIALOG_GCD_FOLD_HOST_STREAMED_CONTROL=1
DIALOG_GCD_FOLD_HOST_E_TOP_CARRY=1
DIALOG_GCD_FOLD_HOST_D_CARRY12=1
DIALOG_GCD_FOLD_HOST_OVF2_CARRY13=1
DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS=22
DIALOG_GCD_FOLD_CARRY_TRUNC_W=18
DIALOG_GCD_FOLD_PARK_LOW_CARRIES=17
DIALOG_GCD_SPECIAL_FOLD_PARK_LOW_CARRIES=15
SQUARE_ROW_MAX_SEG=144
DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES=1
DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS=25
DIALOG_GCD_APPLY_CHUNKED_F_CUTS=16,31,46,61,75,89,103,116,129,141,153,164,175,185,195,204,213,221,229,236,243,249,253,255
DIALOG_TAIL_NONCE=91794252
```

## q1175 Structural Deltas

The q1175 overlay depends on these local mechanisms:

- dirty qoffset apply add/sub paths:
  `iadd_dirty_2clean_qoffset`, `iadd_dirty_2clean_qoffset_carry_q`,
  `isub_dirty_2clean_qoffset`, and `isub_dirty_2clean_qoffset_borrow_q`.
- chunked apply add/sub routing that uses the dirty-qoffset path only when:
  implicit high zero is active, dirty-qoffset mode is active, at least three
  future boundary carries are available, and the source suffix is wide enough.
- cleaned boundary replay borrowing, where already-measured boundary targets may
  be borrowed as carry lanes during phase-conditioned replay.
- streamed/hosted fold controls, gated by the high `DIALOG_GCD_FOLD_*` parking
  thresholds.

These are structural changes. They should not be reviewed as nonce-only changes.

## q1175 Port Slices

The source selector exposes q1175 as incremental overlays on top of the q1185
default:

```text
POINT_ADD_ROUTE_PRESET=q1175_dirty_qoffset_first
POINT_ADD_ROUTE_PRESET=q1175_dirty_qoffset_core
POINT_ADD_ROUTE_PRESET=q1175_boundary_borrow
POINT_ADD_ROUTE_PRESET=q1175_apply_chunk_shape
POINT_ADD_ROUTE_PRESET=q1175_repaired_clean_91794252
```

The intended order is:

1. `q1175_dirty_qoffset_first`: the narrowest formally-backed local mechanism.
   It enables future boundary carry borrowing and dirty-qoffset only for the
   first eligible apply block.
2. `q1175_dirty_qoffset_core`: the all-block dirty-qoffset mechanism.
   It enables future boundary carry borrowing and the dirty-qoffset add/sub
   path, while preserving the q1185 chunk count and other q1185 pins.
3. `q1175_boundary_borrow`: adds cleaned-boundary replay borrowing. This is a
   lifecycle/formal-contract slice, not yet a Lean all-width proof slice.
4. `q1175_apply_chunk_shape`: adds the q1175 25-block apply chunk shape and
   explicit cut list. This is the first slice that should move the apply peak
   toward the 1175q structure.
5. `q1175_repaired_clean_91794252`: full known-clean q1175 overlay, including
   fold-hosting, park-low carries, square-row segment, clean-compare giveback,
   and seed nonce.

Only slice 1 is currently backed by both bounded Z3 gate simulation and Lean
all-width arithmetic-model qoffset facts. Treat later slices as structured
experiments until they have trusted CPU evidence.

Initial CPU evidence:

```text
q1175_dirty_qoffset_first:
  emitted ops: 10247550
  qubits: 1185
  avg executed Toffoli: 1431485.885
  verdict: not clean
  failures: 13 classical mismatches, 7 phase-garbage batches, 0 ancilla batches

q1175_dirty_qoffset_core:
  emitted ops: 10887554
  qubits: 1185
  avg executed Toffoli: 1523586.295
  verdict: not clean
  failures: 11 classical mismatches, 9 phase-garbage batches, 0 ancilla batches

q1175_apply_chunk_shape:
  emitted ops: 10886266
  qubits: 1185
  avg executed Toffoli: 1529028.504
  verdict: not clean
  failures: 19 classical mismatches, 7 phase-garbage batches, 0 ancilla batches
```

These failures mean the dirty-qoffset port is not independently compatible
with the q1185 route. The local primitive is formally covered, but the route
still needs a larger coupled context before it is clean. The next small step is
not more nonce search; it is to isolate which full-q1175 pins provide the
missing route-level side condition, starting with fold-hosting and park-low
carry changes.

## Formal Shape

The q1175 formulation is split across the four dev formal tools:

- MDD: `dev/solinas_cuccaro_adder_key.mmd` splits q1175 into structural deltas,
  lossy-island pins, and the seed-only nonce pin.
- TLA+: `Q1175LifecycleSafe` checks dirty-qoffset lifecycle, future boundary
  carry availability, cleaned-boundary borrowing, hosted fold controls, and
  cleanup to zero live q1175 resources.
- Z3: `q1175_dirty_qoffset_*` symbolically executes the bounded Boolean gate
  skeleton for add, add-with-carry-q, sub, and sub-with-borrow-q.
- Lean4: `SolinasCuccaroFacts.lean` proves the q1175 dirty-qoffset add/sub
  arithmetic-model equivalence, dirty-word restoration, the concrete overlay
  shape, and the q1185/q1175 score identities. Broader lifecycle hooks remain
  named obligations.

The proof boundary is intentionally conservative. The formal tools do not prove:

- `DIALOG_TAIL_NONCE=91794252`,
- all-width dirty-qoffset equivalence,
- all reachable carry-window guards,
- convergence or width safety for all lossy-island pins.

Trusted CPU `eval_circuit` remains the promotion gate.

## Porting Order

Recommended order for a future source port:

1. Keep q1185 as the default typed preset and verify it independently.
2. Keep q1175 behind `POINT_ADD_ROUTE_PRESET=q1175_repaired_clean_91794252`.
3. Port/review q1175 structural helpers separately from route pins.
4. Run formal checks before any trusted eval:

```powershell
cargo run --manifest-path dev\point_add_route_refactor\Cargo.toml -- --formal-check
```

5. Build and evaluate q1185 and q1175 separately:

```powershell
Remove-Item Env:\POINT_ADD_ROUTE_PRESET -ErrorAction SilentlyContinue
cargo run --release --bin build_circuit
cargo run --release --bin eval_circuit -- --note q1185_default_after_port

$env:POINT_ADD_ROUTE_PRESET='q1175_repaired_clean_91794252'
cargo run --release --bin build_circuit
cargo run --release --bin eval_circuit -- --note q1175_overlay_after_port
```

6. Do not promote q1175 unless it remains trusted-clean and a follow-on score
   experiment reduces its Toffoli cost enough to beat q1185.

## Next Scoring Work

Use q1175 as a clean qubit-floor scaffold, then peel back expensive pins one at
a time:

- `DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS=22 -> 21 -> 20`
- `DIALOG_GCD_FOLD_PARK_LOW_CARRIES=17` downward toward q1185
- `DIALOG_GCD_SPECIAL_FOLD_PARK_LOW_CARRIES=15` downward toward q1185
- `SQUARE_ROW_MAX_SEG=144` upward toward q1185 values

Each step should get formal/harness classification first, then trusted CPU
validation. GPU island search is only useful after the structural row is not
already disqualified by local formal or CPU evidence.
