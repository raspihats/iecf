(* A hierarchical address the platform has not (docs/c-output.md §4):
   the harness's iecf_address gives NULL for a level of 99, so the start
   faults with IECF_FAULT_IMAGE before anything runs, and no task runs.
   hierfault.iecf.trace: the reference compiler takes the address as a
   variable of its own, with no platform to ask, so nothing faults. *)
PROGRAM main
  VAR x AT %IW99.1 : INT; END_VAR
  VAR q AT %QW0 : INT; END_VAR
  q := x + 1;
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION
