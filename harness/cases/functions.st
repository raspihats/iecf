(* The operators as functions (Ed.3 Tables 29-31, 33): ADD, MUL, AND,
   OR, XOR with more than two inputs; SUB, DIV, MOD; NOT; the
   comparisons GT, GE, EQ, LE, LT, NE; MOVE; SEL; EXPT and **. These are
   what Prokop's ladder and function block diagrams are written in.
   EXPT's results here are exact in binary floating point. *)
PROGRAM main
VAR
  a AT %IW0 : INT;
  b AT %IW2 : INT;
  m AT %IB4 : BYTE;
  s AT %IX5.0 : BOOL;
  r_add AT %MW0 : INT;
  r_mul AT %MW2 : INT;
  r_sub AT %MW4 : INT;
  r_div AT %MW6 : INT;
  r_mod AT %MW8 : INT;
  r_and AT %MB10 : BYTE;
  r_or AT %MB11 : BYTE;
  r_xor AT %MB12 : BYTE;
  r_not AT %MB13 : BYTE;
  r_cmp AT %MB14 : BYTE;
  r_sel AT %MW16 : INT;
  r_move AT %MW18 : INT;
  r_expt AT %MD20 : REAL;
  r_pow AT %ML24 : LREAL;
  r_bool AT %QX0.0 : BOOL;
END_VAR
r_add := ADD(a, b, 3);
r_mul := MUL(a, 2, -1);
r_sub := SUB(a, b);
r_div := DIV(a, b);
r_mod := MOD(a, b);
r_and := AND(m, BYTE#16#F0, BYTE#16#3C);
r_or := OR(m, BYTE#16#01);
r_xor := XOR(m, BYTE#16#FF, BYTE#16#0F);
r_not := NOT(m);
r_cmp := 0;
IF GT(a, b) THEN r_cmp := r_cmp OR 1; END_IF;
IF GE(a, b) THEN r_cmp := r_cmp OR 2; END_IF;
IF EQ(a, b) THEN r_cmp := r_cmp OR 4; END_IF;
IF LE(a, b) THEN r_cmp := r_cmp OR 8; END_IF;
IF LT(a, b) THEN r_cmp := r_cmp OR 16; END_IF;
IF NE(a, b) THEN r_cmp := r_cmp OR 32; END_IF;
r_sel := SEL(s, a, b);
r_move := MOVE(b);
r_expt := EXPT(INT_TO_REAL(b), 2);
r_pow := INT_TO_LREAL(a) ** 2.0 + 2.0 ** b;
r_bool := AND(OR(s, r_bool), NOT(EQ(a, 0)));
END_PROGRAM

CONFIGURATION config
  RESOURCE res ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
