(* More initial values worked out by the compiler, each compared with
   the same expression of variables computed as the program runs: + and
   - on dates (Table 30, a TOD wrapping at midnight), the long dates'
   functions, the text of a real, a duration and a date (the target's,
   written at start-up), a literal's text to its value, and reals that
   go through the target's mathematics to an integer away from where the
   conversion changes (docs/dependencies.md). ok counts those that
   agree, 24 of 24 (Q0 18). Worked by hand for three: x1, REAL_TO_INT
   of SQRT(16.0), is 4 (Q1); x5, 100 / x1, 25 (Q2 19); t3, TOD#23:00:00
   + T#2h, is 01:00, its hour 1 (Q3).
   reference-skip: the reference compiler takes a literal there. *)
PROGRAM main
  VAR
    d5 : DT := DT#2021-05-06-07:08:09 + T#1h;
    d6 : DT := DT#2021-05-06-07:08:09 - T#1h;
    t1 : TIME := DT#2021-05-06-07:08:09 - DT#2021-05-06-07:00:00;
    t2 : TIME := D#2021-05-07 - D#2021-05-06;
    t3 : TOD := TOD#23:00:00 + T#2h;
    t4 : TIME := TOD#10:00:00 - TOD#09:00:00;
    l1 : LDT := LDT#2021-05-06-07:08:09 + LT#1h;
    l2 : LTIME := LDT#2021-05-06-07:08:09 - LDT#2021-05-06-07:00:00;
    l3 : LTOD := CONCAT_LTOD(1, 2, 3, 4);
    l4 : LDT := CONCAT_LDT(2020, 1, 1, 12, 0, 0, 0);
    l5 : LDT := CONCAT_DATE_LTOD(D#2020-01-01, LTOD#12:00:00);
    l6 : LTOD := ADD_LTOD_LTIME(LTOD#23:00:00, LT#2h);
    s1 : STRING[30] := REAL_TO_STRING(1.5);
    s3 : STRING[40] := TIME_TO_STRING(T#1h30m);
    s4 : STRING[24] := DT_TO_STRING(DT#2021-05-06-07:08:09);
    r1 : REAL := STRING_TO_REAL('1.5');
    tm : TIME := STRING_TO_TIME('T#5s');
    dd : DATE := STRING_TO_DATE('D#2021-05-06');
    b1 : BOOL := STRING_TO_BOOL('TRUE');
    x1 : INT := REAL_TO_INT(SQRT(16.0));
    x3 : INT := REAL_TO_INT(SQRT(2.0) * 100.0);
    x4 : REAL := 10.0 / SQRT(2.0);
    x5 : INT := 100 / REAL_TO_INT(SQRT(16.0));
    x11 : DINT := LREAL_TO_DINT(1.0E5 * SQRT(LREAL#2.0));
  END_VAR
  VAR dx : DT := DT#2021-05-06-07:08:09; dy : DT := DT#2021-05-06-07:00:00; h : TIME := T#1h; h2 : TIME := T#2h;
      da : DATE := D#2021-05-07; db : DATE := D#2021-05-06; ta : TOD := TOD#10:00:00; tb : TOD := TOD#09:00:00;
      t23 : TOD := TOD#23:00:00; lx : LDT := LDT#2021-05-06-07:08:09; ly : LDT := LDT#2021-05-06-07:00:00;
      lh : LTIME := LT#1h; lh2 : LTIME := LT#2h; lt23 : LTOD := LTOD#23:00:00; lt12 : LTOD := LTOD#12:00:00;
      d20 : DATE := D#2020-01-01; one : INT := 1; two : INT := 2; three : INT := 3; four : INT := 4;
      y : INT := 2020; twelve : INT := 12; zero : INT; r15 : REAL := 1.5; tt : TIME := T#1h30m;
      sreal : STRING[8] := '1.5'; st : STRING[8] := 'T#5s'; sd : STRING[16] := 'D#2021-05-06'; sb : STRING[8] := 'TRUE';
      r16 : REAL := 16.0; r2 : REAL := 2.0; r100 : REAL := 100.0; l2r : LREAL := 2.0; l1e5 : LREAL := 1.0E5;
      ok : USINT; e1 : INT; hh, mm, ss, mss : INT;
  END_VAR
  VAR q AT %QB0 : USINT; q1 AT %QB1 : USINT; q2 AT %QB2 : USINT; q3 AT %QB3 : USINT; END_VAR
  ok := 0;
  IF d5 = dx + h THEN ok := ok + 1; END_IF;
  IF d6 = dx - h THEN ok := ok + 1; END_IF;
  IF t1 = dx - dy THEN ok := ok + 1; END_IF;
  IF t2 = da - db THEN ok := ok + 1; END_IF;
  IF t3 = t23 + h2 THEN ok := ok + 1; END_IF;
  IF t4 = ta - tb THEN ok := ok + 1; END_IF;
  IF l1 = lx + lh THEN ok := ok + 1; END_IF;
  IF l2 = lx - ly THEN ok := ok + 1; END_IF;
  IF l3 = CONCAT_LTOD(one, two, three, four) THEN ok := ok + 1; END_IF;
  IF l4 = CONCAT_LDT(y, one, one, twelve, zero, zero, zero) THEN ok := ok + 1; END_IF;
  IF l5 = CONCAT_DATE_LTOD(d20, lt12) THEN ok := ok + 1; END_IF;
  IF l6 = ADD_LTOD_LTIME(lt23, lh2) THEN ok := ok + 1; END_IF;
  IF s1 = REAL_TO_STRING(r15) THEN ok := ok + 1; END_IF;
  IF s3 = TIME_TO_STRING(tt) THEN ok := ok + 1; END_IF;
  IF s4 = DT_TO_STRING(dx) THEN ok := ok + 1; END_IF;
  IF r1 = STRING_TO_REAL(sreal) THEN ok := ok + 1; END_IF;
  IF tm = STRING_TO_TIME(st) THEN ok := ok + 1; END_IF;
  IF dd = STRING_TO_DATE(sd) THEN ok := ok + 1; END_IF;
  IF b1 = STRING_TO_BOOL(sb) THEN ok := ok + 1; END_IF;
  IF x1 = REAL_TO_INT(SQRT(r16)) THEN ok := ok + 1; END_IF;
  IF x3 = REAL_TO_INT(SQRT(r2) * r100) THEN ok := ok + 1; END_IF;
  IF x4 = 10.0 / SQRT(r2) THEN ok := ok + 1; END_IF;
  e1 := REAL_TO_INT(SQRT(r16));
  IF x5 = 100 / e1 THEN ok := ok + 1; END_IF;
  IF x11 = LREAL_TO_DINT(l1e5 * SQRT(l2r)) THEN ok := ok + 1; END_IF;
  q := ok;
  q1 := INT_TO_USINT(x1);
  q2 := INT_TO_USINT(x5);
  SPLIT_TOD(t3, hh, mm, ss, mss);
  q3 := INT_TO_USINT(hh);
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION
