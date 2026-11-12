(* RETAIN at an address (Ed.3 Loc_Var_Decls: VAR RETAIN ... AT): its
   initial value is written at a cold start only; at a warm start the
   platform keeps the image bytes, and iecf leaves them (decided
   2026-11-12, florin). Each adds 1 at every tick; a warm restart before
   tick 2. Worked by hand, the M words after set-up, then each tick:
   x RETAIN AT %MW0 := 7:  7, 8, 9 | warm | 10, 11
   y AT %MW2 := 7 (plain): 7, 8, 9 | warm, set again: 7 | 8, 9
   g, a VAR_GLOBAL RETAIN AT %MW4 := 3: 3, 4, 5 | 6, 7
   the block B's RETAIN z AT %MW6 := 10: 10, 11, 12 | 13, 14
   reference-skip: the reference compiler's host makes a warm restart a
   cold one (retain). *)
FUNCTION_BLOCK B
  VAR RETAIN z AT %MW6 : INT := 10; END_VAR
  z := z + 1;
END_FUNCTION_BLOCK
PROGRAM main
  VAR RETAIN x AT %MW0 : INT := 7; END_VAR
  VAR y AT %MW2 : INT := 7; END_VAR
  VAR_EXTERNAL g : INT; END_VAR
  VAR b : B; END_VAR
  x := x + 1;
  y := y + 1;
  g := g + 1;
  b();
END_PROGRAM

CONFIGURATION c
  VAR_GLOBAL RETAIN g AT %MW4 : INT := 3; END_VAR
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
