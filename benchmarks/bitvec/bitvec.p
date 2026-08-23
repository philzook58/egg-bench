% A small, untyped TPTP CNF axiomatisation for width-polymorphic bit-vectors.
%
% bv(T, N) means that T is a bit-vector of width N. Widths use zero, one,
% add, and sub; lt is their strict ordering. Each clause is Horn so that it
% can also be imported by benchtool's CNF-to-egglog converter.

% Basic arithmetic and ordering facts about widths.
cnf(add_zero_left, axiom, add(zero, N) = N).
cnf(add_zero_right, axiom, add(N, zero) = N).
cnf(add_assoc, axiom, add(add(N, M), K) = add(N, add(M, K))).
cnf(add_comm, axiom, add(N, M) = add(M, N)).
cnf(sub_add_l, axiom, sub(add(N,M),M) = N).
cnf(sub_add_r, axiom, sub(add(N,M),N) = M).

cnf(lt_zero_one, axiom, lt(zero, one)).
cnf(lt_trans, axiom, ~lt(N, M) | ~lt(M, K) | lt(N, K)).
cnf(lt_mono_add_left, axiom, ~lt(N, M) | lt(add(K, N), add(K, M))).
cnf(lt_mono_add_right, axiom, ~lt(N, M) | lt(add(N, K), add(M, K))).

% Width preservation for the operations used below.
cnf(bvnot_size, axiom, ~bv(A, N) | bv(bvnot(A), N)).
cnf(bvand_size, axiom, ~bv(A, N) | ~bv(B, N) | bv(bvand(A, B), N)).
cnf(bvor_size, axiom, ~bv(A, N) | ~bv(B, N) | bv(bvor(A, B), N)).
cnf(bvxor_size, axiom, ~bv(A, N) | ~bv(B, N) | bv(bvxor(A, B), N)).
cnf(bvadd_size, axiom, ~bv(A, N) | ~bv(B, N) | bv(bvadd(A, B), N)).
cnf(bvneg_size, axiom, ~bv(A, N) | bv(bvneg(A), N)).
cnf(bvzero_size, axiom, bv(bvzero(N), N)).

% Same-width Boolean identities.
cnf(bvand_comm, axiom,
    ~bv(A, N) | ~bv(B, N) | bvand(A, B) = bvand(B, A)).
cnf(bvand_assoc, axiom,
    ~bv(A, N) | ~bv(B, N) | ~bv(C, N) |
    bvand(bvand(A, B), C) = bvand(A, bvand(B, C))).
cnf(bvand_idem, axiom, ~bv(A, N) | bvand(A, A) = A).
cnf(bvor_comm, axiom,
    ~bv(A, N) | ~bv(B, N) | bvor(A, B) = bvor(B, A)).
cnf(bvor_assoc, axiom,
    ~bv(A, N) | ~bv(B, N) | ~bv(C, N) |
    bvor(bvor(A, B), C) = bvor(A, bvor(B, C))).
cnf(bvor_idem, axiom, ~bv(A, N) | bvor(A, A) = A).
cnf(bvxor_comm, axiom,
    ~bv(A, N) | ~bv(B, N) | bvxor(A, B) = bvxor(B, A)).
cnf(bvxor_assoc, axiom,
    ~bv(A, N) | ~bv(B, N) | ~bv(C, N) |
    bvxor(bvxor(A, B), C) = bvxor(A, bvxor(B, C))).
cnf(bvxor_zero, axiom, ~bv(A, N) | bvxor(A, bvzero(N)) = A).
cnf(bvxor_self, axiom, ~bv(A, N) | bvxor(A, A) = bvzero(N)).
cnf(bvnot_involutive, axiom, ~bv(A, N) | bvnot(bvnot(A)) = A).
cnf(bvand_zero, axiom, ~bv(A, N) | bvand(A, bvzero(N)) = bvzero(N)).
cnf(bvor_zero, axiom, ~bv(A, N) | bvor(A, bvzero(N)) = A).
cnf(bvand_absorb, axiom,
    ~bv(A, N) | ~bv(B, N) | bvand(A, bvor(A, B)) = A).
cnf(bvor_absorb, axiom,
    ~bv(A, N) | ~bv(B, N) | bvor(A, bvand(A, B)) = A).
cnf(bvand_distrib_bvor, axiom,
    ~bv(A, N) | ~bv(B, N) | ~bv(C, N) |
    bvand(A, bvor(B, C)) = bvor(bvand(A, B), bvand(A, C))).
cnf(bvor_distrib_bvand, axiom,
    ~bv(A, N) | ~bv(B, N) | ~bv(C, N) |
    bvor(A, bvand(B, C)) = bvand(bvor(A, B), bvor(A, C))).

% Modular addition identities.
cnf(bvadd_comm, axiom,
    ~bv(A, N) | ~bv(B, N) | bvadd(A, B) = bvadd(B, A)).
cnf(bvadd_assoc, axiom,
    ~bv(A, N) | ~bv(B, N) | ~bv(C, N) |
    bvadd(bvadd(A, B), C) = bvadd(A, bvadd(B, C))).
cnf(bvadd_zero, axiom, ~bv(A, N) | bvadd(A, bvzero(N)) = A).
cnf(bvadd_inverse, axiom, ~bv(A, N) | bvadd(A, bvneg(A)) = bvzero(N)).

% Concatenation.
cnf(concat_assoc, axiom, concat(concat(A, B), C) = concat(A, concat(B, C))).
cnf(concat_size, axiom, ~bv(A, N) | ~bv(B, M) | bv(concat(A, B), add(N, M))).

% Bitwise operations are pointwise over concatenation. Addition is
% deliberately absent: a carry from the low half can change the high half.
cnf(concat_bvnot, axiom,
    ~bv(A, N) | ~bv(B, M) |
    bvnot(concat(A, B)) = concat(bvnot(A), bvnot(B))).
cnf(concat_bvand, axiom,
    ~bv(A, N) | ~bv(B, M) | ~bv(C, N) | ~bv(D, M) |
    bvand(concat(A, B), concat(C, D)) = concat(bvand(A, C), bvand(B, D))).
cnf(concat_bvor, axiom,
    ~bv(A, N) | ~bv(B, M) | ~bv(C, N) | ~bv(D, M) |
    bvor(concat(A, B), concat(C, D)) = concat(bvor(A, C), bvor(B, D))).
cnf(concat_bvxor, axiom,
    ~bv(A, N) | ~bv(B, M) | ~bv(C, N) | ~bv(D, M) |
    bvxor(concat(A, B), concat(C, D)) = concat(bvxor(A, C), bvxor(B, D))).

% SMT-LIB extract is inclusive: extract(A, Hi, Lo) has width Hi - Lo + 1.
% The two ordering premises enforce 0 <= Lo <= Hi < M without a separate
% non-strict ordering predicate.
cnf(extract_size, axiom,
    ~bv(A, M) | ~lt(Hi, M) | ~lt(Lo, add(Hi, one)) |
    bv(extract(A, Hi, Lo), add(sub(Hi, Lo), one))).
cnf(extract_whole, axiom,
    ~bv(A, N) | ~lt(zero, N) |
    extract(A, sub(N, one), zero) = A).
