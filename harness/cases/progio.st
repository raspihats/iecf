(* A program's VAR_IN_OUT (Ed.3 Table 47 2c), connected in the
   CONFIGURATION to a global (6.8.2): each instance of `bump` reads and
   writes the global itself, through its in-out. Two instances on one
   task add 1 and 10 to `shared` each tick, a third program copies it to
   %MW0; `other` is never touched. The reference compiler cannot compile
   program connections (PROPOSAL.md §1b): iecf only, a hand-checked
   trace. *)
PROGRAM bump
VAR_INPUT amount : INT; END_VAR
VAR_IN_OUT n : INT; END_VAR
n := n + amount;
END_PROGRAM

PROGRAM show
VAR_EXTERNAL shared, other : INT; END_VAR
VAR
  r_shared AT %MW0 : INT;
  r_other AT %MW2 : INT;
END_VAR
r_shared := shared;
r_other := other;
END_PROGRAM

CONFIGURATION plant
  VAR_GLOBAL shared : INT := 100; other : INT := 7; END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM one WITH fast : bump(amount := 1, n := shared);
    PROGRAM ten WITH fast : bump(amount := 10, n := shared);
    PROGRAM s WITH fast : show;
  END_RESOURCE
END_CONFIGURATION
