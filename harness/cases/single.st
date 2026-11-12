(* A TASK triggered by SINGLE, and a TASK whose INTERVAL is a variable
   (Ed.3 6.8.2: Data_Source; rules a and b). The tick is 10 ms, the
   constant task's interval.
   ev (SINGLE := %IX0.0, no INTERVAL) runs on each rising edge of
   %IX0.0, which is TRUE at ticks 1, 2, 4 and FALSE at 0, 3, 5: it runs
   at ticks 1 and 4. Its count: 0 1 1 1 2 2 (%QB0).
   both (SINGLE := go, INTERVAL := T#20ms), go a global the program c
   sets from %IX0.1, TRUE at tick 3 only (c runs first, priority 0): the
   edge at 3, and at the interval at ticks 0, 2, 4 while go is FALSE:
   it runs at 0, 2, 3, 4. Count: 1 1 2 3 4 4 (%QB1).
   per (INTERVAL := iv, iv a global the program c sets: T#20ms, then
   T#30ms from tick 3 on): at 0, 2 (20 ms), then 3 (30 ms divides 30).
   Count: 1 1 2 3 3 3 (%QB2).
   reference-skip: the reference compiler's host knows no SINGLE. *)
PROGRAM count
  VAR_INPUT go : BOOL; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  n := n + 1;
END_PROGRAM
PROGRAM ctl
  VAR_EXTERNAL go : BOOL; iv : TIME; END_VAR
  VAR i1 AT %IX0.1 : BOOL; t AT %IB1 : BYTE; END_VAR
  go := i1;
  IF t >= 3 THEN
    iv := T#30ms;
  END_IF;
END_PROGRAM
PROGRAM show
  VAR_EXTERNAL a : INT; b : INT; c2 : INT; END_VAR
  VAR q0 AT %QB0 : SINT; q1 AT %QB1 : SINT; q2 AT %QB2 : SINT; END_VAR
  q0 := INT_TO_SINT(a);
  q1 := INT_TO_SINT(b);
  q2 := INT_TO_SINT(c2);
END_PROGRAM

CONFIGURATION cf
  VAR_GLOBAL go : BOOL; iv : TIME := T#20ms; a : INT; b : INT; c2 : INT; END_VAR
  RESOURCE r ON x
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    TASK ev(SINGLE := %IX0.0, PRIORITY := 1);
    TASK both(SINGLE := go, INTERVAL := T#20ms, PRIORITY := 2);
    TASK per(INTERVAL := iv, PRIORITY := 3);
    TASK last(INTERVAL := T#10ms, PRIORITY := 9);
    PROGRAM c WITH fast : ctl;
    PROGRAM p1 WITH ev : count(n => a);
    PROGRAM p2 WITH both : count(n => b);
    PROGRAM p3 WITH per : count(n => c2);
    PROGRAM s WITH last : show;
  END_RESOURCE
END_CONFIGURATION
