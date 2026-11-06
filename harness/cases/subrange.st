(* A value stored into a subrange outside its bounds is clamped to the
   bound it passed and flagged (docs/dependencies.md, decided
   2026-11-06): by an assignment, into an array's element and a
   structure's field, into a function block's input; a FOR over a
   subrange stops at the bound it would pass. At tick 1, 150 leaves
   percent: p is 100, and the FOR to 150 stops at 3; at tick 2, -5: p is
   0, and the FOR to -5 does not run. The first is flagged, code 4 at
   line 41. matiec stores unchecked but at the function block's input,
   and its FOR leaves the subrange; iecf's trace is
   subrange.iecf.trace. *)
TYPE
  percent : INT (0..100) := 50;
  pair : STRUCT
    a : percent;
  END_STRUCT;
END_TYPE

FUNCTION_BLOCK hold
VAR_INPUT v : percent; END_VAR
VAR_OUTPUT o : INT; END_VAR
o := v;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  k AT %IW0 : INT;
  r_p AT %MW0 : INT;
  r_arr AT %MW2 : INT;
  r_pair AT %MW4 : INT;
  r_fb AT %MW6 : INT;
  r_for AT %MW8 : INT;
END_VAR
VAR
  p : percent;
  arr : ARRAY [1..2] OF percent;
  pr : pair;
  h : hold;
  i : INT (1..3);
  n : INT;
END_VAR
p := k;
arr[2] := k * 2;
pr.a := k + 1;
h(v := k - 1);
r_p := p;
r_arr := arr[2];
r_pair := pr.a;
r_fb := h.o;
n := 0;
FOR i := 1 TO k DO
  n := n + 1;
END_FOR;
r_for := n;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
