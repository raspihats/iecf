(* An array or a structure at an address, or reached through a
   reference, given to an in-out, to an ARRAY[*] input, output or
   in-out, of a FUNCTION and of a FUNCTION_BLOCK. No C object of its
   type is there (the image is read byte by byte), so the call is given
   a copy, stored back as it returns (docs/dependencies.md).
   SUM adds an ARRAY[*] input's first and last element. INC adds 1 to
   the first element of its ARRAY[*], to f[3] and to p.y; the block BMP
   does the same. SET writes 5 to the last element of its ARRAY[*]
   output, the others their initial 0.
   m at %MW0, 3 INTs; lp at %MW6, a PT; g at %MW10, 2 by 3 INTs. rm
   refers to m, r to x. Worked by hand, k %IB0 1, 2, 3:
   m[1] := k; m[3] := 10. SUM(m) k + 10: 11, 12, 13 (%QB0).
   g[2, 1] := 3; g[2, 3] := 4: SUM(g[2]), a row at an address, 7 (%QB1).
   SUM(rm^), through a reference, k + 10 (%QB2).
   INC(m, x, lp): m[1] k + 1; x[3] + 1; lp.y + 1.
   INC(g[j], rm^, ps[2]), j 1: g[1, 1] 1; m[3] 11; ps[2].y + 1.
   INC(r^, m, lp): x[1] + 1; m[3] 12; lp.y + 1.
   SET(a => g[1]): g[1] is [0, 0, 5].
   b(a := m, f := x, p := lp): m[1] k + 2; x[3] + 1; lp.y + 1.
   b(a := g[j], f := rm^, p := ps[j]): g[1, 1] 1; m[3] 13; ps[1].y + 1.
   So x[1] 1, 2, 3 (%QB3); x[3] 2, 4, 6 (%QB4); ps[2].y 1, 2, 3 (%QB5);
   ps[1].y 1, 2, 3 (%QB6); and in %M: m [k + 2, 0, 13], lp.y 3, 6, 9,
   g [[1, 0, 5], [3, 0, 4]].
   reference-skip: ARRAY[*] is Ed.3's; the reference compiler (Ed.2)
   has none. *)
TYPE PT : STRUCT x, y : INT; END_STRUCT; END_TYPE
FUNCTION SUM : INT
  VAR_INPUT a : ARRAY [*] OF INT; END_VAR
  SUM := a[LOWER_BOUND(a, 1)] + a[UPPER_BOUND(a, 1)];
END_FUNCTION
FUNCTION INC
  VAR_IN_OUT a : ARRAY [*] OF INT; f : ARRAY [1..3] OF INT; p : PT; END_VAR
  a[LOWER_BOUND(a, 1)] := a[LOWER_BOUND(a, 1)] + 1;
  f[3] := f[3] + 1;
  p.y := p.y + 1;
END_FUNCTION
FUNCTION SET
  VAR_OUTPUT a : ARRAY [*] OF INT; END_VAR
  a[UPPER_BOUND(a, 1)] := 5;
END_FUNCTION
FUNCTION_BLOCK BMP
  VAR_IN_OUT a : ARRAY [*] OF INT; f : ARRAY [1..3] OF INT; p : PT; END_VAR
  a[LOWER_BOUND(a, 1)] := a[LOWER_BOUND(a, 1)] + 1;
  f[3] := f[3] + 1;
  p.y := p.y + 1;
END_FUNCTION_BLOCK
CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR
    m AT %MW0 : ARRAY [1..3] OF INT;
    lp AT %MW6 : PT;
    g AT %MW10 : ARRAY [1..2, 1..3] OF INT;
  END_VAR
  VAR
    x : ARRAY [1..3] OF INT;
    r, rm : REF_TO ARRAY [1..3] OF INT;
    ps : ARRAY [1..2] OF PT;
    b : BMP;
    j : INT := 1;
  END_VAR
  VAR k AT %IB0 : SINT; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
    q6 AT %QB6 : SINT;
  END_VAR
  m[1] := k;
  m[3] := 10;
  q0 := INT_TO_SINT(SUM(m));
  g[2, 1] := 3;
  g[2, 3] := 4;
  q1 := INT_TO_SINT(SUM(g[2]));
  rm := REF(m);
  q2 := INT_TO_SINT(SUM(rm^));
  INC(m, x, lp);
  INC(g[j], rm^, ps[2]);
  r := REF(x);
  INC(r^, m, lp);
  SET(a => g[1]);
  b(a := m, f := x, p := lp);
  b(a := g[j], f := rm^, p := ps[j]);
  q3 := INT_TO_SINT(x[1]);
  q4 := INT_TO_SINT(x[3]);
  q5 := INT_TO_SINT(ps[2].y);
  q6 := INT_TO_SINT(ps[1].y);
END_PROGRAM
