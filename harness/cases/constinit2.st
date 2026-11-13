(* Initial values that are functions of constants (Table 34's string
   functions, the conversions of whole numbers to and from text, Tables
   30 and 36's date functions): worked out by the compiler. Each is
   compared with the same function of variables, computed as the program
   runs; ok counts those that agree, 19 of 19 (Q0 13). Worked by hand
   for three: LEN of s6, 'abZZef', is 6 (Q1); f1, FIND('abcabc', 'ca'),
   is 3 (Q2); i1, STRING_TO_INT('17'), is 17 (Q3 11).
   reference-skip: the reference compiler takes a literal there. *)
PROGRAM main
  VAR CONSTANT pre : STRING[4] := 'ab'; END_VAR
  VAR
    s1 : STRING[8] := CONCAT('ab', 'cd');
    s2 : STRING[8] := CONCAT(pre, '-x', 'y');
    l1 : INT := LEN('hello');
    s3 : STRING[8] := INT_TO_STRING(-42);
    s4 : STRING[8] := LEFT('abcdef', 3);
    s5 : STRING[8] := RIGHT('abcdef', 2);
    s6 : STRING[8] := REPLACE('abcdef', 'ZZ', 2, 3);
    s7 : STRING[8] := MID('abcdef', 2, 3);
    s9 : STRING[8] := INSERT('abcd', 'XY', 2);
    s10 : STRING[8] := DELETE('abcdef', 2, 3);
    ws : WSTRING[8] := CONCAT("ab", "cd");
    f1 : INT := FIND('abcabc', 'ca');
    i1 : INT := STRING_TO_INT('17');
    d1 : DATE := CONCAT_DATE(2020, 2, 29);
    dt1 : DT := CONCAT_DATE_TOD(D#2020-01-01, TOD#12:00:00);
    t3 : TIME := SUB_TOD_TOD(TOD#10:00:00, TOD#09:00:00);
    d3 : DT := ADD_DT_TIME(DT#2021-05-06-07:08:09, T#1h);
    tod1 : TOD := CONCAT_TOD(1, 2, 3, 4);
    dt2 : DT := CONCAT_DT(2020, 1, 1, 12, 0, 0, 0);
  END_VAR
  VAR a : STRING[8] := 'abcdef'; b : STRING[8] := 'ab'; c : STRING[8] := 'cd'; z : STRING[8] := 'ZZ'; n42 : INT := -42;
      two : INT := 2; three : INT := 3; y : INT := 2020; wa : WSTRING[8] := "ab"; wc : WSTRING[8] := "cd";
      dd : DATE := D#2020-01-01; tt : TOD := TOD#12:00:00; ta : TOD := TOD#10:00:00; tb : TOD := TOD#09:00:00;
      dx : DT := DT#2021-05-06-07:08:09; h : TIME := T#1h; ok : USINT; s17 : STRING[8] := '17'; abc2 : STRING[8] := 'abcabc';
      ca : STRING[8] := 'ca'; xy : STRING[8] := 'XY'; abcd : STRING[8] := 'abcd'; one : INT := 1; four : INT := 4; twelve : INT := 12; zero : INT;
  END_VAR
  VAR q AT %QB0 : USINT; q1 AT %QB1 : USINT; q2 AT %QB2 : USINT; q3 AT %QB3 : USINT; END_VAR
  ok := 0;
  IF s1 = CONCAT(b, c) THEN ok := ok + 1; END_IF;
  IF s2 = CONCAT(b, '-x', 'y') THEN ok := ok + 1; END_IF;
  IF l1 = LEN(CONCAT('hel', 'lo')) THEN ok := ok + 1; END_IF;
  IF s3 = INT_TO_STRING(n42) THEN ok := ok + 1; END_IF;
  IF s4 = LEFT(a, three) THEN ok := ok + 1; END_IF;
  IF s5 = RIGHT(a, two) THEN ok := ok + 1; END_IF;
  IF s6 = REPLACE(a, z, two, three) THEN ok := ok + 1; END_IF;
  IF s7 = MID(a, two, three) THEN ok := ok + 1; END_IF;
  IF s9 = INSERT(abcd, xy, two) THEN ok := ok + 1; END_IF;
  IF s10 = DELETE(a, two, three) THEN ok := ok + 1; END_IF;
  IF ws = CONCAT(wa, wc) THEN ok := ok + 1; END_IF;
  IF f1 = FIND(abc2, ca) THEN ok := ok + 1; END_IF;
  IF i1 = STRING_TO_INT(s17) THEN ok := ok + 1; END_IF;
  IF d1 = CONCAT_DATE(y, two, 29) THEN ok := ok + 1; END_IF;
  IF dt1 = CONCAT_DATE_TOD(dd, tt) THEN ok := ok + 1; END_IF;
  IF t3 = SUB_TOD_TOD(ta, tb) THEN ok := ok + 1; END_IF;
  IF d3 = ADD_DT_TIME(dx, h) THEN ok := ok + 1; END_IF;
  IF tod1 = CONCAT_TOD(one, two, three, four) THEN ok := ok + 1; END_IF;
  IF dt2 = CONCAT_DT(y, one, one, twelve, zero, zero, zero) THEN ok := ok + 1; END_IF;
  q := ok;
  q1 := INT_TO_USINT(LEN(s6));
  q2 := INT_TO_USINT(f1);
  q3 := INT_TO_USINT(i1);
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION
