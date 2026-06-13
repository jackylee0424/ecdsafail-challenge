# point_add Refactor Evaluation for Fresh Formal Knobs

Date: 2026-06-13

Challenge root inspected: `C:\code\ecdsafail-challenge`

Challenge HEAD: `cf310ec` on `main`

Formal baseline: `analysis/current_baseline.json` commit `cf310ec`

Trusted baseline metric: `1185q x 1418587T = 1681025595`, validated `0/0/0`
over 9024 shots in the baseline ingest metadata.

## Verdict

Refactor `point_add`, but keep the cut narrow.

Do not do a wholesale rewrite of the arithmetic or builder topology. The useful
refactor is a route-surface refactor: extract the large
`src/point_add/mod.rs::configure_ecdsafail_submission_route()` default/override
block into named, typed route presets and candidate overlays that the formal
toolset can round-trip.

The reason is practical: the current source already contains several fresh knob
readers, but the knobs are scattered across env readers, the filter model, and
the final hard q1185 override block. Formal tooling can generate adjacent rows,
but it cannot yet materialize or classify the deeper per-step/vector knobs as
first-class route families without another ad hoc env list.

## Current Evidence

The formal baseline and challenge source are aligned:

- `analysis/current_baseline.json`: `cf310ec`, `1185q x 1418587T`.
- `C:\code\ecdsafail-challenge`: clean `main` at `cf310ec`.
- Peak binders at 1185q:
  - `dialog_gcd_apply_chunk_add_final_ripple`
  - `dialog_gcd_apply_chunk_add_ripple`
  - `dialog_gcd_apply_chunk_sub_final_ripple`
  - `dialog_gcd_apply_chunk_sub_ripple`
  - `dialog_gcd_compressed_block_apply_double_y`
  - `dialog_gcd_compressed_block_apply_reverse_halve_y`
  - `dialog_gcd_materialized_special_overflow_fold`
  - `dialog_gcd_materialized_special_underflow_fold`
  - `round84_inplace_solinas_square_forward`
  - `round84_inplace_solinas_square_inverse`

Formal row generation is already live:

```text
analysis/bundle_manifest.py --counts
immediate_priority: 11
coupled_first_shell: 778
next_qubit_ladder: 16
q1297_ladder: 16
q1298_ladder: 12
q1285_blueprint: 18
round84_split_ladder: 4
guarded_transfer: 49
structural_strategy_axis: 20
```

Immediate-priority first shell on `cf310ec`:

```text
1  round84_bigfold_split_minus1       ROUND84_BIGFOLD_SPLIT=0
2  width_margin_minus1_active_minus1  DIALOG_GCD_WIDTH_MARGIN=9, DIALOG_GCD_ACTIVE_ITERATIONS=257
3  width_margin_minus1                DIALOG_GCD_WIDTH_MARGIN=9
4  active_iterations_minus1           DIALOG_GCD_ACTIVE_ITERATIONS=257
5  kal_fold_minus1_nonce_reroll       KAL_FOLD_CARRY_TRUNC_W=17
6  kal_double_minus1_nonce_reroll     KAL_DOUBLE_CARRY_TRUNC_W=18
7  kal_double_fold_minus1             KAL_DOUBLE_CARRY_TRUNC_W=18, KAL_FOLD_CARRY_TRUNC_W=17
8  apply_clean_minus1                 DIALOG_GCD_APPLY_CLEAN_COMPARE_BITS=18
9  width_slope_plus1                  DIALOG_GCD_WIDTH_SLOPE_X1000=1016
10 compare_minus1                     DIALOG_GCD_COMPARE_BITS=45
11 round84_quot_minus1                ROUND84_INPLACE_QUOTIENT_CARRY_TRUNC_W=20
```

Dry-run microtrim report:

- output: `analysis/worker_runs/point_add_refactor_eval_minimal/report.md`
- route count: 7
- same-q score gate at 1185q: must reduce rounded Toffoli by at least 1.
- 1184q score gate: may spend up to 1198 rounded Toffoli.
- 1175q score gate: may spend up to 12073 rounded Toffoli.

## Refactor Point

Target the configuration layer first:

1. Extract `configure_ecdsafail_submission_route()` into a small route preset
   API, for example:
   - `PointAddRoutePreset::accepted_cf310ec()`
   - `PointAddRouteOverlay`
   - `apply_defaults()`
   - `apply_for_submission()`
   - `fingerprint()`
2. Preserve env compatibility. Existing scripts and challenge binaries should
   still accept env overrides exactly as today.
3. Split "default if absent" from "force this submitted route". The current q1185
   block uses hard `set_var` overrides after the default stack; formal tools
   need to know which values are inherited defaults and which are submission
   pins.
