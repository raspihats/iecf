(* CASE subrange labels with a bound at the selector type's own limit:
   `0..5` on a USINT, `-128..-1` and `1..127` on a SINT, `0..255` on a
   USINT, all of it. Such a bound tests nothing, and its comparison is
   one C calls always true (-Wtype-limits): the generated C must still
   be strict-clean. Worked by hand, n and b counting 0, 1, 2 from tick 0,
   s -128, then -1, then 126:

   tick 0: n 0 in 0..5: Q0 01; s -128: Q1 01; b 0: Q2 01.
   tick 1: n 1: 01; s -1: 01; b 1: 01.
   tick 2: n 2: 01; s 126: Q1 03; 01. *)
PROGRAM main
  VAR n : USINT; s : SINT := -128; b : USINT; k : INT; END_VAR
  VAR q0 AT %QB0 : USINT; q1 AT %QB1 : USINT; q2 AT %QB2 : USINT; END_VAR
  CASE n OF 0..5: q0 := 1; ELSE q0 := 2; END_CASE;
  CASE s OF -128..-1: q1 := 1; 0: q1 := 2; 1..127: q1 := 3; END_CASE;
  CASE b OF 0..255: q2 := 1; END_CASE;
  n := n + 1;
  b := b + 1;
  k := k + 1;
  IF k = 1 THEN s := -1; ELSE s := 126; END_IF;
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION
