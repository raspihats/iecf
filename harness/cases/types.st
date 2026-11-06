(* Derived types: enumerated, subrange, ARRAY of one and two dimensions
   with bounds not at 0, STRUCT, an array of structures, initial values of
   each, and assignment of a whole structure and a whole array. *)
TYPE
  mode : (idle, filling, draining) := idle;
  percent : INT (0..100) := 50;
  point : STRUCT
    x : INT := 1;
    y : INT := -1;
  END_STRUCT;
  row : ARRAY [1..4] OF INT := [10, 20, 30, 40];
END_TYPE

PROGRAM main
VAR
  inp AT %IW0 : INT;
  r_mode AT %MB0 : BYTE;
  r_pct AT %MW2 : INT;
  r_sum AT %MW4 : INT;
  r_grid AT %MW6 : INT;
  r_pt AT %MW8 : INT;
  r_pts AT %MW10 : INT;
  r_copy AT %MW12 : INT;
END_VAR
VAR
  m : mode;
  p : percent;
  r : row;
  r2 : row;
  grid : ARRAY [-1..1, 0..2] OF INT := [1, 2, 3, 4, 5, 6, 7, 8, 9];
  pt : point;
  pt2 : point := (x := 5);
  pts : ARRAY [0..2] OF point;
  i : INT;
END_VAR
CASE m OF
  idle: IF inp > 0 THEN m := filling; END_IF;
  filling: IF inp > 10 THEN m := draining; END_IF;
  draining: IF inp <= 0 THEN m := idle; END_IF;
END_CASE;
IF m = idle THEN r_mode := 0; ELSIF m = filling THEN r_mode := 1; ELSE r_mode := 2; END_IF;
p := LIMIT(0, inp * 5, 100);
r_pct := p;
r[2] := r[2] + inp;
r_sum := 0;
FOR i := 1 TO 4 DO
  r_sum := r_sum + r[i];
END_FOR;
r_grid := grid[-1, 0] * 100 + grid[0, 1] * 10 + grid[1, 2];
pt.x := pt.x + inp;
r_pt := pt.x * 100 + pt.y + pt2.x * 10 + pt2.y;
pts[1] := pt;
pts[2].y := pts[1].x;
r_pts := pts[0].x + pts[1].x + pts[2].y;
r2 := r;
r2[1] := 0;
r_copy := r[1] + r2[1] + r2[2];
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
