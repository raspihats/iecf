(* Found in OSCAT NETWORK and BUILDING, each an extension. Worked by hand:
   %QB0 bit 0: UDINT 70000 > DWORD 16#10000 (65536): a bit string compared
   with an integer, as its unsigned integer (bit-string-arithmetic): TRUE.
   %QB0 bit 1: BYTE 200 < INT 100: FALSE.
   %QW2: TOD_TO_TIME(TOD#01:00:00) is T#1h, as DINT ms / 1000, 3600.
   %QD4: TIME_TO_TOD(T#1d2h3m4s5ms) drops the day: TOD#02:03:04.005,
   7384005 ms (date-number-conversions).
   %QD8: D#2026-11-19 + T#1d - T#2d is D#2026-11-18; DATE_TO_UDINT /
   86400 counts its days since 1970-01-01: 20775
   (date-time-arithmetic).
   %QB12: d2 := d + half, half T#12h a variable: no whole day, flagged
   and left undone, d2 stays D#1970-01-01: DATE_TO_UDINT(d2) = 0.
   reference-skip: none of these is IEC 61131-3. *)
PROGRAM main
  VAR
    u : UDINT := 70000; w : DWORD := 16#10000; b : BYTE := 200; i : INT := 100;
    d : DATE := D#2026-11-19; d2 : DATE; half : TIME := T#12h;
  END_VAR
  VAR
    x0 AT %QX0.0 : BOOL; x1 AT %QX0.1 : BOOL; q1 AT %QW2 : INT; q2 AT %QD4 : UDINT;
    q3 AT %QD8 : UDINT; q4 AT %QB12 : BYTE;
  END_VAR
  x0 := u > w;
  x1 := b < i;
  q1 := DINT_TO_INT(TIME_TO_DINT(TOD_TO_TIME(TOD#01:00:00)) / 1000);
  q2 := TOD_TO_UDINT(TIME_TO_TOD(T#1d2h3m4s5ms));
  q3 := DATE_TO_UDINT(d + T#1d - T#2d) / 86400;
  d2 := d + half;
  q4 := UDINT_TO_BYTE(DATE_TO_UDINT(d2));
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
