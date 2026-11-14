(* The initial value of a variable at an address that its type gives
   without the declaration saying so (Ed.3 6.4.4.2.2, 6.4.4.4.2): an
   enumeration starts at its first value, a subrange at its lower limit,
   and a structure or array holding them likewise. Until 2026-11-13 these
   were left at 0 when no initial value was written out. Worked by hand:

   init: e (Mode, first value Run = 5) at %QW0: 05 00; lim, SINT(3..5),
   at %QB2: 03; h, a Hold (m Mode, s SINT(-2..2)), at %QW3: 05 00 fe;
   arr, two Mode at %QW6: 05 00 05 00; zero, SINT(0..9) at %QB10 starts
   at 0: 00. ein, a Mode at %IW0, is the platform's: not written, and
   so ein = Run is FALSE: %QB11 00 at tick 0.
   tick 0: e := Halt (06 00); the rest as at init.
   reference-skip: enumerated values, arrays and structures at an
   address: refused. *)
TYPE
  Mode : (Run := 5, Halt);
  Hold : STRUCT m : Mode; s : SINT(-2..2); END_STRUCT;
END_TYPE
PROGRAM main
  VAR
    e AT %QW0 : Mode;
    lim AT %QB2 : SINT(3..5);
    h AT %QW3 : Hold;
    arr AT %QW6 : ARRAY [0..1] OF Mode;
    zero AT %QB10 : SINT(0..9);
    copy AT %QB11 : BYTE;
  END_VAR
  VAR
    ein AT %IW0 : Mode;
  END_VAR
  copy := BOOL_TO_BYTE(ein = Run);
  e := Halt;
END_PROGRAM
CONFIGURATION plant
  RESOURCE r ON PLC
    TASK t (INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
