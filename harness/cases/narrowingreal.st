(* implicit-narrowing: an integer where a real is wanted and Figure 11
   has no implicit conversion, and an LREAL where a REAL is, converted
   as DINT_TO_REAL and LREAL_TO_REAL would, W0010 at each - as TcOpen's
   `_price := Price` (an LREAL property into a REAL), its
   TcoBytesToReal's LREAL into the REAL result, and a DINT given to
   TcoRealToBytes's `inValue : REAL`. Worked by hand:
   r := d, DINT 16777217 into a REAL: the nearest REAL, 16777216.0,
   16#4B800000 (%QD0).
   r := l, LREAL 1.5: 1.5, 16#3FC00000 (%QD4).
   twice(n), the DINT 3 to a REAL input: 6.0, 16#40C00000 (%QD8).
   quarter(), an LREAL 2.25 into the REAL result: 16#40100000 (%QD12).
   reference-skip: implicit narrowing is CODESYS's, not IEC 61131-3. *)
FUNCTION twice : REAL
  VAR_INPUT x : REAL; END_VAR
  twice := x * 2.0;
END_FUNCTION

FUNCTION quarter : REAL
  VAR l : LREAL := 2.25; END_VAR
  quarter := l;
END_FUNCTION

PROGRAM main
  VAR
    d : DINT := 16777217; l : LREAL := 1.5; n : DINT := 3; r : REAL;
  END_VAR
  VAR
    q0 AT %QD0 : REAL;
    q1 AT %QD4 : REAL;
    q2 AT %QD8 : REAL;
    q3 AT %QD12 : REAL;
  END_VAR
  r := d;
  q0 := r;
  r := l;
  q1 := r;
  q2 := twice(n);
  q3 := quarter();
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
