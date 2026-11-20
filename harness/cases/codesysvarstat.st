(* Static variables (var-stat), as TcOpen's messenger and CRC blocks
   declare them: one of each for the POU, shared by every instance and
   kept from call to call. Worked by hand, two cycles:
   cycle 0: a() n 5 -> 6, b() 7; a.Bump() k 1, n 17; b.Bump() k 2, n 27:
     %QW0 x = 1 + 2 = 3; F twice through its loop: %QW2 2;
     m(), More's body only: %QW4 r = 27 * 2 = 54; %QW6 a.q = 6.
   cycle 1: a() 28, b() 29; Bump 3 (n 39), 4 (n 49): %QW0 7;
     F: %QW2 4; %QW4 98; %QW6 28.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Cnt
VAR_STAT
  n : INT := 5;
END_VAR
VAR_OUTPUT q : INT; END_VAR
METHOD PUBLIC Bump : INT
VAR_STAT k : INT; END_VAR
k := k + 1;
n := n + 10;
Bump := k;
END_METHOD
n := n + 1;
q := n;
END_FUNCTION_BLOCK

FUNCTION_BLOCK More EXTENDS Cnt
VAR_OUTPUT r : INT; END_VAR
r := n * 2;
END_FUNCTION_BLOCK

FUNCTION F : INT
VAR_STAT calls : INT; i : INT; END_VAR
FOR i := 1 TO 2 DO
  calls := calls + 1;
END_FOR;
F := calls;
END_FUNCTION

PROGRAM main
  VAR a, b : Cnt; m : More; END_VAR
  VAR x AT %QW0 : INT; y AT %QW2 : INT; z AT %QW4 : INT; w AT %QW6 : INT; END_VAR
  a();
  b();
  x := a.Bump() + b.Bump();
  y := F();
  m();
  z := m.r;
  w := a.q;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
