(* Found in TcOpen (TwinCAT), 10639 ends without ';' against 829 with,
   and in OSCAT: END_IF and its kin with no ';' (optional-semicolons).
   Worked by hand:
   %QW0: n counts 1 + 2 + 3 + 4 by FOR, then +10 by IF: 20.
   %QW2: WHILE doubles w from 1 past 100: 128; REPEAT halves it to 64,
   then CASE adds 1 for 64: 65.
   reference-skip: not IEC 61131-3. *)
PROGRAM main
  VAR n : INT; i : INT; w : INT := 1; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; END_VAR
  n := 0;
  FOR i := 1 TO 4 DO
    n := n + i;
  END_FOR
  IF n = 10 THEN
    n := n + 10;
  END_IF
  WHILE w <= 100 DO
    w := w * 2;
  END_WHILE
  REPEAT
    w := w / 2;
  UNTIL w <= 100 END_REPEAT
  CASE w OF
    64: w := w + 1;
  END_CASE
  q0 := n;
  q1 := w;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
