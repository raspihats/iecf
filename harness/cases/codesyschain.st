(* Found in OSCAT BUILDING (BOILER): chained assignment, read right to
   left, each inner target read back (chained-assignment). Worked by hand:
   %QD0: r := x := 7 / 2: x is INT, 7 / 2 is 3, r gets 3.0, 16#40400000.
   %QW4: x, 3.
   %QW6, %QW8: q := arr[i] := y := 9, i 2: arr[2] 9, q 9.
   %QW10: inside IF, a := b := c := 5 then a + b + c: 15.
   reference-skip: not IEC 61131-3. *)
PROGRAM main
  VAR
    x : INT; y : SINT; i : INT := 2; arr : ARRAY[0..3] OF INT;
    a : INT; b : INT; c : INT;
  END_VAR
  VAR
    r AT %QD0 : REAL; qx AT %QW4 : INT; q AT %QW6 : INT; q2 AT %QW8 : INT;
    q3 AT %QW10 : INT;
  END_VAR
  r := x := 7 / 2;
  qx := x;
  q := arr[i] := y := 9;
  q2 := arr[2];
  IF i = 2 THEN
    a := b := c := 5;
  END_IF;
  q3 := a + b + c;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
