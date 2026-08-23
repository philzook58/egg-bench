(sort Expr)
(function Var (String) Expr)
(function Lit (String) Expr)
(function Float (String) Expr)
(function Ptr (String) Expr)
(function Int (i64) Expr)
(function Bool (i64) Expr)
(function Mem0 () Expr)
(function Unit () Expr)
(function Null () Expr)
(function Nil () Expr)
(function MissingArg () Expr)
(function Cons (Expr Expr) Expr)
(function Ret (Expr Expr) Expr)
(function Assume (Expr Expr) Expr)
(function Print (Expr Expr) Expr)
(function Alloc (Expr Expr Expr) Expr)
(function Store (Expr Expr Expr) Expr)
(function Load (Expr Expr) Expr)
(function Free (Expr Expr) Expr)
(function CallRet (String Expr Expr) Expr)
(function CallMem (String Expr Expr) Expr)
(function Op (String Expr) Expr)
(function Add (Expr Expr) Expr)
(function And (Expr Expr) Expr)
(function CEq (Expr Expr) Expr)
(function CGe (Expr Expr) Expr)
(function CGt (Expr Expr) Expr)
(function CLe (Expr Expr) Expr)
(function CLt (Expr Expr) Expr)
(function Char2Int (Expr) Expr)
(function Div (Expr Expr) Expr)
(function Eq (Expr Expr) Expr)
(function FAdd (Expr Expr) Expr)
(function FDiv (Expr Expr) Expr)
(function FEq (Expr Expr) Expr)
(function FGe (Expr Expr) Expr)
(function FGt (Expr Expr) Expr)
(function FLe (Expr Expr) Expr)
(function FLt (Expr Expr) Expr)
(function FMul (Expr Expr) Expr)
(function FSub (Expr Expr) Expr)
(function Ge (Expr Expr) Expr)
(function Gt (Expr Expr) Expr)
(function Id (Expr) Expr)
(function Int2Char (Expr) Expr)
(function Le (Expr Expr) Expr)
(function Lt (Expr Expr) Expr)
(function Mul (Expr Expr) Expr)
(function Not (Expr) Expr)
(function Or (Expr Expr) Expr)
(function PtrAdd (Expr Expr) Expr)
(function Sub (Expr Expr) Expr)

;; benchmarks/cfg/bril/benchmarks/core/ackermann.bril:main, 2 traced instructions
(optimize (Ret (Print (CallMem "ack" (Mem0) (Cons (Var "m") (Cons (Var "n") (Nil)))) (Cons (CallRet "ack" (Mem0) (Cons (Var "m") (Cons (Var "n") (Nil)))) (Nil))) (CallRet "ack" (Mem0) (Cons (Var "m") (Cons (Var "n") (Nil))))))

;; benchmarks/cfg/bril/benchmarks/mem/cordic.bril:main, 16 traced instructions
(optimize (Ret (Print (CallMem "cordic" (Mem0) (Cons (Id (Id (FDiv (FMul (Id (Id (Int 50))) (Id (Id (Float "3.141592653589793")))) (Int 180)))) (Nil))) (Cons (Id (Id (CallRet "cordic" (Mem0) (Cons (Id (Id (FDiv (FMul (Id (Id (Int 50))) (Id (Id (Float "3.141592653589793")))) (Int 180)))) (Nil))))) (Nil))) (Id (Id (CallRet "cordic" (Mem0) (Cons (Id (Id (FDiv (FMul (Id (Id (Int 50))) (Id (Id (Float "3.141592653589793")))) (Int 180)))) (Nil)))))))

