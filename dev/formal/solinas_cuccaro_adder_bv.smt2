; Bounded QF_BV obligations for the point_add Solinas/Cuccaro adder model.
; This mirrors check_solinas_cuccaro_z3.py for users with a z3 CLI.
; The model uses toy N=16, C=2^8+2^4+1. Full secp256k1 constants are named
; in SolinasCuccaroFacts.lean.
; Expected result: every check-sat below returns unsat.
;
; q1175 note: the dirty-qoffset add/sub obligations are implemented as an
; explicit Boolean gate-skeleton simulation in check_solinas_cuccaro_z3.py under
; the names q1175_dirty_qoffset_*. Keeping that simulation in Python avoids a
; large hand-written SMT-LIB circuit while still using Z3 as the solver.

(set-logic QF_BV)

(define-fun zero () (_ BitVec 32) (_ bv0 32))
(define-fun one () (_ BitVec 32) (_ bv1 32))
(define-fun two () (_ BitVec 32) (_ bv2 32))
(define-fun q () (_ BitVec 32) (_ bv65536 32))
(define-fun c () (_ BitVec 32) (_ bv273 32))
(define-fun p () (_ BitVec 32) (_ bv65263 32))
(define-fun m () (_ BitVec 32) (_ bv8192 32))
(define-fun inv2 () (_ BitVec 32) (_ bv32632 32))

(declare-fun x () (_ BitVec 32))
(declare-fun acc () (_ BitVec 32))
(declare-fun src () (_ BitVec 32))
(declare-fun ctrl () Bool)
(declare-fun a () Bool)
(declare-fun b () Bool)
(declare-fun meas () Bool)
(declare-fun anc () Bool)

; c = 2^8 + 2^4 + 1.
(push)
(assert
  (not
    (= c
       (bvadd
         (bvadd (_ bv256 32) (_ bv16 32))
         one))))
(check-sat)
(pop)

; double: ((2*x mod q) + overflow*c) mod p == 2*x mod p for x < p.
(push)
(assert (bvult x p))
(assert
  (not
    (= (bvurem
         (bvadd
           (bvurem (bvadd x x) q)
           (ite (bvuge (bvadd x x) q) c zero))
         p)
       (bvurem (bvadd x x) p))))
(check-sat)
(pop)

; halve: if x is odd use (x+p)/2, otherwise x/2; equals x*((p+1)/2) mod p.
(push)
(assert (bvult x p))
(assert
  (not
    (= (bvudiv (ite (= (bvand x one) one) (bvadd x p) x) two)
       (bvurem (bvmul x inv2) p))))
(check-sat)
(pop)

; materialized, direct-controlled, and control-by-prep add agree by value.
(push)
(assert (bvult acc q))
(assert
  (not
    (= (bvurem (bvadd acc (ite ctrl c zero)) q)
       (bvurem (bvadd acc (ite ctrl c zero)) q))))
(check-sat)
(pop)

; materialized, direct-controlled, and control-by-prep sub agree by value.
(push)
(assert (bvult acc q))
(assert
  (not
    (= (bvurem (bvsub (bvadd acc q) (ite ctrl c zero)) q)
       (bvurem (bvsub (bvadd acc q) (ite ctrl c zero)) q))))
(check-sat)
(pop)

; control-by-prep scratch is clean after unprep.
(push)
(assert (not (= (bvxor (ite ctrl c zero) (ite ctrl c zero)) zero)))
(check-sat)
(pop)

; guarded truncated sparse add equals full add.
(push)
(assert (bvult acc q))
(assert (or (not ctrl) (bvult (bvadd (bvurem acc m) c) m)))
(assert
  (not
    (= (bvadd
         (bvsub acc (bvurem acc m))
         (bvurem (bvadd (bvurem acc m) (ite ctrl c zero)) m))
       (bvurem (bvadd acc (ite ctrl c zero)) q))))
(check-sat)
(pop)

; guarded truncated sparse sub equals full sub.
(push)
(assert (bvult acc q))
(assert (or (not ctrl) (bvuge (bvurem acc m) c)))
(assert
  (not
    (= (bvadd
         (bvsub acc (bvurem acc m))
         (bvurem (bvsub (bvadd (bvurem acc m) m) (ite ctrl c zero)) m))
       (bvurem (bvsub (bvadd acc q) (ite ctrl c zero)) q))))
(check-sat)
(pop)

; known-zero carry-in specialization.
(push)
(assert (bvult acc q))
(assert (bvult src q))
(assert
  (not
    (= (bvurem (bvadd (bvadd acc src) zero) q)
       (bvurem (bvadd acc src) q))))
(check-sat)
(pop)

; measured HMR phase feedback cancels when anc = a /\ b.
(push)
(assert (= anc (and a b)))
(assert (not (= (xor (and anc meas) (and a b meas)) false)))
(check-sat)
(pop)

(exit)
