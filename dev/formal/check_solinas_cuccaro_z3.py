"""Bounded Z3 obligations for the point_add Solinas/Cuccaro adder model.

The TLA+ spec owns lifecycle/workflow safety. These checks cover the small
bit-vector obligations that the TLA+ model treats as atomic facts:

- sparse-c double equals multiplication by two modulo p,
- halve is the inverse action modulo p,
- materialized/direct/control-by-prep constant add/sub agree by value,
- carry-window guards are sufficient for truncated sparse-c add/sub,
- measured cleanup phase feedback cancels the HMR phase when the predicate is
  the original Boolean expression.

The width is deliberately bounded. It uses a toy sparse pseudo-Mersenne
constant C = 2^8 + 2^4 + 1 so the bit-vector queries stay fast; the full
secp256k1 constants and all-input facts are named in SolinasCuccaroFacts.lean.

The q1175 checks additionally symbolically execute the Boolean gate skeleton of
the dirty-qoffset helper used by the 1175q route. That keeps this file useful as
a regression guard: if the carry-q or borrowed-dirty recurrence is modeled
incorrectly, Z3 returns a concrete counterexample.
"""

from __future__ import annotations

import sys

from z3 import (
    And,
    BitVec,
    BitVecVal,
    Bool,
    BoolVal,
    If,
    Not,
    Or,
    Solver,
    UDiv,
    UGE,
    ULT,
    URem,
    Xor,
    sat,
    unsat,
)


N = 16
WIDTH = 32
Q = 1 << N
C = (1 << 8) + (1 << 4) + 1
P = Q - C
HI = C.bit_length() - 1
WINDOW = 4
LAST = HI + WINDOW
M = 1 << (LAST + 1)
INV2_MOD_P = (P + 1) // 2


def bv(value: int):
    return BitVecVal(value, WIDTH)


ZERO = bv(0)
ONE = bv(1)
TWO = bv(2)
Q_BV = bv(Q)
C_BV = bv(C)
P_BV = bv(P)
M_BV = bv(M)
INV2_BV = bv(INV2_MOD_P)
QOFFSET_N = 6
SOLVER_TIMEOUT_MS = 60_000


def prove_unsat(name: str, assumptions, bad):
    solver = Solver()
    solver.set(timeout=SOLVER_TIMEOUT_MS)
    solver.add(*assumptions)
    solver.add(bad)
    result = solver.check()
    if result == sat:
        print(f"FAIL {name}")
        print(solver.model())
        return False
    if result != unsat:
        print(f"FAIL {name}: {result}")
        return False
    print(f"ok {name}: {result}")
    return True


def bools(prefix: str, n: int):
    return [Bool(f"{prefix}_{i}") for i in range(n)]


def bits_to_bv(bits, width: int):
    result = BitVecVal(0, width)
    for idx, bit in enumerate(bits):
        result = result + If(bit, BitVecVal(1 << idx, width), BitVecVal(0, width))
    return result


class BoolCircuit:
    def __init__(self, initial):
        self.state = dict(initial)

    def x(self, q: str):
        self.state[q] = Not(self.state[q])

    def cx(self, ctrl: str, target: str):
        self.state[target] = Xor(self.state[target], self.state[ctrl])

    def ccx(self, ctrl_a: str, ctrl_b: str, target: str):
        self.state[target] = Xor(
            self.state[target],
            And(self.state[ctrl_a], self.state[ctrl_b]),
        )

    def reset(self, q: str):
        self.state[q] = BoolVal(False)


def simulate_add_vented_qoffset(
    circuit: BoolCircuit,
    target,
    clean2,
    offset,
    carry_in: bool,
    carry_q,
    carry_xor_target,
):
    n = len(target)
    assert n >= 5

    for k in range(n):
        circuit.cx(offset[k], target[k])

    def carry_key(k: int):
        if k == 0:
            return None
        if k == n - 1:
            return target[n - 1]
        return clean2[k % 2]

    for k in range(n - 1):
        if k < n - 2:
            circuit.reset(carry_key(k + 1))

        if k == 0:
            next_q = carry_key(1)
            if carry_q is not None:
                circuit.cx(carry_q, offset[0])
            if carry_in:
                circuit.x(offset[0])
            circuit.ccx(target[0], offset[0], next_q)
            if carry_in:
                circuit.x(offset[0])
            if carry_q is not None:
                circuit.cx(carry_q, offset[0])
        else:
            cur = carry_key(k)
            next_q = carry_key(k + 1)
            circuit.cx(offset[k], cur)
            circuit.ccx(target[k], cur, next_q)
            circuit.cx(offset[k], cur)

        if k == 0:
            if carry_in:
                circuit.x(target[0])
            if carry_q is not None:
                circuit.cx(carry_q, target[0])
        else:
            cur = carry_key(k)
            circuit.cx(cur, target[k])

        if k < len(carry_xor_target) and carry_xor_target[k] is not None:
            if k == 0:
                if carry_in:
                    circuit.x(carry_xor_target[k])
                if carry_q is not None:
                    circuit.cx(carry_q, carry_xor_target[k])
            else:
                circuit.cx(carry_key(k), carry_xor_target[k])

        if k > 0:
            circuit.reset(carry_key(k))

        next_q = carry_key(k + 1)
        if next_q is not None:
            circuit.cx(offset[k], next_q)


def simulate_xor_right_shifted_carries_qoffset(
    circuit: BoolCircuit,
    q_src,
    q_offset,
    q_dst,
    carry_in: bool,
    carry_q,
):
    n = len(q_dst)
    assert n <= len(q_src) <= n + 1

    def ccx_with_qxor(ctrl_a, xor_a, ctrl_b, xor_b, target):
        if xor_a is not None:
            circuit.cx(xor_a, ctrl_a)
        if xor_b is not None:
            circuit.cx(xor_b, ctrl_b)
        circuit.ccx(ctrl_a, ctrl_b, target)
        if xor_b is not None:
            circuit.cx(xor_b, ctrl_b)
        if xor_a is not None:
            circuit.cx(xor_a, ctrl_a)

    for k in range(n - 1, 0, -1):
        ccx_with_qxor(q_src[k], q_offset[k], q_dst[k - 1], None, q_dst[k])

    for k in range(n):
        circuit.cx(q_offset[k], q_dst[k])

    circuit.cx(q_offset[0], q_src[0])
    if carry_q is not None:
        circuit.cx(carry_q, q_offset[0])
    if carry_in:
        circuit.x(q_offset[0])
    circuit.ccx(q_src[0], q_offset[0], q_dst[0])
    if carry_in:
        circuit.x(q_offset[0])
    if carry_q is not None:
        circuit.cx(carry_q, q_offset[0])
    circuit.cx(q_offset[0], q_src[0])

    for k in range(1, n):
        ccx_with_qxor(q_src[k], q_offset[k], q_dst[k - 1], q_offset[k], q_dst[k])


def simulate_iadd_dirty_qoffset(circuit, target, dirty, clean2, offset, carry_in, carry_q):
    n = len(target)
    carry_xor_target = [None] + dirty[:] + [None]
    simulate_add_vented_qoffset(
        circuit,
        target,
        clean2,
        offset,
        carry_in,
        carry_q,
        carry_xor_target,
    )

    for q in target:
        circuit.x(q)
    simulate_xor_right_shifted_carries_qoffset(
        circuit,
        target[: n - 1],
        offset,
        dirty,
        carry_in,
        carry_q,
    )
    for q in target:
        circuit.x(q)


def q1175_dirty_qoffset_obligation(name: str, *, subtract: bool, carry_q_enabled: bool):
    n = QOFFSET_N
    target = [f"{name}_t_{idx}" for idx in range(n)]
    offset = [f"{name}_o_{idx}" for idx in range(n)]
    dirty = [f"{name}_d_{idx}" for idx in range(n - 2)]
    clean2 = [f"{name}_clean0", f"{name}_clean1"]
    carry_q = f"{name}_carry_q" if carry_q_enabled else None

    target0 = bools(f"{name}_target", n)
    offset0 = bools(f"{name}_offset", n)
    dirty0 = bools(f"{name}_dirty", n - 2)
    initial = {key: bit for key, bit in zip(target, target0)}
    initial.update({key: bit for key, bit in zip(offset, offset0)})
    initial.update({key: bit for key, bit in zip(dirty, dirty0)})
    initial[clean2[0]] = BoolVal(False)
    initial[clean2[1]] = BoolVal(False)
    if carry_q is not None:
        initial[carry_q] = Bool(f"{name}_carry_q_in")

    circuit = BoolCircuit(initial)
    if subtract:
        for q in target:
            circuit.x(q)
    simulate_iadd_dirty_qoffset(
        circuit,
        target,
        dirty,
        clean2,
        offset,
        False,
        carry_q,
    )
    if subtract:
        for q in target:
            circuit.x(q)

    target_in = bits_to_bv(target0, n)
    offset_in = bits_to_bv(offset0, n)
    final_target = bits_to_bv([circuit.state[key] for key in target], n)
    carry = (
        If(circuit.state[carry_q], BitVecVal(1, n), BitVecVal(0, n))
        if carry_q is not None
        else BitVecVal(0, n)
    )
    expected = (
        target_in - offset_in - carry
        if subtract
        else target_in + offset_in + carry
    )

    restoration = []
    restoration.extend(circuit.state[key] != bit for key, bit in zip(offset, offset0))
    restoration.extend(circuit.state[key] != bit for key, bit in zip(dirty, dirty0))
    restoration.extend(circuit.state[key] != BoolVal(False) for key in clean2)
    if carry_q is not None:
        restoration.append(circuit.state[carry_q] != initial[carry_q])

    return prove_unsat(
        name,
        [],
        Or(final_target != expected, *restoration),
    )


