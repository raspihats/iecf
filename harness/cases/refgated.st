(* REF() of an element whose index is a call with EN and ENO: the call
   made once, its ENO taken. Worked by hand: the index is ADD(1, 2), 3,
   so r refers to a[3], 30 (Q0 1e); ENO TRUE (Q1 01).
   reference-skip: REF_TO and EN/ENO, as `refimage`. *)
PROGRAM main
  VAR a : ARRAY [1..4] OF INT := [10, 20, 30, 40]; r : REF_TO INT; b : BOOL := TRUE; ok : BOOL; END_VAR
  VAR q AT %QB0 : USINT; q1 AT %QB1 : USINT; END_VAR
  r := REF(a[ADD(EN := b, IN1 := 1, IN2 := 2, ENO => ok)]);
  q := INT_TO_USINT(r^);
  q1 := BOOL_TO_USINT(ok);
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION
