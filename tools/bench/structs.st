(* iecf's benchmark (docs/testing/performance.md): an array of 200 nested structures filled, copied through a FUNCTION's VAR_IN_OUT and reversed.
   Standard IEC 61131-3, for both compilers: tools/bench/run.sh. *)
TYPE
  Point : STRUCT x : DINT; y : DINT; END_STRUCT;
  Seg : STRUCT a : Point; b : Point; len2 : DINT; tag : INT; END_STRUCT;
END_TYPE
FUNCTION Measure : DINT
  VAR_IN_OUT s : Seg; END_VAR
  VAR dx : DINT; dy : DINT; END_VAR
  dx := s.b.x - s.a.x;
  dy := s.b.y - s.a.y;
  s.len2 := dx * dx + dy * dy;
  Measure := s.len2;
END_FUNCTION
PROGRAM main
  VAR segs : ARRAY[1..200] OF Seg; tmp : Seg; i : INT; rep : INT; acc : DINT; seed : DINT := 1; d : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; o2 AT %QD4 : DINT; END_VAR
  FOR rep := 1 TO 20 DO
  FOR i := 1 TO 200 DO
    seed := (seed * 1103 + 12345) MOD 65536;
    segs[i].a.x := seed MOD 1000;
    segs[i].a.y := seed / 1000;
    segs[i].b := segs[i].a;
    segs[i].b.x := segs[i].b.x + INT_TO_DINT(i);
    segs[i].tag := i;
    tmp := segs[i];
    d := Measure(s := tmp);
    segs[i] := tmp;
  END_FOR;
  FOR i := 1 TO 100 DO
    tmp := segs[i];
    segs[i] := segs[201 - i];
    segs[201 - i] := tmp;
  END_FOR;
  END_FOR;
  acc := 0;
  FOR i := 1 TO 200 DO
    acc := (acc + segs[i].len2 * INT_TO_DINT(segs[i].tag)) MOD 1000003;
  END_FOR;
  o1 := acc;
  o2 := segs[1].a.x + segs[200].b.y;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
