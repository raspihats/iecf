(* A pointer indexed (the pointers extension): p[i] is (p + i *
   SIZEOF(T))^, as CODESYS and TwinCAT index a pointer, checked as p^
   is. Worked by hand: a is 10 20 30 40 50, so the loop adds 150 and
   %QB0 is 15; p[k] with k 2 writes a[2] := 7; p moved to ADR(p[3]),
   a[3], reads p[-1] + p[0], 7 + 40, so %QB1 is 47; q[1].y := 9 and
   s[2].x := 4 read back through q and s, 9 + 4, %QB2 13; p[1] is
   a[4], 50, %QB3 5; %QB4 set to 1, then p[2] is past a's end: a
   fault, IECF_FAULT_INDEX, on line 36, %QB4 left at 1.
   reference-skip: pointers are CODESYS's, not IEC 61131-3. *)
TYPE pair : STRUCT x : INT; y : INT; END_STRUCT; END_TYPE
PROGRAM main
  VAR
    a : ARRAY[0..4] OF INT := [10, 20, 30, 40, 50];
    s : ARRAY[0..2] OF pair;
    p : POINTER TO INT;
    q : POINTER TO pair;
    i, n : INT;
    k : UINT := 2;
  END_VAR
  VAR q0 AT %QB0 : SINT; q1 AT %QB1 : SINT; q2 AT %QB2 : SINT; q3 AT %QB3 : SINT; q4 AT %QB4 : SINT; END_VAR
  p := ADR(a);
  n := 0;
  FOR i := 0 TO 4 DO
    n := n + p[i];
  END_FOR;
  q0 := INT_TO_SINT(n / 10);
  p[k] := 7;
  p := ADR(p[3]);
  q1 := INT_TO_SINT(p[-1] + p[0]);
  q := ADR(s);
  q[1].y := 9;
  s[2].x := 4;
  q2 := INT_TO_SINT(s[1].y + q[2].x);
  q3 := INT_TO_SINT(p[1] / 10);
  q4 := 1;
  q4 := INT_TO_SINT(p[2]);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
