(* AND_THEN and OR_ELSE (short-circuit): the right operand evaluated
   only when the left does not decide, as CODESYS and TwinCAT have them.
   c.Hit() counts its calls. Worked by hand:
   cycle 0: FALSE AND_THEN c.Hit(): not called, FALSE; TRUE AND_THEN
     c.Hit(): called (n 1), TRUE; TRUE OR_ELSE c.Hit(): not called, TRUE;
     FALSE OR_ELSE c.Hit(): called (n 2), TRUE. %QW0 n = 2;
     %QW2 r1 + 2 r2 + 4 r3 + 8 r4 = 0 + 2 + 4 + 8 = 14.
     d = 0: d <> 0 AND_THEN 10 / d > 1 divides not, no fault: %QW4 7.
     The same in an IF's ELSIF and a WHILE, each Hit() a call more only
     where the left lets it: k 1 (ELSIF taken), %QW6 = n after = 3.
   cycle 1: n from 3: 5, 14, 7; %QW6 6.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Counter
VAR_OUTPUT n : INT; END_VAR
METHOD PUBLIC Hit : BOOL
n := n + 1;
Hit := TRUE;
END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR c : Counter; f : BOOL; t : BOOL := TRUE; r1, r2, r3, r4 : BOOL; d, k : INT; END_VAR
  VAR q AT %QW0 : INT; x AT %QW2 : INT; w AT %QW4 : INT; v AT %QW6 : INT; END_VAR
  r1 := f AND_THEN c.Hit();
  r2 := t AND_THEN c.Hit();
  r3 := t OR_ELSE c.Hit();
  r4 := f OR_ELSE c.Hit();
  q := c.n;
  x := BOOL_TO_INT(r1) + 2 * BOOL_TO_INT(r2) + 4 * BOOL_TO_INT(r3) + 8 * BOOL_TO_INT(r4);
  w := 0;
  IF d <> 0 AND_THEN 10 / d > 1 THEN
    w := 1;
  END_IF;
  w := w + 7;
  k := 0;
  IF f AND_THEN c.Hit() THEN
    k := 2;
  ELSIF t AND_THEN c.Hit() THEN
    k := 1;
  END_IF;
  WHILE k > 0 AND_THEN t DO
    k := k - 1;
  END_WHILE;
  v := c.n;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
