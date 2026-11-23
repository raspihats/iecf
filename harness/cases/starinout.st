(* A structure with `*` components given to an in-out (Ed.3 6.4.4.8,
   6.5.5.4): the in-out holds no data of its own, it binds the caller's
   variable, whose components VAR_CONFIG places. A FUNCTION_BLOCK and a
   FUNCTION each reach the image through it.
   Worked by hand, each tick: d.a := d.a + 10 (a at %MW0, from 0);
   r(x := d) sets q (%QW0) to a + 1; Twice(x := d) sets o (%QW2) to
   a * 2 and gives it, into %QW4.
   Tick 1: a 10, q 11, o 20, 20. Tick 2: a 20, q 21, o 40, 40.
   reference-skip: VAR_CONFIG, as varconfig. *)
TYPE Io : STRUCT
  a AT %M* : INT;
  q AT %Q* : INT;
  o AT %Q* : INT;
END_STRUCT;
END_TYPE

FUNCTION_BLOCK Robot
  VAR_IN_OUT x : Io; END_VAR
  x.q := x.a + 1;
END_FUNCTION_BLOCK

FUNCTION Twice : INT
  VAR_IN_OUT x : Io; END_VAR
  x.o := x.a * 2;
  Twice := x.o;
END_FUNCTION

PROGRAM main
  VAR d : Io; END_VAR
  VAR r : Robot; END_VAR
  VAR t AT %QW4 : INT; END_VAR
  d.a := d.a + 10;
  r(x := d);
  t := Twice(x := d);
END_PROGRAM

CONFIGURATION cf
  RESOURCE rs ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
  VAR_CONFIG
    rs.p.d.a AT %MW0 : INT;
    rs.p.d.q AT %QW0 : INT;
    rs.p.d.o AT %QW2 : INT;
  END_VAR
END_CONFIGURATION
