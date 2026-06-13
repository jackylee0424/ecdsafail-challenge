---- MODULE PointAddSolinasCuccaroAdder ----
\* Research-side TLA+ model for the point_add Cuccaro/Solinas adder family.
\*
\* Rust anchors:
\* - src/point_add/arith/adder.rs: Cuccaro MAJ/UMA and measured/HMR variants.
\* - src/point_add/arith/const_arith.rs: sparse-c direct/materialized const paths,
\*   truncated carry windows, per-position fold controls, hosted controls.
\* - src/point_add/arith/modular.rs: secp256k1 double/halve/fold actions.
\*
\* This module is intentionally small-state. TLC should run it with a toy
\* pseudo-Mersenne prime, while Z3/Lean own the bit-vector and secp256k1-wide
\* arithmetic facts named by the route dump.

EXTENDS Naturals, Integers, FiniteSets, TLC

CONSTANTS
    N,
    Q,
    P,
    C,
    CHi,
    CarryWindow,
    PeakLimit,
    TargetOperation,
    TargetConstantForm,
    TargetControlModel,
    TargetCarryModel,
    TargetCleanupModel,
    TargetHostModel,
    Q1175DirtyQoffset,
    Q1175BorrowCleanedBoundary,
    Q1175BorrowFutureBoundary,
    Q1175StreamFoldControls,
    Q1175ApplyBlocks,
    Q1175CleanCompareBits,
    Q1175FoldParkLow,
    Q1175SpecialFoldParkLow

VARIABLES
    pc,
    acc0,
    operand0,
    ctrl0,
    acc,
    operand,
    ctrl,
    key,
    owned,
    borrowed,
    streamed,
    phaseDebt,
    measured,
    peak,
    dirtyQoffset,
    boundaryBorrowed,
    futureBoundaryBorrowed,
    hostedFoldControls

vars ==
    << pc, acc0, operand0, ctrl0, acc, operand, ctrl, key,
       owned, borrowed, streamed, phaseDebt, measured, peak,
       dirtyQoffset, boundaryBorrowed, futureBoundaryBorrowed,
       hostedFoldControls >>

OpSet == {"add", "sub", "fold", "double", "halve"}
ConstantForms == {"sparse_c", "signed_sparse_c", "generic"}
ControlModels == {"materialized", "direct_controlled", "control_by_prep"}
CarryModels == {"full", "truncated_window", "guarded_tail"}
CleanupModels == {"coherent", "measured_hmr", "phase_corrected"}
HostModels == {"fresh", "borrowed", "streamed", "derived_control_hosted"}
PCs == {"Ready", "Prepared", "RippleDone", "Done"}

KeyOK(k) ==
    k \in [ operation : OpSet,
             constant_form : ConstantForms,
             control_model : ControlModels,
             carry_model : CarryModels,
             cleanup_model : CleanupModels,
             host_model : HostModels ]

TargetKey ==
    [ operation |-> TargetOperation,
      constant_form |-> TargetConstantForm,
      control_model |-> TargetControlModel,
      carry_model |-> TargetCarryModel,
      cleanup_model |-> TargetCleanupModel,
      host_model |-> TargetHostModel ]

RECURSIVE Pow2(_)
Pow2(i) ==
    IF i = 0 THEN 1 ELSE 2 * Pow2(i - 1)

Max(a, b) == IF a >= b THEN a ELSE b
Min(a, b) == IF a <= b THEN a ELSE b
ModQ(x) == x % Q
ModP(x) == x % P
Low(x, last) == x % Pow2(last + 1)
LastCarryBit == Min(N - 1, CHi + CarryWindow)

ASSUME
    /\ N \in Nat
    /\ N > 1
    /\ Q = Pow2(N)
    /\ C \in 1..(Q - 1)
    /\ P = Q - C
    /\ P > C
    /\ CHi \in 0..(N - 1)
    /\ Pow2(CHi) <= C
    /\ C < Pow2(CHi + 1)
    /\ CarryWindow \in Nat
    /\ PeakLimit \in Nat
    /\ TargetOperation \in OpSet
    /\ TargetConstantForm \in ConstantForms
    /\ TargetControlModel \in ControlModels
    /\ TargetCarryModel \in CarryModels
    /\ TargetCleanupModel \in CleanupModels
    /\ TargetHostModel \in HostModels
    /\ KeyOK(TargetKey)
    /\ Q1175DirtyQoffset \in BOOLEAN
    /\ Q1175BorrowCleanedBoundary \in BOOLEAN
    /\ Q1175BorrowFutureBoundary \in BOOLEAN
    /\ Q1175StreamFoldControls \in BOOLEAN
    /\ Q1175ApplyBlocks \in Nat
    /\ Q1175CleanCompareBits \in Nat
    /\ Q1175FoldParkLow \in Nat
    /\ Q1175SpecialFoldParkLow \in Nat
    /\ Q1175DirtyQoffset => /\ Q1175BorrowFutureBoundary
                            /\ Q1175ApplyBlocks >= 3
    /\ Q1175BorrowCleanedBoundary => Q1175BorrowFutureBoundary
    /\ Q1175StreamFoldControls => Q1175FoldParkLow >= 12
    /\ Q1175SpecialFoldParkLow <= Q1175FoldParkLow

