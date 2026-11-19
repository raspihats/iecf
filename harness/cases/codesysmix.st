(* Two generator fixes found building the OSCAT NETWORK and BUILDING
   libraries. Worked by hand:
   %QW0: a duration times a BYTE (bit-string-arithmetic): T#100ms *
   BYTE#3 is T#300ms, as DINT 300 (BUILDING's ACTUATOR_2P).
   %QW2: inside Cron a block input named Bump shadows the FUNCTION Bump,
   which writes a global (a call with effects, so its statement is
   sequenced); arr[1] := Bump(5) calls the function: g 10 + 5 = 15, and
   g becomes 11 (NETWORK's DLOG_CRON_TAB).
   %QW4: g after the call, 11.
   reference-skip: both forms are CODESYS's, not IEC 61131-3. *)
VAR_GLOBAL g : INT := 10; END_VAR

FUNCTION Bump : INT
  VAR_INPUT x : INT; END_VAR
  VAR_EXTERNAL g : INT; END_VAR
  Bump := g + x;
  g := g + 1;
END_FUNCTION

FUNCTION_BLOCK Cron
  VAR_INPUT bump : STRING[10]; END_VAR
  VAR_OUTPUT arr : ARRAY[0..2] OF INT; END_VAR
  arr[1] := Bump(5);
END_FUNCTION_BLOCK

PROGRAM main
  VAR c : Cron; t : TIME; k : BYTE := 3; END_VAR
  VAR_EXTERNAL g : INT; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; END_VAR
  t := T#100ms * k;
  q0 := DINT_TO_INT(TIME_TO_DINT(t));
  c(bump := 'x');
  q1 := c.arr[1];
  q2 := g;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
