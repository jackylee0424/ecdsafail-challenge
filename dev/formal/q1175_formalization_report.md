# q1175 Formalization Report

Status: research/dev formalization only. This does not claim submission
readiness and does not prove `DIALOG_TAIL_NONCE=91794252`.

## Route Delta

The q1175 overlay is formalized as a delta on top of the clean 1185q route:

- dirty qoffset apply add/sub:
  `DIALOG_GCD_APPLY_ALL_DIRTY_QOFFSET=1`
- borrowed cleaned boundary replay:
  `DIALOG_GCD_BOUNDARY_REPLAY_BORROW_CLEANED=1`
- future boundary carry borrowing:
  `DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES=1`
- streamed/hosted fold controls:
  `DIALOG_GCD_FOLD_STREAM_CONTROLS=1`,
  `DIALOG_GCD_FOLD_HOST_STREAMED_CONTROL=1`,
  `DIALOG_GCD_FOLD_HOST_E_TOP_CARRY=1`,
  `DIALOG_GCD_FOLD_HOST_D_CARRY12=1`,
  `DIALOG_GCD_FOLD_HOST_OVF2_CARRY13=1`
- lossy-island route pins:
  `DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS=22`,
  `DIALOG_GCD_FOLD_PARK_LOW_CARRIES=17`,
  `DIALOG_GCD_SPECIAL_FOLD_PARK_LOW_CARRIES=15`,
  `SQUARE_ROW_MAX_SEG=144`,
  `DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS=25`,
  `DIALOG_GCD_APPLY_CHUNKED_F_CUTS=16,31,46,61,75,89,103,116,129,141,153,164,175,185,195,204,213,221,229,236,243,249,253,255`
- seed-only pin:
  `DIALOG_TAIL_NONCE=91794252`

## Tool Coverage

MDD:

- `dev/solinas_cuccaro_adder_key.mmd` now has a `q1175 repaired clean overlay`
  branch.
- The nonce pin is explicitly separated as seed-only evidence.

TLA+:

- `PointAddSolinasCuccaroAdder.tla` includes q1175 lifecycle state for
  dirty-qoffset use, cleaned-boundary borrowing, future-boundary borrowing, and
  hosted fold controls.
- `PointAddSolinasCuccaroAdderQ1175.cfg` checks `Q1175LifecycleSafe` with the
  q1175 overlay shape.

Z3:

- `check_solinas_cuccaro_z3.py` symbolically executes the bounded Boolean gate
  skeleton for:
  - `q1175 dirty qoffset add restores dirty and equals full add`
  - `q1175 dirty qoffset add carry-q equals full add`
  - `q1175 dirty qoffset sub restores dirty and equals full sub`
  - `q1175 dirty qoffset sub borrow-q equals full sub`

Lean4:

- `SolinasCuccaroFacts.lean` now proves all-width arithmetic-model lemmas for
  q1175 dirty-qoffset add equivalence, sub equivalence, and dirty-word
  restoration.
- The Lean file also proves the q1175/q1185 score identities and the fact that
  q1175 is clean but not a score beat against the q1185 baseline metrics.
- Cleaned-boundary replay borrowing and streamed fold host lifecycle remain
  named proof hooks pending deeper proof terms.

## Validation

Command:

```powershell
cargo run -- --formal-check
```

Result:

- Z3: pass, including all four q1175 dirty-qoffset checks.
- TLC core config: pass.
- TLC q1175 config: pass.
- Lean4: pass.

## Remaining Proof Gap

The formalized q1175 delta proves local bounded gate equivalence, all-width
Lean arithmetic-model dirty-qoffset add/sub equivalence, dirty restoration, and
lifecycle safety. It does not prove:

- the nonce,
- all-width gate-level dirty-qoffset equivalence beyond the Lean arithmetic
  model,
- all reachable carry-window guard facts,
- all reachable convergence/width facts for the lossy-island pins.

Those gaps are exactly why q1175 remains a clean empirical state plus formalized
local mechanisms, not a proof-complete submission path.
