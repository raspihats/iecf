(* iecf's benchmark (docs/testing/performance.md): a structure assigned field by field and copied.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
TYPE Pt : STRUCT x : DINT; y : DINT; END_STRUCT; END_TYPE
PROGRAM main
  VAR i : DINT; p : Pt; q : Pt; n : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  FOR i := 1 TO 20000 DO
    p.x := i; p.y := i MOD 9; q := p; n := (n + q.x + q.y) MOD 100000;
  END_FOR;
  o1 := n;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