4. Share parsers between circuit config and classical filter config for map and
   vector knobs. Today the circuit and filter both parse related env strings,
   which is a drift risk for new per-step knobs.

Do not move the arithmetic implementations in this first step. The goal is to
make new route axes cheap to expose, not to change circuit behavior.

## Fresh Knob Surface to Expose

These are already present in the challenge source or partially modeled, and are
good candidates for formal-tool generated route rows after the route-surface
refactor:

- `DIALOG_GCD_APPLY_CHUNKED_F_CUTS`: vectorized apply chunk boundaries for the
  current 18-block q1185 family. This is better than extending only the older
  `CUT`, `CUT2`, `CUT3`, `CUT4` scalar family.
- `DIALOG_GCD_COMPARE_STEP_BITS`: per-step compare schedule overrides from the
  reachable-support/convergence model.
- `DIALOG_GCD_FOLD_CARRY_TRUNC_STEP_WINDOWS` and
  `DIALOG_GCD_SPECIAL_FOLD_CARRY_TRUNC_STEP_WINDOWS`: per-step fold trim maps.
- `DIALOG_GCD_TOBITVECTOR_SHIFT_BODY_TRIM`: direct body-width reuse for the
  shift path. Treat as a qubit-ladder row requiring strict filter plus trusted
  CPU validation.
- `DIALOG_GCD_K5_HEAD11_CODEC`: head-codec compression family. Keep this tied
  to a codec self-test and matching filter semantics.
- `DIALOG_GCD_APPLY_IMPLICIT_HIGH_ZERO` and
  `DIALOG_GCD_APPLY_BORROW_FUTURE_BOUNDARY_CARRIES`: q1185 apply-boundary
  family; useful only if the filter and circuit route consume the same route
  object.
- `SQUARE_ROW_WINDOW_CLEAN_ROW_BITS` and
  `SQUARE_ROW_WINDOW_CLEAN_SITE_BITS`: targeted square cleanup trims. These are
  secondary while the 1185q route is co-bound by apply, compressed-block, special
  fold, and round84 square phases.
- `DIALOG_GCD_FOLD_HOST_*` controls in `arith/const_arith.rs`: potentially high
  value, but higher blast radius. Put behind explicit proof obligations before
  adding to the first shell.

## GPU Search-Space Reduction

The stronger reason to refactor is not readability. It is to collapse GPU search
space.

Today many route rows can point at the same nonce-quality class:

- rows that only change the spelling of a derived envelope;
- rows that move a lossless/provable-exact truncation but leave the actual hard
  Fiat-Shamir failure mode unchanged;
- rows where the global nonce failure is caused by a different co-located floor
  than the knob being varied;
- rows that share the same secp256k1 affine-add input distribution and GCD
  factor hardness, even if their resource route differs.

Those rows should not each receive independent broad GPU windows. The route
surface should expose enough structure to compute a `nonce_quality_key` before
launch:

```text
nonce_quality_key =
  affine_input_seed_path
  + dialog_gcd_factor_model
  + width/convergence envelope
  + compare decision schedule
  + apply-clean/special-clean hazard model
  + phase-clean measured/HMR hazard set
```

Rows with the same key should share density evidence and candidate windows. Rows
with a strictly stronger formal key should inherit only the safe parts of the
evidence. Rows with a changed seed path but unchanged hazard model need a small
density calibration, not a full independent sweep. Rows that change the hazard
model need a fresh bounded probe.

This turns route planning from:

```text
route row -> GPU island search
```

into:

```text
route row -> classify axis kind -> nonce_quality_key -> dedupe/search budget
```

Axis classification should be:

- `derived`: recompute from parent structure; no GPU search just to find the
  value. Width slope/margin and some active-iteration schedules are in this
  class.
- `provable_exact`: isolated classical/formal sweep finds the lossless floor;
  GPU search is skipped unless the row still changes a genuinely lossy hazard.
- `seed_only`: same hazard model, different op-stream seed path; inherit density
  and run only a small calibration window.
- `lossy_island`: genuinely changes the hard-input class; spend GPU budget here.

This matches the existing formal-toolset lesson from `PLAN.md`: width
slope/margin should be recomputed from the parent K axis, and every pessimistic
island verdict should first pass a current-baseline falsifier or provable-exact
sweep before it is allowed to consume broad GPU time.

## secp256k1 Research Transfer

Use secp256k1 point-add research as a source of equivalence classes and
component rewrites, not as a wholesale architecture replacement.

The contest route is specialized affine point addition with two Kaliski-style
uncomputations over:

```text
p = 2^256 - 2^32 - 977
a = 0
b = 7
```

