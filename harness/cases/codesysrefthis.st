(* A method that gives its own instance as a REFERENCE TO, `REF= THIS^`,
   called again on what it gives - TcOpen's StringBuilder,
   `sb.Append('a').Append('b')` (codesys-oo, reference-to).
   twice overrides add: a call through the reference given reaches it.
   Worked by hand, each tick:
   a.add(1).add(2).add(3): a.n + 6.
   t.add(1).add(10): twice's both, + 2 + 20; r REF= t.add(100): + 200,
   r is t; r.add(1000), through r, twice's: + 2000. t.n + 2222.
   %QD0 a.n, %QD4 t.n, %QD8 r.n (t's):
   tick 0: 6, 2222, 2222; tick 1: 12, 4444, 4444.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK acc
  VAR_OUTPUT n : DINT; END_VAR
  METHOD add : REFERENCE TO acc
    VAR_INPUT k : DINT; END_VAR
    n := n + k;
    add REF= THIS^;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK twice EXTENDS acc
  METHOD add : REFERENCE TO acc
    VAR_INPUT k : DINT; END_VAR
    n := n + 2 * k;
    add REF= THIS^;
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR a : acc; t : twice; r : REFERENCE TO acc; END_VAR
  VAR q0 AT %QD0 : DINT; END_VAR
  VAR q1 AT %QD4 : DINT; END_VAR
  VAR q2 AT %QD8 : DINT; END_VAR
  a.add(1).add(2).add(3);
  t.add(1).add(10);
  r REF= t.add(100);
  r.add(1000);
  q0 := a.n;
  q1 := t.n;
  q2 := r.n;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