InputOK(k, a) ==
    IF k.operation \in {"double", "halve", "fold"}
    THEN a \in 0..(P - 1)
    ELSE a \in 0..(Q - 1)

ConstValue(k, o) ==
    IF k.constant_form = "generic" THEN o ELSE C

ControlledDelta(k, o, c) ==
    IF c THEN ConstValue(k, o) ELSE 0

QSub(a, d) == ModQ(a + Q - (d % Q))
PSub(a, d) == ModP(a + P - (d % P))

CoreAdderAction(k, a, o, c) ==
    CASE k.operation = "add" -> ModQ(a + ControlledDelta(k, o, c))
      [] k.operation = "sub" -> QSub(a, ControlledDelta(k, o, c))
      [] OTHER -> a

DoubleAlg(a) ==
    LET raw == 2 * a IN
        ModP((raw % Q) + (IF raw >= Q THEN C ELSE 0))

DoubleSpec(a) == ModP(2 * a)

HalveAlg(a) ==
    IF a % 2 = 0
    THEN a \div 2
    ELSE (a + P) \div 2

HalveSpec(a) == ModP(a * ((P + 1) \div 2))

FoldAction(k, a, o, c) ==
    IF k.operation = "fold"
    THEN
        IF k.constant_form = "signed_sparse_c"
        THEN ModP(a + ControlledDelta(k, o, c))
        ELSE ModP(a + ControlledDelta(k, o, c))
    ELSE a

SpecResult(k, a, o, c) ==
    CASE k.operation \in {"add", "sub"} -> CoreAdderAction(k, a, o, c)
      [] k.operation = "double" -> DoubleSpec(a)
      [] k.operation = "halve" -> HalveSpec(a)
      [] k.operation = "fold" -> FoldAction(k, a, o, c)

ImplResult(k, a, o, c) ==
    CASE k.operation \in {"add", "sub"} -> CoreAdderAction(k, a, o, c)
      [] k.operation = "double" -> DoubleAlg(a)
      [] k.operation = "halve" -> HalveAlg(a)
      [] k.operation = "fold" -> FoldAction(k, a, o, c)

SparseAddGuard(a, d, last) ==
    Low(a, last) + d < Pow2(last + 1)

SparseSubGuard(a, d, last) ==
    Low(a, last) >= d

TailGuard(k, a, o, c) ==
    IF ~(k.carry_model \in {"truncated_window", "guarded_tail"})
    THEN TRUE
    ELSE
        CASE k.operation = "double" ->
                LET raw == 2 * a IN
                    SparseAddGuard(raw % Q, IF raw >= Q THEN C ELSE 0, LastCarryBit)
          [] k.operation = "halve" ->
                SparseSubGuard(a, IF a % 2 = 1 THEN C ELSE 0, LastCarryBit)
          [] k.operation = "sub" ->
                SparseSubGuard(a, ControlledDelta(k, o, c), LastCarryBit)
          [] OTHER ->
                SparseAddGuard(a, ControlledDelta(k, o, c), LastCarryBit)

OwnedNeed(k) ==
    CASE k.control_model = "materialized" -> N
      [] k.control_model = "control_by_prep" -> CHi + 1
      [] OTHER -> 0

BorrowedNeed(k) ==
    CASE k.host_model = "borrowed" ->
            LastCarryBit + 1 + (IF Q1175BorrowFutureBoundary THEN 3 ELSE 0)
      [] k.host_model = "streamed" ->
            1 + (IF Q1175StreamFoldControls THEN 1 ELSE 0)
      [] k.host_model = "derived_control_hosted" ->
            4 + (IF Q1175StreamFoldControls THEN 1 ELSE 0)
      [] OTHER -> 0

MeasuredNeed(k) ==
    IF k.cleanup_model \in {"measured_hmr", "phase_corrected"} THEN 1 ELSE 0

PhaseDebtAdded(k) ==
    IF k.cleanup_model \in {"measured_hmr", "phase_corrected"} THEN 1 ELSE 0

MeasurementPredicateKnown(k) ==
    /\ k.cleanup_model \in {"measured_hmr", "phase_corrected"} => measured > 0
    /\ k.host_model = "derived_control_hosted" => borrowed > 0

