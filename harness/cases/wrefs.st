(* REF_TO a WSTRING (Ed.3 6.4.4.10): one reference to a WSTRING at an
   address (odd byte 1, its units little-endian there) and, when pick, to
   one in C memory; what it reaches read, written whole and by s[i], and
   copied to and from; a REF_TO a structure at an address holding one.
   Worked by hand (inputs win at %IW0, pick at %IX2.0):
   tick 0, pick FALSE, win 'A' (16#41): r reaches qw "ab"; r^[1] := 'A':
   "Ab"; r^ := CONCAT(r^, '!'): "Ab!" (fits); k "Ab!"; mem := r^ "Ab!";
   qn LEN(r^) 3; rp^.w := LEFT(k, 2) "Ab", rp^.n 2; cp := rp^; qc
   cp.w[1] 'A'; qm := mem "Ab!".
   Q: 00 | qw 41 00 62 00 21 00 00 00 | qp.n 02 00 | qp.w 41 00 62 00 00
   00 | qn 03 | qc 41 00 | qm 41 00 62 00 21 00 00 00.
   Tick 1, pick TRUE, win 16#263A: r reaches mem "Ab!": "$263Ab!", then
   CONCAT "$263Ab!!" cut to "$263Ab!", flagged STRING_LENGTH at line 42,
   the first flag, which stays; mem := r^ copies it onto itself; qw
   unchanged; qp.w "$263Ab"; qc 16#263A; qm "$263Ab!".
   Tick 2, pick FALSE, win 16#E9: r reaches qw "Ab!" again: "$00E9b!";
   qp.w "$00E9b"; qc 16#E9; mem and qm "$00E9b!".
   reference-skip: WCHAR and s[i] are Ed.3's; the reference compiler
   (Ed.2) has neither. *)
TYPE pair : STRUCT n : INT; w : WSTRING[2]; END_STRUCT; END_TYPE

PROGRAM main
VAR
  win AT %IW0 : WCHAR;
  pick AT %IX2.0 : BOOL;
  qw AT %QW1 : WSTRING[3] := "ab";
  qp AT %QW9 : pair;
  qn AT %QB17 : USINT;
  qc AT %QW18 : WCHAR;
  qm AT %QW20 : WSTRING[3];
END_VAR
VAR
  mem : WSTRING[3] := "xy";
  cp : pair;
  r : REF_TO WSTRING[3];
  r2 : REF_TO WSTRING[3] := REF(mem);
  rp : REF_TO pair;
  k : WSTRING[3];
END_VAR
IF pick THEN r := REF(mem); ELSE r := REF(qw); END_IF;
r^[1] := win;
r^ := CONCAT(r^, "!");
k := r^;
r2^ := r^;
qn := INT_TO_USINT(LEN(r^));
rp := REF(qp);
rp^.w := LEFT(k, 2);
rp^.n := LEN(rp^.w);
cp := rp^;
qc := cp.w[1];
qm := mem;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
