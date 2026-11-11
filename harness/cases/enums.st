(* Enumerations and types with named values (Ed.3 6.4.4.2, 6.4.4.3).
   col has named values: red 1, green 5, blue one more, 6; its initial
   value green. A col is used as the INT it is: other values assigned,
   arithmetic, comparison. mode is an enumeration, its initial value run.
   An enumeration is a FUNCTION's input, output and result and a block's
   in-out; a result or an input not given starts as the type's initial
   value. Worked by hand, k %IB0 1, 2, 3:
   %QB0: c as the tick starts: 5 (green), then what the tick before left,
   3, 4. %QB1: x := c + blue when c = green: 11, and kept.
   c := k, then CASE c OF red: 10; green - 3 (2), blue: 20; ELSE 30:
   %QB2 10, 20, 30.
   b(c := c, m := m, d := red + 1): c + 2, %QB3 3, 4, 5; m := NEXT(m):
   run gives stop; stop matches no label, so NEXT keeps its initial value,
   run; then stop again: %QB4 2, 1, 2 (stop 2, run 1).
   lv#hi OR lo, a WORD: 16#30, %QB5.
   n, an INT, given to the in-out c: 10 + blue each tick: %QB6 16, 22, 28.
   m3 := DEF(o => m4), both idle before: DEF's result is not set and its
   input m not given, so both are run: %QB7 1.
   %QB8 blue, 6. %QB9 a[2], an array's element of col: green, 5.
   mi : (low, mid, top) := mid, declared in place: mid gives 7 and
   becomes top, top gives 8: %QB10 7, 8, 8.
   reference-skip: named values are Ed.3's; the reference compiler
   (Ed.2) has none. *)
TYPE
  col : INT (red := 1, green := 5, blue) := green;
  lv : WORD (lo := 16#10, hi := 16#20);
  mode : (idle, run, stop) := run;
END_TYPE

FUNCTION NEXT : mode
  VAR_INPUT m : mode; END_VAR
  CASE m OF
    idle: NEXT := run;
    run: NEXT := stop;
  END_CASE;
END_FUNCTION

FUNCTION DEF : mode
  VAR_INPUT m : mode; END_VAR
  VAR_OUTPUT o : mode; END_VAR
  o := m;
END_FUNCTION

FUNCTION_BLOCK BUMP
  VAR_IN_OUT c : col; m : mode; END_VAR
  VAR_INPUT d : col; END_VAR
  c := c + d;
  m := NEXT(m);
END_FUNCTION_BLOCK

PROGRAM main
  VAR
    c : col;
    x : INT;
    w : WORD;
    m, m2 : mode;
    m3, m4 : mode := idle;
    b : BUMP;
    n : INT := 10;
    a : ARRAY [1..2] OF col;
    mi : (low, mid, top) := mid;
  END_VAR
  VAR k AT %IB0 : SINT; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : BYTE;
    q6 AT %QB6 : SINT;
    q7 AT %QB7 : SINT;
    q8 AT %QB8 : SINT;
    q9 AT %QB9 : SINT;
    q10 AT %QB10 : SINT;
  END_VAR
  q0 := INT_TO_SINT(c);
  IF c = col#green THEN
    x := c + blue;
  END_IF;
  q1 := INT_TO_SINT(x);
  c := k;
  CASE c OF
    red: q2 := 10;
    green - 3, blue: q2 := 20;
  ELSE
    q2 := 30;
  END_CASE;
  b(c := c, m := m, d := red + 1);
  q3 := INT_TO_SINT(c);
  IF m = stop THEN
    q4 := 2;
  ELSIF m = mode#run THEN
    q4 := 1;
  ELSE
    q4 := 0;
  END_IF;
  w := lv#hi OR lo;
  q5 := WORD_TO_BYTE(w);
  b(c := n, m := m2, d := blue);
  q6 := INT_TO_SINT(n);
  m3 := DEF(o => m4);
  IF m3 = run AND m4 = run THEN
    q7 := 1;
  END_IF;
  q8 := INT_TO_SINT(blue);
  q9 := INT_TO_SINT(a[2]);
  IF mi = mid THEN
    q10 := 7;
    mi := top;
  ELSIF mi = top THEN
    q10 := 8;
  END_IF;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
