(* Located variables without a name (Ed.3 Loc_Var_Decl: Variable_Name ?)
   and a FOR control variable at an address. Worked by hand:
   AT %QW4 : INT := 5 sets %QW4 to 5 (Q bytes 4, 5: 05 00) as the
   program is set up; AT %MB0 : ARRAY [1..2] OF BYTE := [1, 2] sets
   %MB0, %MB1 (01 02); AT %QB9 : BYTE := 7 sets %QB9 (07) -
   in the PROGRAM: a block's VAR has no full address (Ed.3
   Other_Var_Decls).
   FOR i AT %MW2 := 1 TO 3 adds i to n: 6 (%QB0); i is 4 after it
   (%MW2, 04 00).
   FOR j AT %MW4 := 1 TO 5, left by EXIT at 3: j is 3 (%MW4, 03 00).
   FOR k AT %MW6 := 1 TO 10, the body adding 2 to k: k 1, 3; 4, 6; 7,
   9; 10, 12; then 13 is past 10: four passes (%QB1 4), k 13 (%MW6,
   0d 00).
   FOR z AT %MW8 := 5 TO 1 does not run: z is 5 (%MW8, 05 00).
   reference-skip: a located variable without a name, an array at an
   address: refused. *)
FUNCTION_BLOCK BT
  VAR n : INT; END_VAR
  n := n + 1;
END_FUNCTION_BLOCK
PROGRAM main
  VAR AT %QW4 : INT := 5; END_VAR
  VAR AT %MB0 : ARRAY [1..2] OF BYTE := [1, 2]; END_VAR
  VAR AT %QB9 : BYTE := 7; END_VAR
  VAR
    i AT %MW2 : INT;
    j AT %MW4 : INT;
    k AT %MW6 : INT;
    z AT %MW8 : INT;
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
  END_VAR
  VAR
    b : BT;
    n : INT;
    cnt : INT;
  END_VAR
  b();
  n := 0;
  FOR i := 1 TO 3 DO
    n := n + i;
  END_FOR;
  q0 := INT_TO_SINT(n);
  FOR j := 1 TO 5 DO
    IF j = 3 THEN
      EXIT;
    END_IF;
  END_FOR;
  cnt := 0;
  FOR k := 1 TO 10 DO
    k := k + 2;
    cnt := cnt + 1;
  END_FOR;
  q1 := INT_TO_SINT(cnt);
  FOR z := 5 TO 1 DO
    cnt := 99;
  END_FOR;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
