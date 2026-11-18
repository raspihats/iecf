(* An initial value that faults as its variable is set up (the
   initial-expressions extension, docs/extensions.md): z is 0, so Quot(x,
   z) divides by zero in Quot, at its line (8). The fault stops the
   set-up there, as a statement's stops its body, and the program is
   stopped until the next start (docs/c-output.md §5): no task runs, and
   %QB0 stays 0.
   reference-skip: a variable in an initial value is not IEC 61131-3. *)
FUNCTION Quot : INT VAR_INPUT a, b : INT; END_VAR Quot := a / b; END_FUNCTION

PROGRAM main
  VAR
    z : INT;
    x : INT := 7;
    y : INT := Quot(x, z);
    w : INT := 3;
  END_VAR
  VAR q0 AT %QB0 : SINT; END_VAR
  q0 := INT_TO_SINT(x + y + w);
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