def main() -> int:
    acc = BitVec("acc", WIDTH)
    src = BitVec("src", WIDTH)
    x = BitVec("x", WIDTH)
    ctrl = Bool("ctrl")

    k = If(ctrl, C_BV, ZERO)
    low = URem(acc, M_BV)

    double_raw = x + x
    double_alg = URem(URem(double_raw, Q_BV) + If(UGE(double_raw, Q_BV), C_BV, ZERO), P_BV)
    double_spec = URem(double_raw, P_BV)

    odd = (x & ONE) == ONE
    halve_alg = UDiv(If(odd, x + P_BV, x), TWO)
    halve_spec = URem(x * INV2_BV, P_BV)

    full_add = URem(acc + k, Q_BV)
    trunc_add = (acc - low) + URem(low + k, M_BV)
    add_guard = Or(Not(ctrl), ULT(low + C_BV, M_BV))

    full_sub = URem(acc + Q_BV - k, Q_BV)
    trunc_sub = (acc - low) + URem(low + M_BV - k, M_BV)
    sub_guard = Or(Not(ctrl), UGE(low, C_BV))

    materialized_add = URem(acc + If(ctrl, C_BV, ZERO), Q_BV)
    direct_add = URem(acc + k, Q_BV)
    prep_add = URem(acc + k, Q_BV)

    materialized_sub = URem(acc + Q_BV - If(ctrl, C_BV, ZERO), Q_BV)
    direct_sub = URem(acc + Q_BV - k, Q_BV)
    prep_sub = URem(acc + Q_BV - k, Q_BV)

    scratch_after_unprep = k ^ k

    a = Bool("a")
    b = Bool("b")
    m = Bool("m")
    anc = Bool("anc")
    hmr_phase = And(anc, m)
    feedback_phase = And(And(a, b), m)

    checks = [
        (
            "toy sparse-c signed identity",
            [],
            C_BV
            != (
                bv(1 << 8)
                + bv(1 << 4)
                + bv(1)
            ),
        ),
        (
            "double sparse-c action",
            [ULT(x, P_BV)],
            double_alg != double_spec,
        ),
        (
            "halve sparse-c action",
            [ULT(x, P_BV)],
            halve_alg != halve_spec,
        ),
        (
            "materialized direct prep add agree",
            [ULT(acc, Q_BV)],
            Or(materialized_add != direct_add, direct_add != prep_add),
        ),
        (
            "materialized direct prep sub agree",
            [ULT(acc, Q_BV)],
            Or(materialized_sub != direct_sub, direct_sub != prep_sub),
        ),
        (
            "control-by-prep scratch unpreps clean",
            [],
            scratch_after_unprep != ZERO,
        ),
        (
            "guarded truncated sparse add equals full add",
            [ULT(acc, Q_BV), add_guard],
            trunc_add != full_add,
        ),
        (
            "guarded truncated sparse sub equals full sub",
            [ULT(acc, Q_BV), sub_guard],
            trunc_sub != full_sub,
        ),
        (
            "known-zero carry-in specialization",
            [ULT(acc, Q_BV), ULT(src, Q_BV)],
            URem(acc + src + ZERO, Q_BV) != URem(acc + src, Q_BV),
        ),
        (
            "measured HMR phase feedback cancels",
            [anc == And(a, b)],
            Xor(hmr_phase, feedback_phase) != BoolVal(False),
        ),
    ]

    ok = True
    for name, assumptions, bad in checks:
        ok &= prove_unsat(name, assumptions, bad)
    ok &= q1175_dirty_qoffset_obligation(
        "q1175 dirty qoffset add restores dirty and equals full add",
        subtract=False,
        carry_q_enabled=False,
    )
    ok &= q1175_dirty_qoffset_obligation(
        "q1175 dirty qoffset add carry-q equals full add",
        subtract=False,
        carry_q_enabled=True,
    )
    ok &= q1175_dirty_qoffset_obligation(
        "q1175 dirty qoffset sub restores dirty and equals full sub",
        subtract=True,
        carry_q_enabled=False,
    )
    ok &= q1175_dirty_qoffset_obligation(
        "q1175 dirty qoffset sub borrow-q equals full sub",
        subtract=True,
        carry_q_enabled=True,
    )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
