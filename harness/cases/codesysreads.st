(* A function block's internal VAR read from outside, never written
   (internal-reads), as CODESYS and TwinCAT allow, each read warned
   (W0012). Worked by hand, after a(x := 5) and a(x := 6):
   %QW0: a.count, 2.  %QD2: a.total, 5 + 6 = 11.
   %QW6: a.arr[2], 3.  %QW8: a.inn.q, Inner's output, 7.
   %QW10: a.inn.k, Inner's own VAR, 7.
   %QW12: dr.st.Restore() twice, a method of Drive's internal instance
   st, as TcOpen calls one: 10 + 10 = 20.
   %QW14: dr.cnt(inc := 3, total => q6) twice, Drive's internal
   instance cnt called from outside, as TcOpen's `_failTimer(...)`
   through its task: 3 + 3 = 6. %QW16: w.dr.cnt(inc := 100) once,
   two levels in, then w.dr.cnt.total read: 100.
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

FUNCTION_BLOCK State
  VAR_OUTPUT n : INT; END_VAR
  METHOD PUBLIC Restore
    n := n + 10;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Counter
  VAR_INPUT inc : INT; END_VAR
  VAR_OUTPUT total : INT; END_VAR
  total := total + inc;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Drive
  VAR st : State; cnt : Counter; END_VAR
END_FUNCTION_BLOCK

FUNCTION_BLOCK Wrap
  VAR dr : Drive; END_VAR
END_FUNCTION_BLOCK

PROGRAM main
  VAR a : Acc; dr : Drive; w : Wrap; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QD2 : DINT; q2 AT %QW6 : INT; q3 AT %QW8 : INT; q4 AT %QW10 : INT; q5 AT %QW12 : INT;
      q6 AT %QW14 : INT; q7 AT %QW16 : INT; END_VAR
  a(x := 5);
  a(x := 6);
  q0 := a.count;
  q1 := a.total;
  q2 := a.arr[2];
  q3 := a.inn.q;
  q4 := a.inn.k;
  dr.st.Restore();
  dr.st.Restore();
  q5 := dr.st.n;
  dr.cnt(inc := 3, total => q6);
  dr.cnt(inc := 3, total => q6);
  w.dr.cnt(inc := 100);
  q7 := w.dr.cnt.total;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
