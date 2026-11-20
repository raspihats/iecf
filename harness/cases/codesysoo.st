(* CODESYS's and TwinCAT's object orientation (codesys-oo), as TcOpen
   writes it: THIS^ and SUPER^, overriding without OVERRIDE, no access
   specifier meaning PUBLIC. Worked by hand:
   %QD0: sq.Area(): side * side + SUPER^.Area(), 3.0 * 3.0 + 0.0 = 9.0,
   16#41100000.
   %QW4: sq.Describe(), Shape's, calls THIS^.Kind(): Square's override,
   2.
   %QW6: sq's body calls SUPER^(), Shape's body, once: calls = 1.
   %QW8: sq.Twice(), Shape's, calls Kind() bare, as THIS.Kind():
   Square's override, 2 * 2 = 4.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Shape
  VAR_OUTPUT calls : INT; END_VAR
  METHOD Area : REAL
    Area := 0.0;
  END_METHOD
  METHOD Describe : INT
    Describe := THIS^.Kind();
  END_METHOD
  METHOD Kind : INT
    Kind := 1;
  END_METHOD
  METHOD Twice : INT
    Twice := Kind() * 2;
  END_METHOD
  calls := calls + 1;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Square EXTENDS Shape
  VAR_INPUT side : REAL; END_VAR
  METHOD Area : REAL
    Area := side * side + SUPER^.Area();
  END_METHOD
  METHOD Kind : INT
    Kind := 2;
  END_METHOD
  SUPER^();
END_FUNCTION_BLOCK

PROGRAM main
  VAR sq : Square; END_VAR
  VAR a AT %QD0 : REAL; k AT %QW4 : INT; c AT %QW6 : INT; t AT %QW8 : INT; END_VAR
  sq(side := 3.0);
  a := sq.Area();
  k := sq.Describe();
  c := sq.calls;
  t := sq.Twice();
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
