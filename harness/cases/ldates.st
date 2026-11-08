(* LDATE, LTOD and LDT (Ed.3 Table 10, 22, 30, 36; docs/dependencies.md):
   signed 64-bit nanoseconds, LDT from 1677-09-21 to 2262-04-11. An LTOD
   and an LDT go on 7 hours a tick, from LTOD#20:00:00.000000001 and
   LDT#1969-12-31-20:00:00.5; each tick shows, as bits, comparisons of
   what the operators, conversions and Table 36 give. An LTOD wraps at
   midnight; a long value made a short one rounds down. iecf only
   (reference-skip: the reference compiler has no L dates). *)
PROGRAM main
VAR
  r_bits AT %MW0 : WORD;
  r_year AT %MW2 : UINT;
  r_ms AT %MW4 : UINT;
END_VAR
VAR
  clock : LTOD := LTOD#20:00:00.000_000_001;
  stamp : LDT := LDT#1969-12-31-20:00:00.5;
  start : LDT := LDT#1969-12-31-00:00:00;
  day : LDATE;
  span : LTIME;
  mo, da, h, mi, s : UINT;
END_VAR
clock := clock + T#7h;
stamp := ADD_LDT_LTIME(stamp, LTIME#7h);
day := LDT_TO_LDATE(stamp);
span := T#1500ms;
r_bits := 0;
IF clock < LTOD#12:00:00 THEN r_bits := r_bits OR 16#0001; END_IF;
IF LDT_TO_LTOD(stamp) - clock = LTIME#499_999_999ns THEN r_bits := r_bits OR 16#0002; END_IF;
IF stamp - start > LTIME#1d THEN r_bits := r_bits OR 16#0004; END_IF;
IF SUB_LTOD_LTOD(LTOD#01:00:00, LTOD#23:00:00) = LTIME#-22h THEN r_bits := r_bits OR 16#0008; END_IF;
IF CONCAT_DATE_LTOD(LDT_TO_DATE(stamp), LDT_TO_LTOD(stamp)) = stamp THEN r_bits := r_bits OR 16#0010; END_IF;
IF LDT_TO_LDATE(LDT#1969-12-31-23:59:59.999999999) = LD#1969-12-31 THEN r_bits := r_bits OR 16#0020; END_IF;
IF LDT_TO_DT(stamp) = DT#1970-01-01-03:00:00 THEN r_bits := r_bits OR 16#0040; END_IF;
IF SUB_LDT_LTIME(stamp, LTIME#1d) < start THEN r_bits := r_bits OR 16#0080; END_IF;
IF LTOD_TO_TOD(clock) = TOD#03:00:00 THEN r_bits := r_bits OR 16#0100; END_IF;
IF LTIME_TO_TIME(LTIME#-1500us) = T#-2ms THEN r_bits := r_bits OR 16#0200; END_IF;
IF span = LTIME#1500ms THEN r_bits := r_bits OR 16#0400; END_IF;
IF DT_TO_LDT(DT#2026-11-08-12:00:00) = LDT#2026-11-08-12:00:00 THEN r_bits := r_bits OR 16#0800; END_IF;
IF clock - LTIME#21h = LTOD#06:00:00.000000001 THEN r_bits := r_bits OR 16#1000; END_IF;
IF STRING_TO_LDT(LDT_TO_STRING(stamp)) = stamp THEN r_bits := r_bits OR 16#2000; END_IF;
IF MAX(stamp, start) = stamp THEN r_bits := r_bits OR 16#4000; END_IF;
IF day = LD#1970-01-02 THEN r_bits := r_bits OR 16#8000; END_IF;
SPLIT_LDT(LDT#1677-09-21-00:12:43.145224192, r_year, mo, da, h, mi, s, r_ms);
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
