(* DATE, TOD and DT (Ed.3 Table 30; docs/dependencies.md): a TOD and a DT
   go on 7 hours a tick, from TOD#20:00:00 and DT#2026-11-07-22:00:00; each
   tick shows, as bits, comparisons of what Table 30's operators and
   functions give (DAY_OF_WEEK, which matiec lacks, is
   runtime/tests/dates.c's). A TOD wraps at midnight in
   iecf, where matiec's goes on past 24 hours: at tick 0 the TOD is
   TOD#03:00:00 in iecf and 27 hours in matiec. iecf's trace is
   dates.iecf.trace. *)
PROGRAM main
VAR
  r_bits AT %MW0 : WORD;
END_VAR
VAR
  clock : TOD := TOD#20:00:00;
  stamp : DT := DT#2026-11-07-22:00:00;
  day : DATE;
  start : DT := DT#2026-11-07-00:00:00;
END_VAR
clock := clock + T#7h;
stamp := ADD_DT_TIME(stamp, T#7h);
day := DT_TO_DATE(stamp);
r_bits := 0;
IF clock < TOD#12:00:00 THEN r_bits := r_bits OR 16#0001; END_IF;
IF stamp > DT#2026-11-08-12:00:00 THEN r_bits := r_bits OR 16#0002; END_IF;
IF stamp - start > T#1d THEN r_bits := r_bits OR 16#0004; END_IF;
IF DT_TO_TOD(stamp) - clock = T#2h THEN r_bits := r_bits OR 16#0008; END_IF;
IF CONCAT_DATE_TOD(day, DT_TO_TOD(stamp)) = stamp THEN r_bits := r_bits OR 16#0010; END_IF;
IF SUB_DATE_DATE(day, D#2026-11-07) >= T#1d THEN r_bits := r_bits OR 16#0020; END_IF;
IF day = D#2026-11-09 THEN r_bits := r_bits OR 16#0040; END_IF;
IF SUB_DT_TIME(stamp, T#1d) < start THEN r_bits := r_bits OR 16#0080; END_IF;
IF clock - T#21h = TOD#06:00:00 THEN r_bits := r_bits OR 16#0100; END_IF;
IF SUB_TOD_TOD(TOD#01:00:00, TOD#23:00:00) = T#-22h THEN r_bits := r_bits OR 16#0200; END_IF;
IF MAX(clock, TOD#10:00:00) = clock THEN r_bits := r_bits OR 16#0400; END_IF;
IF DT_TO_DATE(DT#2026-11-07-23:59:59) = D#2026-11-07 THEN r_bits := r_bits OR 16#0800; END_IF;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