;; benchmarks/cfg/bril/benchmarks/core/sum-divisors.bril:main, 25 traced instructions
(optimize (Ret (Print (Assume (Assume (Print (Assume (CallMem "mod" (Assume (Assume (Mem0) (Lt (Var "n") (Int 0))) (Not (Gt (Mul (Add (Int 0) (Int 1)) (Add (Int 0) (Int 1))) (Mul (Var "n") (Int -1))))) (Cons (Mul (Var "n") (Int -1)) (Cons (Add (Int 0) (Int 1)) (Nil)))) (Eq (CallRet "mod" (Assume (Assume (Mem0) (Lt (Var "n") (Int 0))) (Not (Gt (Mul (Add (Int 0) (Int 1)) (Add (Int 0) (Int 1))) (Mul (Var "n") (Int -1))))) (Cons (Mul (Var "n") (Int -1)) (Cons (Add (Int 0) (Int 1)) (Nil)))) (Int 0))) (Cons (Add (Int 0) (Int 1)) (Nil))) (Eq (Div (Mul (Var "n") (Int -1)) (Add (Int 0) (Int 1))) (Add (Int 0) (Int 1)))) (Gt (Mul (Add (Add (Int 0) (Int 1)) (Int 1)) (Add (Add (Int 0) (Int 1)) (Int 1))) (Mul (Var "n") (Int -1)))) (Cons (Add (Int 0) (Add (Int 0) (Int 1))) (Nil))) (Add (Int 0) (Add (Int 0) (Int 1)))))

;; benchmarks/cfg/bril/benchmarks/mem/char-poly.bril:main, 7 traced instructions
(optimize (Ret (Free (CallMem "print_array" (CallMem "char_poly" (CallMem "pack" (Mem0) (Cons (Int 9) (Cons (Var "n1") (Cons (Var "n2") (Cons (Var "n3") (Cons (Var "n4") (Cons (Var "n5") (Cons (Var "n6") (Cons (Var "n7") (Cons (Var "n8") (Cons (Var "n9") (Nil)))))))))))) (Cons (CallRet "pack" (Mem0) (Cons (Int 9) (Cons (Var "n1") (Cons (Var "n2") (Cons (Var "n3") (Cons (Var "n4") (Cons (Var "n5") (Cons (Var "n6") (Cons (Var "n7") (Cons (Var "n8") (Cons (Var "n9") (Nil)))))))))))) (Nil))) (Cons (CallRet "char_poly" (CallMem "pack" (Mem0) (Cons (Int 9) (Cons (Var "n1") (Cons (Var "n2") (Cons (Var "n3") (Cons (Var "n4") (Cons (Var "n5") (Cons (Var "n6") (Cons (Var "n7") (Cons (Var "n8") (Cons (Var "n9") (Nil)))))))))))) (Cons (CallRet "pack" (Mem0) (Cons (Int 9) (Cons (Var "n1") (Cons (Var "n2") (Cons (Var "n3") (Cons (Var "n4") (Cons (Var "n5") (Cons (Var "n6") (Cons (Var "n7") (Cons (Var "n8") (Cons (Var "n9") (Nil)))))))))))) (Nil))) (Cons (Int 4) (Nil)))) (CallRet "char_poly" (CallMem "pack" (Mem0) (Cons (Int 9) (Cons (Var "n1") (Cons (Var "n2") (Cons (Var "n3") (Cons (Var "n4") (Cons (Var "n5") (Cons (Var "n6") (Cons (Var "n7") (Cons (Var "n8") (Cons (Var "n9") (Nil)))))))))))) (Cons (CallRet "pack" (Mem0) (Cons (Int 9) (Cons (Var "n1") (Cons (Var "n2") (Cons (Var "n3") (Cons (Var "n4") (Cons (Var "n5") (Cons (Var "n6") (Cons (Var "n7") (Cons (Var "n8") (Cons (Var "n9") (Nil)))))))))))) (Nil)))) (Unit)))

;; benchmarks/cfg/bril/benchmarks/core/sum-sq-diff.bril:main, 14 traced instructions
(optimize (Ret (Print (CallMem "squareOfSum" (CallMem "sumOfSquares" (Mem0) (Cons (Id (Var "n")) (Nil))) (Cons (Id (Var "n")) (Nil))) (Cons (Id (Id (Sub (Id (Id (CallRet "squareOfSum" (CallMem "sumOfSquares" (Mem0) (Cons (Id (Var "n")) (Nil))) (Cons (Id (Var "n")) (Nil))))) (Id (Id (CallRet "sumOfSquares" (Mem0) (Cons (Id (Var "n")) (Nil)))))))) (Nil))) (Id (Id (Sub (Id (Id (CallRet "squareOfSum" (CallMem "sumOfSquares" (Mem0) (Cons (Id (Var "n")) (Nil))) (Cons (Id (Var "n")) (Nil))))) (Id (Id (CallRet "sumOfSquares" (Mem0) (Cons (Id (Var "n")) (Nil))))))))))

