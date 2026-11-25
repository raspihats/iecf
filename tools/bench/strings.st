(* iecf's benchmark (docs/testing/performance.md): CONCAT, LEFT, RIGHT, LEN, FIND, INT_TO_STRING and STRING_TO_INT, 1000 times a cycle.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
PROGRAM main
  VAR i : INT; s : STRING; t : STRING := 'pump-station-'; n : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 1000 DO
    s := CONCAT(LEFT(t, 5 + i MOD 8), INT_TO_STRING(i));
    n := (n + LEN(s) + FIND(s, '9') + INT_TO_DINT(STRING_TO_INT(RIGHT(s, 1)))) MOD 1000003;
    IF s = 'pump-sta123' THEN n := n + 1; END_IF;
  END_FOR;
  o1 := n;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
