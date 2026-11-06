(* The ST statements: IF/ELSIF/ELSE, CASE with lists and ranges, FOR up
   and down with BY, WHILE, REPEAT, EXIT, RETURN. *)
PROGRAM main
VAR
  v AT %IW0 : INT;
  r_if AT %MW0 : INT;
  r_case AT %MW2 : INT;
  r_for AT %MD4 : DINT;
  r_down AT %MD8 : DINT;
  r_while AT %MW12 : INT;
  r_repeat AT %MW14 : INT;
  r_exit AT %MW16 : INT;
  r_ret AT %MW18 : INT;
END_VAR
VAR
  i : INT;
  j : INT;
END_VAR
IF v < 0 THEN
  r_if := -1;
ELSIF v = 0 THEN
  r_if := 0;
ELSIF v < 10 THEN
  r_if := 1;
ELSE
  r_if := 2;
END_IF;

CASE v OF
  0: r_case := 100;
  1, 2, 3: r_case := 200;
  4..9: r_case := 300 + v;
  -5..-1: r_case := -300;
ELSE
  r_case := 999;
END_CASE;

r_for := 0;
FOR i := 1 TO v BY 2 DO
  r_for := r_for + INT_TO_DINT(i);
END_FOR;
r_down := 0;
FOR i := 10 TO 1 BY -3 DO
  r_down := r_down * 10 + INT_TO_DINT(i);
END_FOR;

r_while := 0;
i := v;
WHILE i > 0 DO
  r_while := r_while + 1;
  i := i / 2;
END_WHILE;

r_repeat := 0;
REPEAT
  r_repeat := r_repeat + 1;
UNTIL r_repeat >= v
END_REPEAT;

r_exit := 0;
FOR i := 1 TO 10 DO
  FOR j := 1 TO 10 DO
    IF j > i THEN
      EXIT;
    END_IF;
    r_exit := r_exit + 1;
  END_FOR;
  IF i >= v THEN
    EXIT;
  END_IF;
END_FOR;

r_ret := 1;
IF v = 7 THEN
  RETURN;
END_IF;
r_ret := 2;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
