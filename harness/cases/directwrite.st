(* Directly represented variables written in a body (Ed.3 6.5.5,
   deprecated: W0002): assigned, a block's output and ENO taken into
   one, one given to a FUNCTION's and a block's in-out. %IX1.0 TRUE,
   %IW2 16#1234, %IX0.2 TRUE from tick 0. Worked by hand:

   tick 0: %QX1.0 := %IX1.0, 1 (Q1 01); %QW2 := %IW2 (Q2 34 12); the
   TON's Q into %QX0.3, not yet; fo's ENO into %QX0.0, TRUE (Q0 01);
   setb sets bit 0 of %MW4 through its in-out (M4 01); the block sets
   bit 8 of %MW6 (M7 01); w counts, 1 (Q4 01). The program io, its
   in-out connected to %MW8 in the CONFIGURATION, shifts a 1 into it:
   1 (M8 01).
   tick 1: w 2, %MW8 3. tick 2: w 3, %MW8 7, and the TON is done after
   20 ms: %QX0.3 (Q0 09).
   reference-skip: the reference compiler's C writes no direct address
   in a body. *)
FUNCTION setb : WORD
  VAR_IN_OUT x : WORD; END_VAR
  x := x OR 16#0001;
  setb := x;
END_FUNCTION
FUNCTION fo : INT
  VAR_INPUT x : INT; END_VAR
  fo := x;
END_FUNCTION
FUNCTION_BLOCK blk
  VAR_IN_OUT x : WORD; END_VAR
  x := x OR 16#0100;
END_FUNCTION_BLOCK
PROGRAM main
  VAR t : TON; w : USINT; k : INT; b : blk; r : WORD; END_VAR
  %QX1.0 := %IX1.0;
  %QW2 := %IW2;
  t(IN := %IX0.2, PT := T#20ms, Q => %QX0.3);
  k := fo(EN := %IX0.2, x := 1, ENO => %QX0.0);
  r := setb(%MW4);
  b(x := %MW6);
  w := w + 1;
  %QB4 := USINT_TO_BYTE(w);
END_PROGRAM
PROGRAM io
  VAR_IN_OUT n : WORD; END_VAR
  n := ROL(n, 1) OR 16#0001;
END_PROGRAM
CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
    PROGRAM p2 WITH t : io(n := %MW8);
  END_RESOURCE
END_CONFIGURATION
