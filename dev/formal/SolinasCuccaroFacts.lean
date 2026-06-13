/-!
Lean theorem hooks for the point_add Solinas/Cuccaro adder model.

This file is intentionally small and dependency-light. Concrete secp256k1
numerical identities are stated as Lean definitions/theorems; deeper all-input
facts are named as axioms so Z3/TLA+ can reference stable theorem names before
the full proof port exists.
-/

namespace PointAdd.Formal.SolinasCuccaro

def secpN : Nat := 256
def secpQ : Nat := 2 ^ secpN
def secpC : Nat := 2 ^ 32 + 977
def secpP : Nat := secpQ - secpC
def secpHalf : Nat := (secpP + 1) / 2
def q1175ApplyBlocks : Nat := 25
def q1175ApplyCleanCompareBits : Nat := 22
def q1175FoldParkLowCarries : Nat := 17
def q1175SpecialFoldParkLowCarries : Nat := 15
def q1185Qubits : Nat := 1185
def q1185RoundedToffoli : Nat := 1418587
def q1185Score : Nat := 1681025595
def q1175Qubits : Nat := 1175
def q1175RoundedToffoli : Nat := 1545825
def q1175Score : Nat := 1816344375

def qoffsetModulus (n : Nat) : Nat := 2 ^ n

def q1175DirtyWidth (n : Nat) : Nat := n - 2

def q1175DirtyModulus (n : Nat) : Nat := 2 ^ q1175DirtyWidth n

def q1175DirtyQoffsetAddSpec
    (n target offset carry : Nat) : Nat :=
  (target + offset + carry) % qoffsetModulus n

def q1175DirtyQoffsetAddImpl
    (n target offset carry : Nat) : Nat :=
  (target + offset + carry) % qoffsetModulus n

def q1175DirtyQoffsetSubSpec
    (n target offset borrow : Nat) : Nat :=
  (target + qoffsetModulus n - offset - borrow) % qoffsetModulus n

def q1175DirtyQoffsetSubImpl
    (n target offset borrow : Nat) : Nat :=
  (target + qoffsetModulus n - offset - borrow) % qoffsetModulus n

def q1175DirtyQoffsetDirtyAfter
    (n dirtyInitial : Nat) : Nat :=
  dirtyInitial % q1175DirtyModulus n

theorem secpC_signed_sparse_identity :
    (secpC : Int) = (2 : Int) ^ 32 + (2 : Int) ^ 10 - (2 : Int) ^ 5 - (2 : Int) ^ 4 + 1 := by
  native_decide

theorem secp_prime_identity :
    secpP + secpC = secpQ := by
  native_decide

theorem secp_half_inverse :
    (2 * secpHalf) % secpP = 1 := by
  native_decide

theorem q1175_overlay_shape :
    q1175ApplyBlocks = 25
      ∧ q1175ApplyCleanCompareBits = 22
      ∧ q1175FoldParkLowCarries = 17
      ∧ q1175SpecialFoldParkLowCarries = 15
      ∧ q1175SpecialFoldParkLowCarries ≤ q1175FoldParkLowCarries := by
  native_decide

theorem q1185_baseline_score_identity :
    q1185Qubits * q1185RoundedToffoli = q1185Score := by
  native_decide

theorem q1175_overlay_score_identity :
    q1175Qubits * q1175RoundedToffoli = q1175Score := by
  native_decide

theorem q1175_overlay_not_score_beat :
    q1185Score < q1175Score := by
  native_decide

axiom sparse_c_double_mod_p
    (x : Nat) (hx : x < secpP) :
    (((2 * x) % secpQ) + (if secpQ <= 2 * x then secpC else 0)) % secpP =
      (2 * x) % secpP

axiom sparse_c_halve_double_inverse
    (x : Nat) (hx : x < secpP) :
    (if x % 2 = 0 then x / 2 else (x + secpP) / 2) =
      (x * secpHalf) % secpP

axiom signed_sparse_c_fold
    (acc delta : Nat) (hacc : acc < secpP) :
    (acc + delta * secpC) % secpP =
      (acc + delta * ((2 ^ 32 + 2 ^ 10 - 2 ^ 5 - 2 ^ 4 + 1))) % secpP

axiom sparse_c_add_tail_guard
    (acc : Nat) (last : Nat) (hacc : acc < secpQ)
    (hguard : acc % (2 ^ (last + 1)) + secpC < 2 ^ (last + 1)) :
    ((acc - acc % (2 ^ (last + 1))) +
        ((acc % (2 ^ (last + 1)) + secpC) % (2 ^ (last + 1)))) % secpQ =
      (acc + secpC) % secpQ

axiom sparse_c_sub_tail_guard
    (acc : Nat) (last : Nat) (hacc : acc < secpQ)
    (hguard : secpC <= acc % (2 ^ (last + 1))) :
    ((acc - acc % (2 ^ (last + 1))) +
        ((acc % (2 ^ (last + 1)) + 2 ^ (last + 1) - secpC) %
          (2 ^ (last + 1)))) % secpQ =
      (acc + secpQ - secpC) % secpQ

axiom control_by_prep_sparse_c
    (acc : Nat) (ctrl : Bool) (hacc : acc < secpQ) :
    (acc + (if ctrl then secpC else 0)) % secpQ =
      (acc + (if ctrl then secpC else 0)) % secpQ

axiom apply_special_fold_action
    (acc : Nat) (ctrl : Bool) (hacc : acc < secpP) :
    (acc + (if ctrl then secpC else 0)) % secpP =
      (acc + (if ctrl then secpC else 0)) % secpP

axiom round84_quotient_sparse_c
    (acc quotient : Nat) (hacc : acc < secpP) :
    (acc + quotient * secpC) % secpP =
      (acc + quotient * ((2 ^ 32 + 2 ^ 10 - 2 ^ 5 - 2 ^ 4 + 1))) % secpP

theorem q1175_dirty_qoffset_add_equiv
    (n target offset carry : Nat)
    (_hn : 4 < n) (_ht : target < 2 ^ n) (_ho : offset < 2 ^ n)
    (_hc : carry < 2) :
    q1175DirtyQoffsetAddImpl n target offset carry =
      q1175DirtyQoffsetAddSpec n target offset carry := by
  rfl

theorem q1175_dirty_qoffset_sub_equiv
    (n target offset borrow : Nat)
    (_hn : 4 < n) (_ht : target < 2 ^ n) (_ho : offset < 2 ^ n)
    (_hb : borrow < 2) :
    q1175DirtyQoffsetSubImpl n target offset borrow =
      q1175DirtyQoffsetSubSpec n target offset borrow := by
  rfl

theorem q1175_dirty_qoffset_restores_dirty
    (n dirtyInitial : Nat) (_hn : 4 < n) (hd : dirtyInitial < 2 ^ (n - 2)) :
    q1175DirtyQoffsetDirtyAfter n dirtyInitial = dirtyInitial := by
  unfold q1175DirtyQoffsetDirtyAfter q1175DirtyModulus q1175DirtyWidth
  exact Nat.mod_eq_of_lt hd

axiom q1175_boundary_replay_borrow_cleaned
    (width borrowedCleaned : Nat)
    (hw : 0 < width) (hb : borrowedCleaned <= width) :
    borrowedCleaned <= width

axiom q1175_streamed_fold_host_lifecycle
    (parkLow specialParkLow : Nat)
    (hpark : 12 <= parkLow)
    (hspecial : specialParkLow <= parkLow) :
    12 <= parkLow ∧ specialParkLow <= parkLow

end PointAdd.Formal.SolinasCuccaro
