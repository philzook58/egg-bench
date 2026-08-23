;; generated from circuit file benchmarks/circuit/example.aag
;; outputs: out
(sort BoolExpr)

(function True () BoolExpr)
(function False () BoolExpr)
(function Var (String) BoolExpr)
(function Not (BoolExpr) BoolExpr)
(function And (BoolExpr BoolExpr) BoolExpr)
(function Or (BoolExpr BoolExpr) BoolExpr)
(function Out (String BoolExpr) BoolExpr)

;; Boolean simplification rules.
(rewrite and.true.left (And True x) x)
(rewrite and.true.right (And x True) x)
(rewrite and.false.left (And False x) False)
(rewrite and.false.right (And x False) False)
(rewrite or.false.left (Or False x) x)
(rewrite or.false.right (Or x False) x)
(rewrite or.true.left (Or True x) True)
(rewrite or.true.right (Or x True) True)
(rewrite not.true (Not True) False)
(rewrite not.false (Not False) True)
(rewrite not.double (Not (Not x)) x)

(optimize (Out "out" (And (Var "a") (Var "b"))))
