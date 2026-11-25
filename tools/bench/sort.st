(* iecf's benchmark (docs/testing/performance.md): an insertion sort of 300 DINTs, filled by a linear congruential generator.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
PROGRAM main
  VAR a : ARRAY[1..300] OF DINT; i : INT; j : INT; k : DINT; seed : DINT := 7; sum : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; o2 AT %QD4 : DINT; END_VAR
  FOR i := 1 TO 300 DO
    seed := (seed * 1103 + 12345) MOD 65536;
    a[i] := seed;
  END_FOR;
  FOR i := 2 TO 300 DO
    k := a[i];
    j := i - 1;
    WHILE j >= 1 DO
      IF a[j] <= k THEN EXIT; END_IF;
      a[j + 1] := a[j];
      j := j - 1;
    END_WHILE;
    a[j + 1] := k;
  END_FOR;
  sum := 0;
  FOR i := 1 TO 300 DO sum := (sum + a[i] * INT_TO_DINT(i)) MOD 1000003; END_FOR;
  o1 := sum;
  o2 := a[150];
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
