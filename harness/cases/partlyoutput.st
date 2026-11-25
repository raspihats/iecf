(* A function block's VAR_OUTPUT of a structure with `*` components
   (Ed.3 6.4.4.8 excludes only VAR_INPUT, VAR_IN_OUT and VAR_TEMP),
   completed per instance by VAR_CONFIG (6.5.5.4): d.o.q at %QW0, d.o.m
   at %MW0; the program's own s of the same type at %QW2 and %MW2.
   Worked by hand, k counting 1, 2: d sets o.q := n * 2, o.m + 1; the
   program reads d.o.m into %QW4 and copies d.o into s.
   Tick 0: %QW0 2, %QW2 2, %QW4 1; %MW0 1, %MW2 1.
   Tick 1: %QW0 4, %QW2 4, %QW4 2; %MW0 2, %MW2 2.
   reference-skip: the reference compiler has no `*` structure components. *)
TYPE Out : STRUCT
  q AT %Q* : INT;
  m AT %M* : INT;
END_STRUCT;
END_TYPE

FUNCTION_BLOCK Drive
  VAR_INPUT n : INT; END_VAR
  VAR_OUTPUT o : Out; END_VAR
  o.q := n * 2;
  o.m := o.m + 1;
END_FUNCTION_BLOCK

PROGRAM main
  VAR d : Drive; k : INT; s : Out; END_VAR
  VAR seen AT %QW4 : INT; END_VAR
  k := k + 1;
  d(n := k);
  seen := d.o.m;
  s := d.o;
END_PROGRAM

CONFIGURATION cf
  RESOURCE rs ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
  VAR_CONFIG
    rs.p.d.o.q AT %QW0 : INT;
    rs.p.d.o.m AT %MW0 : INT;
    rs.p.s.q AT %QW2 : INT;
    rs.p.s.m AT %MW2 : INT;
  END_VAR
END_CONFIGURATION
