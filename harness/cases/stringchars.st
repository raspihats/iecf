(* One character of a STRING, s[i] (Ed.3 6.4.2), from 1, a CHAR: read,
   and written - only a character the string has: a position below 1 or
   past its end leaves s as it is, flagged STRING_POSITION; c = '$00'
   cuts the string there, as '$00' ends any string. In an array of
   strings, a[1][2], as Ed.3's Example 1; a CHAR into a STRING.
   Worked by hand, a[1] '5678' at the start: qc '6' (16#36); a[1] then
   '5668', '5666', '6666'; a[0] '6'; t 'A'.
   Tick 0, n 2: s 'AxC'.
   Tick 1, n 4: past the end, s stays 'ABC', flag STRING_POSITION on
   line 34.
   Tick 2, n 0: below 1, s stays 'ABC', still flagged.
   Tick 3, n 3: the last character, s 'ABx'; the flag stays until the
   platform clears it (docs/c-output.md).
   reference-skip: s[i] is Ed.3's; the reference compiler (Ed.2) has
   none. *)
PROGRAM main
VAR
  n AT %IB0 : SINT;
  qs AT %QB0 : STRING[4];
  qc AT %QB5 : CHAR;
  qa AT %QB6 : STRING[4];
  qz AT %QB11 : STRING[4];
  qw AT %QB16 : STRING[4];
END_VAR
VAR
  s : STRING[4];
  t : STRING[4];
  a : ARRAY[0..1] OF STRING[4] := ['1234', '5678'];
END_VAR
t := 'ABC';
t[2] := '$00';
qw := t;
s := 'ABC';
s[n] := 'x';
qs := s;
qc := a[1][2];
a[1][3] := qc;
a[1][4] := '6';
a[1][1] := a[1][4];
a[0] := a[1][4];
qa := a[1];
qz := a[0];
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
