(* A manual mode (docs/c-output.md §3): while the platform holds the
   charts no transition is cleared, though its condition is TRUE; the
   steps stay as they are, or as the platform enters and leaves them,
   and their actions run as for any step - a P action on a step the
   platform entered, with its final scan, and the step's time from that
   entry. Freed, the chart goes on from the steps it then has. The x
   outputs show the active steps, npulse the P action's runs, late that
   S2 has been active 20 ms. A scan is 10 ms. iecf's alone (reference-skip), its
   trace checked by hand. *)
PROGRAM main
VAR
  go AT %IX0.0 : BOOL;
  x0 AT %QX0.0 : BOOL;
  x1 AT %QX0.1 : BOOL;
  x2 AT %QX0.2 : BOOL;
  npulse AT %QB1 : USINT;
  late AT %QX0.3 : BOOL;
END_VAR

INITIAL_STEP S0:
  x0(N);
END_STEP
STEP S1:
  x1(N);
END_STEP
STEP S2:
  x2(N);
  Pulse(P);
  Clock(N);
END_STEP

TRANSITION FROM S0 TO S1 := go; END_TRANSITION
TRANSITION FROM S1 TO S2 := go; END_TRANSITION
TRANSITION FROM S2 TO S0 := NOT go; END_TRANSITION

ACTION Pulse:
  npulse := npulse + 1;
END_ACTION
ACTION Clock:
  late := S2.T >= T#20ms;
END_ACTION
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON cpu
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
