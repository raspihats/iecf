(* WSTRING and WCHAR (Ed.3 6.4.2, Tables 6, 25, 27; docs/dependencies.md):
   a WSTRING's characters are UTF-16 units, a character above U+FFFF two
   (a surrogate pair, which LEN counts as two); literals in double quotes
   with $hhhh, ${...}, WCHAR#16#..., and an untyped "X" where a WCHAR is
   wanted (Table 6, 2b); an initial value, in an array and a structure, a
   function block's input and output; CONCAT, LEFT, INSERT, FIND, LEN,
   comparison, s[i] read and written; WORD_TO_WCHAR and WCHAR_TO_WORD
   (binary transfer, Table 25); WSTRING_TO_WCHAR (Table 27 row 2, though
   Figure 11 has none: decided, florin); WSTRING_TO_STRING and
   WCHAR_TO_CHAR, a character a STRING cannot hold skipped, flagged
   CONVERSION; INT_TO_WSTRING and WSTRING_TO_INT.
   Worked by hand, the input wc:
   tick 0, 'q' (16#0071): ws "a$263Aq", t "a$263Aqz": qlen 16#24 (LEN(t) 4,
   LEN(emoji) 2), qfind 4, qc 16#263A, qcmp TRUE, qeq TRUE, qn 75, qs '<q>',
   qch 'q', qw 16#0061, qk 4, qr 'q' (r.w "hqi"), qx 'q' (k.last[3]).
   Tick 1, 16#00E9 (e acute, in Latin-1): the same with 16#E9; qs '<'
   16#E9 '>'.
   Tick 2, 16#4E2D (a CJK character): qs '<>', qch '$00', qr and qx
   16#4E2D; flagged CONVERSION at line 75 (WSTRING_TO_STRING), the first
   flag, which stays until the platform clears it (docs/c-output.md §5).
   Tick 3, 16#0030 ('0'): WSTRING_TO_WCHAR of an empty WSTRING, "$0000"
   into qx (flagged too, behind the flag of tick 2: the book's checked
   table shows that flag, docs/behaviour/strings.md).
   reference-skip: WCHAR and s[i] are Ed.3's; the reference compiler
   (Ed.2) has neither. *)
TYPE rec : STRUCT w : WSTRING[4] := "hi"; c : WCHAR := "z"; END_STRUCT; END_TYPE

FUNCTION_BLOCK keep
VAR_INPUT s : WSTRING[8]; END_VAR
VAR_OUTPUT last : WSTRING[8] := "?"; n : INT; END_VAR
last := s;
n := LEN(s);
END_FUNCTION_BLOCK

PROGRAM main
VAR
  win AT %IW0 : WORD;
  qlen AT %QB0 : USINT;
  qfind AT %QB1 : USINT;
  qc AT %QW2 : WCHAR;
  qcmp AT %QX4.0 : BOOL;
  qeq AT %QX4.1 : BOOL;
  qn AT %QB5 : USINT;
  qs AT %QB6 : STRING[6];
  qch AT %QB13 : CHAR;
  qw AT %QW14 : WORD;
  qk AT %QB16 : USINT;
  qr AT %QW17 : WCHAR;
  qx AT %QW19 : WCHAR;
END_VAR
VAR
  ws : WSTRING[6] := "a$263Ab";
  wc : WCHAR;
  emoji : WSTRING := "${1F600}";
  arr : ARRAY[0..1] OF WSTRING[3] := ["xy", "z"];
  r : rec;
  k : keep;
  t : WSTRING[6];
  e : WSTRING[2] := "";
END_VAR
wc := WORD_TO_WCHAR(win);
ws[3] := wc;
t := CONCAT(ws, arr[1]);
qlen := INT_TO_USINT(LEN(t)) + INT_TO_USINT(LEN(emoji)) * 16;
qfind := INT_TO_USINT(FIND(t, "z"));
qc := t[2];
qcmp := t > "a";
qeq := LEFT(t, 1) = "a" AND r.c = "z";
qn := INT_TO_USINT(WSTRING_TO_INT(CONCAT(INT_TO_WSTRING(7), "5")));
k(s := t);
qk := INT_TO_USINT(k.n);
r.w := "hi";
r.w := INSERT(r.w, wc, 1);
qr := r.w[2];
qs := WSTRING_TO_STRING(CONCAT("<", wc, ">"));
qch := WCHAR_TO_CHAR(wc);
qw := WCHAR_TO_WORD(WSTRING_TO_WCHAR(t));
IF wc = "0" THEN
  qx := WSTRING_TO_WCHAR(e);
ELSE
  qx := k.last[3];
END_IF;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
