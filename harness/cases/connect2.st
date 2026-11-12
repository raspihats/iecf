(* A program's connections (Ed.3 6.8.2) of strings, subranges, fields,
   and in-outs of every kind: a structure, an array, a STRING, an
   instance, variables at addresses, a subrange of other bounds.
   Worked by hand, tick 0 then tick 1:
   s : STRING[3] := 'abcdef' is cut: LEN 3 (%QB0); flagged
   STRING_LENGTH, the first flag, at the CONFIGURATION's line of it.
   a : SUBR (0..5) := 9 is clamped: 5 (%QB1). f := gp.x, a field: 7,
   then 8 (%QB2). o => gsub, a SUBR, given 9: 5, read through gsub the
   next tick: 0 then 5 (%QB3). os => gs2, a STRING[2], given 'abcd':
   'ab', LEN 0 then 2 (%QB4).
   In-outs, the globals themselves: ip.x + 1, gp.x 8 then 9 (%QB5);
   ia[2] + 10, ga[2] 12 then 22 (%QB6); is + 'z', gstr 'az' then 'azz'
   (%QB7); ic(), gcnt.n 1 then 2 (%QB8).
   At addresses, copied in before the run and back after it: il, gl at
   %MW0, 4 + 1 = 5 then 6 (%QB9, and %MW0); ila[1], gla at %MW2, 10 + 2
   = 12 then 14 (%QB10, and %MW2).
   isub, an INT, given gsub2, a SUBR of 1: 1 + 9 = 10 in the program
   (%QB11), stored back clamped to 5; the next tick 5 + 9 = 14.
   reference-skip: the reference compiler crashes on program
   connections (connections). *)
TYPE
  SUBR : INT (0..5);
  PT : STRUCT x : INT; END_STRUCT;
END_TYPE
FUNCTION_BLOCK Cnt
  VAR_OUTPUT n : INT; END_VAR
  n := n + 1;
END_FUNCTION_BLOCK
PROGRAM main
  VAR_INPUT s : STRING[3]; a : SUBR; f : INT; END_VAR
  VAR_OUTPUT o : INT; os : STRING; END_VAR
  VAR_IN_OUT
    ip : PT;
    ia : ARRAY [1..3] OF INT;
    is : STRING;
    ic : Cnt;
    il : INT;
    ila : ARRAY [1..2] OF INT;
    isub : INT;
  END_VAR
  VAR_EXTERNAL gsub : SUBR; gs2 : STRING[2]; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
    q6 AT %QB6 : SINT;
    q7 AT %QB7 : SINT;
    q8 AT %QB8 : SINT;
    q9 AT %QB9 : SINT;
    q10 AT %QB10 : SINT;
    q11 AT %QB11 : SINT;
  END_VAR
  q0 := INT_TO_SINT(LEN(s));
  q1 := INT_TO_SINT(a);
  q2 := INT_TO_SINT(f);
  q3 := INT_TO_SINT(gsub);
  q4 := INT_TO_SINT(LEN(gs2));
  ip.x := ip.x + 1;
  q5 := INT_TO_SINT(ip.x);
  ia[2] := ia[2] + 10;
  q6 := INT_TO_SINT(ia[2]);
  is := CONCAT(is, 'z');
  q7 := INT_TO_SINT(LEN(is));
  ic();
  q8 := INT_TO_SINT(ic.n);
  il := il + 1;
  q9 := INT_TO_SINT(il);
  ila[1] := ila[1] + 2;
  q10 := INT_TO_SINT(ila[1]);
  isub := isub + 9;
  q11 := INT_TO_SINT(isub);
  o := 9;
  os := 'abcd';
END_PROGRAM

CONFIGURATION c
  VAR_GLOBAL
    gp : PT := (x := 7);
    ga : ARRAY [1..3] OF INT := [1, 2, 3];
    gstr : STRING := 'a';
    gcnt : Cnt;
    gsub : SUBR;
    gs2 : STRING[2];
    gsub2 : SUBR := 1;
  END_VAR
  VAR_GLOBAL
    gl AT %MW0 : INT := 4;
    gla AT %MW2 : ARRAY [1..2] OF INT := [10, 20];
  END_VAR
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main(
      s := 'abcdef',
      a := 9,
      f := gp.x,
      o => gsub,
      os => gs2,
      ip := gp,
      ia := ga,
      is := gstr,
      ic := gcnt,
      il := gl,
      ila := gla,
      isub := gsub2);
  END_RESOURCE
END_CONFIGURATION
