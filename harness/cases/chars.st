(* CHAR (Ed.3 Table 10, a Latin-1 character in iecf, docs/dependencies.md):
   CHAR#'X', CHAR#16#41 and an untyped 'X' where a CHAR is wanted (Table
   6, 1b); at an address, in an array and a structure, an initial value,
   a function block's input and output, a function's input and result;
   compared, MAX, SEL; CHAR_TO_BYTE and BYTE_TO_CHAR (binary transfer,
   Table 25), CHAR_TO_USINT; to a STRING, implicitly and by
   CHAR_TO_STRING, and in CONCAT; FIND of a CHAR in a STRING (Table 34,
   NOTE 3); STRING_TO_CHAR, the first character - an empty string gives
   '$00', flagged CONVERSION.
   Worked by hand: tick 0, bin 16#62 ('b'): qc 'b' (16#62), qn 16#63,
   qa 'A' (16#41), qm 'b', qlt TRUE, qeq TRUE, qs 'xb', qf 'x', qu 98,
   qp 21 (FIND(s, c) 2, FIND(s, CHAR#'x') 1).
   Tick 1, bin 16#30 ('0'): qc '0', qn '1', qm 'b' (MAX of '0' and 'b'),
   qlt TRUE, qeq FALSE, qs 'x0', qf 'x', qu 48, qp 21; then the empty string,
   flag CONVERSION.
   reference-skip: CHAR is Ed.3's; the reference compiler (Ed.2) has none. *)
TYPE rec : STRUCT c : CHAR := 'z'; n : USINT; END_STRUCT; END_TYPE

FUNCTION next_char : CHAR
VAR_INPUT c : CHAR; END_VAR
next_char := BYTE_TO_CHAR(USINT_TO_BYTE(CHAR_TO_USINT(c) + 1));
END_FUNCTION

FUNCTION_BLOCK keep
VAR_INPUT c : CHAR; END_VAR
VAR_OUTPUT last : CHAR := CHAR#'?'; END_VAR
last := c;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  bin AT %IB0 : BYTE;
  qc AT %QB0 : CHAR;
  qn AT %QB1 : CHAR;
  qa AT %QB2 : CHAR;
  qm AT %QB3 : CHAR;
  qlt AT %QX4.0 : BOOL;
  qeq AT %QX4.1 : BOOL;
  qu AT %QB5 : USINT;
  qs AT %QB6 : STRING[3];
  qf AT %QB10 : CHAR;
  qp AT %QB11 : SINT;
END_VAR
VAR
  c : CHAR;
  arr : ARRAY[0..1] OF CHAR := [CHAR#'b', 'a'];
  r : rec;
  k : keep;
  s : STRING[3];
  e : STRING[3] := '';
END_VAR
c := BYTE_TO_CHAR(bin);
qc := c;
qn := next_char(c);
qa := CHAR#16#41;
qm := MAX(c, arr[0]);
qlt := c < r.c;
qeq := c = 'b';
k(c := c);
s := CONCAT('x', k.last);
qs := s;
qf := STRING_TO_CHAR(s);
qp := INT_TO_SINT(FIND(s, c) * 10 + FIND(IN1 := s, IN2 := CHAR#'x'));
qu := CHAR_TO_USINT(SEL(TRUE, arr[1], c));
qu := CHAR_TO_USINT(c);
IF c = '0' THEN
  qf := STRING_TO_CHAR(e);
END_IF;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