Init ==
    /\ pc = "Ready"
    /\ key = TargetKey
    /\ acc0 \in 0..(Q - 1)
    /\ operand0 \in 0..(Q - 1)
    /\ ctrl0 \in BOOLEAN
    /\ InputOK(key, acc0)
    /\ acc = acc0
    /\ operand = operand0
    /\ ctrl = ctrl0
    /\ owned = 0
    /\ borrowed = 0
    /\ streamed = FALSE
    /\ phaseDebt = 0
    /\ measured = 0
    /\ peak = 0
    /\ dirtyQoffset = FALSE
    /\ boundaryBorrowed = 0
    /\ futureBoundaryBorrowed = 0
    /\ hostedFoldControls = FALSE

Prepare ==
    /\ pc = "Ready"
    /\ LET own == OwnedNeed(key)
           bor == BorrowedNeed(key)
       IN
           /\ pc' = "Prepared"
           /\ owned' = own
           /\ borrowed' = bor
           /\ streamed' = (key.host_model = "streamed")
           /\ dirtyQoffset' = Q1175DirtyQoffset
           /\ boundaryBorrowed' = IF Q1175BorrowCleanedBoundary THEN 1 ELSE 0
           /\ futureBoundaryBorrowed' = IF Q1175BorrowFutureBoundary THEN 3 ELSE 0
           /\ hostedFoldControls' = Q1175StreamFoldControls
           /\ peak' = Max(peak, own + bor + (IF Q1175DirtyQoffset THEN 3 ELSE 0))
    /\ UNCHANGED << acc0, operand0, ctrl0, acc, operand, ctrl,
                    key, phaseDebt, measured >>

Ripple ==
    /\ pc = "Prepared"
    /\ TailGuard(key, acc, operand, ctrl)
    /\ Q1175DirtyQoffset =>
           /\ dirtyQoffset
           /\ futureBoundaryBorrowed >= 3
           /\ Q1175ApplyBlocks >= 3
    /\ Q1175BorrowCleanedBoundary => boundaryBorrowed <= measured + 1
    /\ Q1175StreamFoldControls =>
           /\ hostedFoldControls
           /\ borrowed > 0
    /\ pc' = "RippleDone"
    /\ acc' = ImplResult(key, acc, operand, ctrl)
    /\ measured' = measured + MeasuredNeed(key)
    /\ phaseDebt' = phaseDebt + PhaseDebtAdded(key)
    /\ UNCHANGED << acc0, operand0, ctrl0, operand, ctrl,
                    key, owned, borrowed, streamed, peak,
                    dirtyQoffset, boundaryBorrowed,
                    futureBoundaryBorrowed, hostedFoldControls >>

Cleanup ==
    /\ pc = "RippleDone"
    /\ MeasurementPredicateKnown(key)
    /\ pc' = "Done"
    /\ owned' = 0
    /\ borrowed' = 0
    /\ streamed' = FALSE
    /\ phaseDebt' = 0
    /\ dirtyQoffset' = FALSE
    /\ boundaryBorrowed' = 0
    /\ futureBoundaryBorrowed' = 0
    /\ hostedFoldControls' = FALSE
    /\ UNCHANGED << acc0, operand0, ctrl0, acc, operand, ctrl,
                    key, measured, peak >>

Done ==
    /\ pc = "Done"
    /\ UNCHANGED vars

Next == Prepare \/ Ripple \/ Cleanup \/ Done
Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ pc \in PCs
    /\ KeyOK(key)
    /\ acc0 \in 0..(Q - 1)
    /\ operand0 \in 0..(Q - 1)
    /\ ctrl0 \in BOOLEAN
    /\ acc \in 0..(Q - 1)
    /\ operand \in 0..(Q - 1)
    /\ ctrl \in BOOLEAN
    /\ owned \in Nat
    /\ borrowed \in Nat
    /\ streamed \in BOOLEAN
    /\ phaseDebt \in Nat
    /\ measured \in Nat
    /\ peak \in Nat
    /\ dirtyQoffset \in BOOLEAN
    /\ boundaryBorrowed \in Nat
    /\ futureBoundaryBorrowed \in Nat
    /\ hostedFoldControls \in BOOLEAN

ValueCorrect ==
    pc = "Done" => acc = SpecResult(key, acc0, operand0, ctrl0)

ScratchCleanWhenDone ==
    pc = "Done" => /\ owned = 0 /\ borrowed = 0 /\ streamed = FALSE /\ phaseDebt = 0

PeakEnvelope ==
    peak <= PeakLimit

GuardedProgress ==
    pc = "Prepared" => TailGuard(key, acc, operand, ctrl)

Q1175LifecycleSafe ==
    /\ dirtyQoffset => /\ futureBoundaryBorrowed >= 3
                       /\ Q1175DirtyQoffset
    /\ boundaryBorrowed > 0 => Q1175BorrowCleanedBoundary
    /\ hostedFoldControls => /\ Q1175StreamFoldControls
                             /\ borrowed > 0
    /\ pc = "Done" =>
        /\ dirtyQoffset = FALSE
        /\ boundaryBorrowed = 0
        /\ futureBoundaryBorrowed = 0
        /\ hostedFoldControls = FALSE

THEOREM Spec => []TypeOK

====
