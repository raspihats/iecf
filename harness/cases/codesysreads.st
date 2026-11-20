(* A function block's internal VAR read from outside, never written
   (internal-reads), as CODESYS and TwinCAT allow, each read warned
   (W0012). Worked by hand, after a(x := 5) and a(x := 6):
   %QW0: a.count, 2.  %QD2: a.total, 5 + 6 = 11.
   %QW6: a.arr[2], 3.  %QW8: a.inn.q, Inner's output, 7.
   %QW10: a.inn.k, Inner's own VAR, 7.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Inner
  VAR_OUTPUT q : INT; END_VAR
  VAR k : INT := 7; END_VAR
  q := k;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Acc
  VAR_INPUT x : INT; END_VAR
  VAR count : INT; total : DINT; arr : ARRAY[0..2] OF INT := [1, 2, 3]; inn : Inner; END_VAR
  count := count + 1;
  total := total + x;
  inn();
END_FUNCTION_BLOCK

PROGRAM main
  VAR a : Acc; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QD2 : DINT; q2 AT %QW6 : INT; q3 AT %QW8 : INT; q4 AT %QW10 : INT; END_VAR
  a(x := 5);
  a(x := 6);
  q0 := a.count;
  q1 := a.total;
  q2 := a.arr[2];
  q3 := a.inn.q;
  q4 := a.inn.k;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