Prior Solinas/Cuccaro review already found that full affine/Jacobian mixed-add
machinery is too wide for this benchmark. The transferable material is narrower:

- sparse `c = 2^32 + 977` aggregate folds;
- control-by-prep identities;
- guarded carry truncation;
- measured/HMR phase-clean cleanup patterns;
- lifetime splits that reduce co-live lanes without changing the affine
  input distribution.

Those research-derived transformations are especially useful if the route
surface can express their proof obligations and nonce-quality key. Otherwise the
same idea appears as several ad hoc env rows and gets overcharged with duplicate
GPU searches.

## Solinas-Cuccaro Adder Research

Treat Solinas-Cuccaro research as an adder-normalization layer.

The challenge source already contains multiple variants of the same underlying
adder idea:

- materialized `ctrl ? c : 0` constant add/sub paths;
- direct controlled sparse-constant add/sub paths;
- measurement-uncomputed Cuccaro ripples;
- fold/double carry-tail truncation windows;
- per-position majority optimizations;
- hosted or streamed derived controls in the Solinas fold tail.

These variants should be represented as one typed adder family with attributes,
not as unrelated route env strings:

```text
SolinasCuccaroAdderKey =
  operation(add/sub/fold/double/halve)
  + constant_form(sparse_c, signed_sparse_c, generic)
  + control_model(materialized, direct_controlled, control_by_prep)
  + carry_model(full, truncated_window, guarded_tail)
  + cleanup_model(coherent, measured_hmr, phase_corrected)
  + host_model(fresh, borrowed, streamed, derived_control_hosted)
```

Refactor around this key, then let each formal tool own a different part of the
classification:

- MDD: route/evidence taxonomy. The new graph
  `mdd/solinas_cuccaro_adder_key.mmd` records the six fields and the evidence
  flow into `nonce_quality_key`. Every secp256k1 Solinas/Cuccaro route row
  should land on this graph before it receives GPU budget.
- TLA+: lifecycle and workflow safety. TLA should check borrowed scratch,
  streamed hosts, measured/HMR cleanup reuse, phase-corrected cleanup, and peak
  envelope safety. It should also preserve the existing rule that missing proof
  blocks promotion but does not close a route family.
- Z3: bounded bit-vector obligations. Z3 should prove or counterexample
  sparse-`c` add/sub equivalence, direct-controlled versus control-by-prep
  rewrites, carry-window guards, known-zero controls, and phase side
  conditions for measured cleanup.
- Lean4: deep all-input facts. Lean should own the secp256k1 prime identities,
  modular action of signed sparse-`c` folds, width/convergence assumptions, and
  named facts that Z3 imports as assumptions. Open Lean obligations keep a row
  conditional; they do not become proof-clean by GPU evidence alone.

The desired route dump shape is:

```text
point_add_route_dump =
  effective_env
  + point_add_phase
  + SolinasCuccaroAdderKey
  + proof_obligations(mdd_node, tla_model, z3_query, lean_theorem)
  + nonce_quality_key
```

That key lets the formal tools answer a better question before GPU launch:

```text
does this row change the adder's mathematical action, or only its resource
realization?
```

If it only changes the resource realization and the cleanup proof is present,
the row belongs in `provable_exact` or `seed_only`, not `lossy_island`. If it
changes the carry-tail hazard or measured cleanup boundary, it gets a new
`nonce_quality_key` and a bounded probe.

Concrete research transfers:

- `sparse_c_tail_guard`: keep the truncated Solinas correction, but add a small
  witness for whether the dropped sparse-`c` carry tail mattered.
- `control_by_prep_sparse_c`: prepare a zero-or-`c` addend under control, then
  run an unconditional Cuccaro ripple. This can reduce controlled-adder pressure
  but must carry an explicit phase-cleanup obligation.
- `signed_sparse_c_fold`: normalize the secp256k1 identity
  `c = 2^32 + 2^10 - 2^5 - 2^4 + 1` so round84, KAL double/fold, and apply
  guards share one proof object.
- `guarded_truncation`: replace raw one-bit giveback searches with rows that
  explain the dropped carry condition. This is the path for CPU-dirty
  GPU-clean lanes where the current evidence says `classical=0 phase=1` or
  low `classical+phase` dirt.
- `lifetime_split_cuccaro`: detect carry or constant-prep lanes that can be
  XORed into the output and uncomputed before a known 1185q binder.

This also gives route search a cleaner budget policy:

- broad GPU search only for distinct `SolinasCuccaroAdderKey` values with a
  genuinely changed hazard model;
