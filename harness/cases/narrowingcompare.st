(* implicit-narrowing: a ULINT and a signed integer, which no type holds
   both of, compared by their values, W0010 at each - as TcOpen's tests'
   `_plcCycleCounter >= _reqStep + 7`, a ULINT and an INT. Worked by
   hand, u 5, m -1, big 2^64 - 1, l -1, seven 7, four 4:
   u > m TRUE (%QX0.0); m < u TRUE (%QX0.1); u = m FALSE (%QX0.2);
   big > l TRUE, where the bits alike would say equal (%QX0.3);
   u + 2 = seven, 7 = 7, TRUE (%QX0.4); u <= four FALSE (%QX0.5).
   reference-skip: implicit narrowing is CODESYS's, not IEC 61131-3. *)
PROGRAM main
  VAR
    u : ULINT := 5; big : ULINT := 18446744073709551615;
    m : INT := -1; l : LINT := -1; seven : DINT := 7; four : SINT := 4;
  END_VAR
  VAR
    q0 AT %QX0.0 : BOOL; q1 AT %QX0.1 : BOOL; q2 AT %QX0.2 : BOOL;
    q3 AT %QX0.3 : BOOL; q4 AT %QX0.4 : BOOL; q5 AT %QX0.5 : BOOL;
  END_VAR
  q0 := u > m;
  q1 := m < u;
  q2 := u = m;
  q3 := big > l;
  q4 := u + 2 = seven;
  q5 := u <= four;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
