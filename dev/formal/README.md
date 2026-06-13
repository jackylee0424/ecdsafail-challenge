# point_add Solinas/Cuccaro Formal Artifacts

This directory is research tooling only. It does not change `src/point_add`
behavior and does not claim submission readiness.

## Files

- `PointAddSolinasCuccaroAdder.tla`: TLA+ lifecycle/value model for the core
  Cuccaro adder and Solinas sparse-constant variants.
- `PointAddSolinasCuccaroAdder.cfg`: small TLC model for the KAL double key.
- `PointAddSolinasCuccaroAdderQ1175.cfg`: TLC model for the q1175 lifecycle
  delta: dirty qoffset apply, borrowed cleaned boundary replay, future boundary
  carry borrowing, and streamed/hosted fold controls.
- `check_solinas_cuccaro_z3.py`: runnable bounded Z3 checks using the Python
  `z3` package, including q1175 dirty-qoffset gate-skeleton checks.
- `solinas_cuccaro_adder_bv.smt2`: SMT-LIB mirror of the Z3 obligations for
  environments with a `z3` CLI. The q1175 dirty-qoffset gate simulation is kept
  in the Python checker because the explicit Boolean circuit is easier to audit
  there.
- `SolinasCuccaroFacts.lean`: Lean4 theorem names and secp256k1 fact hooks.

## Model Boundary

The TLA+ module checks the workflow contract:

- `control_model`: materialized, direct-controlled, or control-by-prep.
- `carry_model`: full, truncated window, or guarded tail.
- `cleanup_model`: coherent, measured/HMR, or phase-corrected.
- `host_model`: fresh, borrowed, streamed, or derived-control-hosted.
- done states must have clean scratch, no borrowed lanes, no live stream host,
  and no phase debt.

The Z3 checks cover bounded bit-vector facts for a toy sparse constant
`c = 2^8 + 2^4 + 1` under a 16-bit pseudo-Mersenne model. The Lean file owns
the stable theorem names for the full secp256k1 facts that should eventually
replace the TLA assumptions and bounded Z3 checks.

## q1175 Formal Boundary

The q1175 route is treated as a clean empirical state plus formalized local
mechanisms, not as an end-to-end proof. The formal scope is:

- MDD: `q1175 repaired clean overlay` is split into structural deltas, lossy
  island pins, and the seed-only nonce pin `DIALOG_TAIL_NONCE=91794252`.
- TLA+: `Q1175LifecycleSafe` checks that dirty-qoffset use has at least three
  future boundary borrows, cleaned boundary replay only borrows cleaned targets,
  and streamed/hosted fold controls have borrowed host capacity.
- Z3: `q1175_dirty_qoffset_*` symbolically executes the bounded Boolean gate
  skeleton for add, add-with-carry-q, sub, and sub-with-borrow-q, proving target
  equivalence plus dirty/offset/scratch restoration.
- Lean4: q1175 dirty-qoffset add/sub equivalence and dirty restoration are now
  all-width theorems over the Lean arithmetic model. The remaining q1175
  lifecycle hooks name obligations that still need deeper proof terms.

The nonce is intentionally outside the proof boundary. It remains CPU-evaluated
evidence only.

## Commands

Run the bounded Z3 checks:

```powershell
python dev\formal\check_solinas_cuccaro_z3.py
```

Run all installed formal checks through the route-refactor dev harness:

```powershell
cargo run --manifest-path dev\point_add_route_refactor\Cargo.toml -- --formal-check
```

Run TLC if `tla2tools.jar` is available:

```powershell
java -cp dev\formal\tools\tla2tools.jar tlc2.TLC -metadir dev\formal\tlc-states -config dev\formal\PointAddSolinasCuccaroAdder.cfg dev\formal\PointAddSolinasCuccaroAdder.tla
```

Run the q1175 TLC lifecycle model directly:

```powershell
java -cp dev\formal\tools\tla2tools.jar tlc2.TLC -metadir dev\formal\tlc-states-q1175 -config dev\formal\PointAddSolinasCuccaroAdderQ1175.cfg dev\formal\PointAddSolinasCuccaroAdder.tla
```

Run Lean through the installed Lean4 toolchain:

```powershell
elan run 4.30.0 lean dev\formal\SolinasCuccaroFacts.lean
```

On this Windows workspace the plain `lean` shim may report no active toolchain
even after `elan default`; the installed Lean 4.30 binary can also be used
directly:

```powershell
C:\Users\doomsday\.elan\toolchains\leanprover--lean4---v4.30.0\bin\lean.exe dev\formal\SolinasCuccaroFacts.lean
```
