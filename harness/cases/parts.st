(* CONCAT_DATE, CONCAT_TOD, CONCAT_DT and the SPLITs (Ed.3 Table 36;
   docs/dependencies.md): a month from 11 and an hour from 22 go up by one
   each tick, joined into a DATE, a TOD and a DT and split back. The parts
   are valid at tick 1 (2026-12-31, 23:30:15.250); a day not in its month
   (November 31st, at tick 0), a month 13 or an hour 24 gives the type's
   default value in iecf, flagged; iecf's trace is parts.iecf.trace. *)
PROGRAM main
VAR
  r_year AT %MW0 : UINT;
  r_month AT %MB2 : USINT;
  r_day AT %MB3 : USINT;
  r_hour AT %MB4 : USINT;
  r_min AT %MB5 : USINT;
  r_sec AT %MB6 : USINT;
  r_ms AT %MW8 : UINT;
  r_dt_month AT %MB10 : USINT;
  r_dt_hour AT %MB11 : USINT;
END_VAR
VAR
  month : INT := 10;
  hour : SINT := 21;
  d : DATE;
  t : TOD;
  s : DT;
  y, m, dd : UDINT;
END_VAR
month := month + 1;
hour := hour + 1;
d := CONCAT_DATE(2026, month, 31);
t := CONCAT_TOD(hour, 30, 15, 250);
s := CONCAT_DT(YEAR := 2026, MONTH := month, DAY := 28, HOUR := hour,
               MINUTE := 59, SECOND := 59, MILLISECOND := 999);
SPLIT_DATE(d, y, m, dd);
r_year := UDINT_TO_UINT(y);
r_month := UDINT_TO_USINT(m);
r_day := UDINT_TO_USINT(dd);
SPLIT_TOD(IN := t, HOUR => r_hour, MINUTE => r_min, SECOND => r_sec, MILLISECOND => r_ms);
SPLIT_DT(IN := s, MONTH => r_dt_month, HOUR => r_dt_hour);
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
