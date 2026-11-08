(* Variables at an address (Ed.3 6.5.5.3; docs/c-output.md §4): the
   address is where a variable starts, and it takes its own type's width
   from there, little-endian - a TIME, DATE or TOD 4 bytes, an LTIME or
   LDT 8, whatever the size prefix says (W0004 when it differs). A BOOL
   at a byte takes the whole byte: TRUE written as 1, any value but 0
   read as TRUE. A program's TIME output connected to an LTIME global at
   an address is converted, ms to ns. iecf only (reference-skip: the
   reference compiler refuses an LTIME at an address, and holds a TIME,
   DATE, TOD or DT as a structure of seconds and nanoseconds, not 4
   bytes). *)
PROGRAM main
VAR_INPUT
  tick : TIME;
END_VAR
VAR_OUTPUT
  el_out : TIME;
END_VAR
VAR
  el AT %MD0 : TIME;
  big AT %ML4 : LTIME;
  day AT %MD12 : DATE;
  odd AT %MW16 : TIME;
  r AT %MB20 : REAL;
  toggle AT %QB0 : BOOL;
  seen AT %QX1.0 : BOOL;
  inb AT %IB0 : BOOL;
END_VAR
VAR
  n : INT;
END_VAR
n := n + 1;
el := el - tick;
big := el;
IF n = 2 THEN day := D#2026-11-08; END_IF;
odd := el + T#1ms;
r := INT_TO_REAL(n) / 2.0;
toggle := NOT toggle;
seen := inb;
el_out := el;
END_PROGRAM

CONFIGURATION plant
  VAR_GLOBAL
    g AT %ML24 : LTIME;
  END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main(tick := T#250ms, el_out => g);
  END_RESOURCE
END_CONFIGURATION
