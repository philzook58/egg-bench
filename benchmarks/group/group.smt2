(declare-sort G)
(declare-fun mul (G G) G)
(declare-fun e () G)
(declare-fun inv (G) G)

(declare-const a G)
(declare-const b G)
(declare-const c G)

(assert (forall ((x G)) (= (mul x e) x)))
(assert (forall ((x G)) (= (mul e x) x)))
(assert (forall ((x G)) (= (mul x (inv x)) e)))
(assert (forall ((x G) (y G) (z G)) (= (mul x (mul y z)) (mul (mul x y) z))))

(define-const start_term G (mul a (inv a)))


(optimize start_term)