- small calibration for seed-only re-spellings of the same adder key;
- no GPU search for adder rewrites with a current proof of value, phase, and
  lifecycle equivalence;
- trusted CPU eval remains mandatory before promotion.

## ecadd-1169-lowqubit Reference

The linked branch is useful as a structural reference, not as a route target:

- source: `https://github.com/teddyjfpender/ecdsafail-challenge/tree/ecadd-1169-lowqubit`
- verified branch commit: `ddb93eef6dc2ef759dbc5655119a9ba0db5cd682`
- scratch clone inspected: `C:\tmp\ecadd-1169-lowqubit-ddb93ee`
- prior validation memory for this branch: `1226q x 1444681.695T`,
  `9024/9024`

The current `cf310ec` baseline is already better at
`1185q x 1418587T`, so do not spend GPU budget searching the branch as-is.
Use it to seed the refactor vocabulary and equivalence tests.

Useful imports from the branch:

- Borrowed-carry and no-cin Cuccaro variants in `arith/adder.rs`. These should
  normalize under `SolinasCuccaroAdderKey` instead of becoming another loose
  set of env toggles.
- Hosted and borrowed scratch routing in the dialog GCD path, including host
  gated controls, body host carry-in, boundary hosts, apply replay swap host,
  and borrowed-subtrahend apply paths. These map directly to `host_model` and
  `lifetime_split_cuccaro`.
- Square row self-host/windowing surfaces in `multiply.rs`, including row max
  segment and self-hosted square cleanup lanes. These should become a
  `SquareRowKey` or an extension of `nonce_quality_key`.
- The old low-qubit recipe around `DIALOG_GCD_APPLY_CHUNKED_F_BLOCKS=10` and
  `SQUARE_ROW_MAX_SEG=199`. This is stale as a default, but good for generating
  old-parent versus new-parent deltas in formal tooling.
- Materialized/direct controlled sparse-constant adder paths and per-position
  majority optimization hooks. These are exactly the kind of secp256k1
  Solinas-Cuccaro rewrites that need proof/equivalence classification before
  any broad GPU assignment.

Make this branch a regression fixture for the proposed route dump:

```text
ecadd-1169-lowqubit route dump
  -> effective resource key
  -> SolinasCuccaroAdderKey
  -> SquareRowKey
  -> nonce_quality_key

cf310ec route dump
  -> same keys
  -> diff classified as resource-only, seed-only, provable-exact, or lossy
```

That fixture is a direct test of whether the refactor actually reduces GPU
island search space. If the diff cannot say which branch changes alter nonce
quality, the refactor is not yet doing the useful work.

## Proposed Work Order

1. Make `SolinasCuccaroAdderKey` the center of the refactor contract: every
   Solinas/Cuccaro adder site in `point_add` should emit operation, constant
   form, control model, carry model, cleanup model, and host model.
2. Add a route preset/overlay module in the challenge repo and move the q1185
   accepted route into it without behavior changes.
3. Add a route fingerprint dump mode that emits the effective knobs after all
   defaults and hard submission pins, plus the adder key and formal proof
   obligation handles. The formal toolset should ingest this instead of
   reconstructing effective env by hand.
4. Add `nonce_quality_key` generation to the route dump and teach
   `analysis/bundle_manifest.py` / `analysis/avg_gate_microtrim.py` to dedupe
   search budgets by that key.
5. Add formal decomposition gates:
   - MDD axis placement for every new adder key;
   - TLA lifecycle checks for host/cleanup changes;
   - Z3 bit-vector checks for carry/control/constant rewrites;
   - Lean4 theorem hooks for secp256k1 identities and global bounds.
6. Teach the generators to emit rows for vector/map knobs:
   `DIALOG_GCD_APPLY_CHUNKED_F_CUTS`, `DIALOG_GCD_COMPARE_STEP_BITS`,
   per-step fold windows, and codec rows.
7. Add the adder key to route profiles so sparse-`c`, control, carry-window,
   cleanup, and host variants can be deduped before GPU assignment.
8. Baseline guard before any search: rebuild challenge binaries, run the
   baseline control, and require the same `1185q x 1418587T` metric before
   trusting new knob probes.
9. Only then probe the minimal first shell plus the new vector/map rows. Promotion
   still requires trusted `0/0/0`; scanner-clean or `1/0/0` is not enough.

## Priority Conclusion

Prioritize the narrow route-surface refactor before broader `point_add`
rewrites. It should unlock fresh formal knobs with low behavior risk:
per-step compare bits, per-step fold windows, vector apply cuts, and codec
families. Do not switch effort into wholesale arithmetic refactoring until the
route-surface refactor can reproduce the current accepted `cf310ec` route
byte-for-byte or metric-for-metric.
