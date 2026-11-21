(* CODESYS's and TwinCAT's ANY_TO_<type> (any-to): Ed.3's overloaded
   TO_<type>, as TcOpen's TcoRealToBytes and StringBuilder call it.
   Worked by hand, d + 1 and b + 1 each tick, d from 41, b from 7:
   %QW0 ANY_TO_INT(r * 2.0), r = ANY_TO_REAL(d): tick 0 d 42: 84;
   %QW2 ANY_TO_INT(ANY_TO_STRING(b) + '0' joined): b 7: '70': 70;
   %QW4 ANY_TO_INT(ANY_TO_BOOL(b - 7)): b 7: FALSE: 0.
   tick 1: d 43: 86; b 8: 80; 1.
   reference-skip: not IEC 61131-3. *)
PROGRAM main
  VAR d : DINT := 41; r : REAL; b : BYTE := 7; END_VAR
  VAR q0 AT %QW0 : INT; END_VAR
  VAR q1 AT %QW2 : INT; END_VAR
  VAR q2 AT %QW4 : INT; END_VAR
  d := d + 1;
  r := ANY_TO_REAL(d);
  q0 := ANY_TO_INT(r * 2.0);
  q1 := ANY_TO_INT(CONCAT(ANY_TO_STRING(b), '0'));
  q2 := ANY_TO_INT(ANY_TO_BOOL(b - 7));
  b := b + 1;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