;; benchmarks/cfg/bril/benchmarks/mem/adler32.bril:main, 6 traced instructions
(optimize (Ret (Free (Print (CallMem "adler32" (CallMem "fill_array" (Alloc (Mem0) (Ptr "a0") (Int 512)) (Cons (Ptr "a0") (Cons (Int 512) (Nil)))) (Cons (Ptr "a0") (Cons (Int 512) (Nil)))) (Cons (CallRet "adler32" (CallMem "fill_array" (Alloc (Mem0) (Ptr "a0") (Int 512)) (Cons (Ptr "a0") (Cons (Int 512) (Nil)))) (Cons (Ptr "a0") (Cons (Int 512) (Nil)))) (Nil))) (Ptr "a0")) (CallRet "adler32" (CallMem "fill_array" (Alloc (Mem0) (Ptr "a0") (Int 512)) (Cons (Ptr "a0") (Cons (Int 512) (Nil)))) (Cons (Ptr "a0") (Cons (Int 512) (Nil))))))

;; benchmarks/cfg/bril/benchmarks/core/armstrong.bril:main, 23 traced instructions
(optimize (Ret (Print (Assume (CallMem "power" (CallMem "mod" (Assume (CallMem "power" (CallMem "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil)))) (Gt (Div (Id (Var "input")) (Int 10)) (Int 0))) (Cons (Div (Id (Var "input")) (Int 10)) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "power" (CallMem "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil)))) (Gt (Div (Id (Var "input")) (Int 10)) (Int 0))) (Cons (Div (Id (Var "input")) (Int 10)) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil)))) (Not (Gt (Div (Div (Id (Var "input")) (Int 10)) (Int 10)) (Int 0)))) (Cons (Eq (Var "input") (Add (Add (Int 0) (CallRet "power" (CallMem "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil))))) (CallRet "power" (CallMem "mod" (Assume (CallMem "power" (CallMem "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil)))) (Gt (Div (Id (Var "input")) (Int 10)) (Int 0))) (Cons (Div (Id (Var "input")) (Int 10)) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "power" (CallMem "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil)))) (Gt (Div (Id (Var "input")) (Int 10)) (Int 0))) (Cons (Div (Id (Var "input")) (Int 10)) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil)))))) (Nil))) (Eq (Var "input") (Add (Add (Int 0) (CallRet "power" (CallMem "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil))))) (CallRet "power" (CallMem "mod" (Assume (CallMem "power" (CallMem "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil)))) (Gt (Div (Id (Var "input")) (Int 10)) (Int 0))) (Cons (Div (Id (Var "input")) (Int 10)) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "power" (CallMem "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "mod" (Assume (CallMem "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Gt (Id (Var "input")) (Int 0))) (Cons (Id (Var "input")) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil)))) (Gt (Div (Id (Var "input")) (Int 10)) (Int 0))) (Cons (Div (Id (Var "input")) (Int 10)) (Cons (Int 10) (Nil)))) (Cons (CallRet "getDigits" (Mem0) (Cons (Var "input") (Nil))) (Nil))))))))

;; benchmarks/cfg/bril/benchmarks/core/binpow.bril:main, 3 traced instructions
(optimize (Ret (Print (CallMem "bin_pow" (Mem0) (Cons (Var "x") (Cons (Var "n") (Cons (Int 1) (Nil))))) (Cons (CallRet "bin_pow" (Mem0) (Cons (Var "x") (Cons (Var "n") (Cons (Int 1) (Nil))))) (Nil))) (CallRet "bin_pow" (Mem0) (Cons (Var "x") (Cons (Var "n") (Cons (Int 1) (Nil)))))))
