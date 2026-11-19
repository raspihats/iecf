(* A pointer read past its variable (the pointers extension): a fault,
   IECF_FAULT_INDEX, the task stopped where the read leaves the variable,
   never C's undefined behaviour. Worked by hand: a has 4 bytes, 1 2 3 4;
   the loop adds them through pt for i 0..4, so the fifth read, past the
   end, faults on line 21 in the first tick, n holding 10 so far, and
   the task stops there: %QB0, set after the loop, stays 0. (The host
   leaves the fault set, so a later tick stops at once: whether to run
   again is the platform's; %QB1 is the first statement's, 0.)
   reference-skip: pointers are CODESYS's, not IEC 61131-3. *)
PROGRAM main
  VAR
    a : ARRAY[0..3] OF BYTE := [1, 2, 3, 4];
    pt : POINTER TO BYTE;
    i, n : INT;
  END_VAR
  VAR q0 AT %QB0 : SINT; q1 AT %QB1 : SINT; END_VAR
  q1 := INT_TO_SINT(n);
  n := 0;
  pt := ADR(a);
  FOR i := 0 TO 4 DO
    n := n + BYTE_TO_INT(pt^);
    pt := pt + 1;
  END_FOR;
  q0 := INT_TO_SINT(n);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
