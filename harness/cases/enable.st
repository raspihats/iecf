(* EN and ENO on a call (Ed.3 6.6.1.5; docs/behaviour/en-eno.md): with
   EN FALSE the call does not happen and its statement is left undone,
   the target keeping its value; ENO is FALSE then, and when the
   function's own operation flags. %IX0.0 is EN over the ticks; the
   divisor %IW2 is 0 at tick 2, when EN is FALSE, so DIV is not called
   and nothing faults. The reference compiler, by its own reading, assigns the function's
   default result when EN is FALSE, and keeps ENO TRUE through an
   overflow: iecf's trace is enable.iecf.trace. *)
FUNCTION twice : INT
VAR_INPUT x : INT; END_VAR
twice := x * 2;
END_FUNCTION

PROGRAM main
VAR
  go AT %IX0.0 : BOOL;
  d AT %IW2 : INT;
  r_div AT %MW0 : INT;
  r_twice AT %MW2 : INT;
  r_eno AT %MB4 : BYTE;
END_VAR
VAR
  ok1, ok2, ok3 : BOOL;
  n : INT := 16000;
END_VAR
r_div := DIV(EN := go, IN1 := INT#100, IN2 := d, ENO => ok1);
r_twice := twice(EN := go, x := n, ENO => ok2);
n := n + 100;
r_eno := 0;
IF ok1 THEN r_eno := r_eno OR 1; END_IF;
IF ok2 THEN r_eno := r_eno OR 2; END_IF;
END_PROGRAM

CONFIGURATION plant
  RESOURCE res ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
