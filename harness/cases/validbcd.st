(* Ed.3's validate functions (6.6.2.5.15, Table 39), BCD conversions
   (6.6.2.5.2, Table 22 3a-4b, footnote d) and endianness (6.6.2.5.13,
   Table 37). Worked by hand, on a little-endian target (as the
   harness's and Prokop's are):
   MB0  IS_VALID: 1.0E38 TRUE, its tenfold (an infinity) FALSE, that minus
        itself (a NaN) FALSE, LREAL 2.5 TRUE: 2#1001, 16#09.
   MB1  IS_VALID_BCD: WORD 16#1234 TRUE, 16#12A4 FALSE, BYTE 16#99 TRUE,
        16#9A FALSE, LWORD sixteen 9s TRUE, DWORD 16#F000 FALSE: 16#15.
   MW2  WORD_BCD_TO_UINT(16#0369) = 369 (footnote d's own); MW4
        BCD_TO_INT(16#9999) = 9999; MB6 USINT_TO_BCD_BYTE(25) = 16#25
        (footnote d's); MW8 TO_BCD_WORD(UINT#1234) = 16#1234; MD10
        DWORD_BCD_TO_UDINT(16#12345678) = 12345678; ML14
        LWORD_BCD_TO_ULINT(sixteen 9s) = 9999999999999999.
   MD22 TO_BIG_ENDIAN(DWORD#16#12345678): its bytes 12 34 56 78; MW26
        TO_BIG_ENDIAN(INT#16#0102): 01 02; MB28 TO_BIG_ENDIAN(BYTE#16#12):
        not converted, 12; MD30 FROM_BIG_ENDIAN(TO_BIG_ENDIAN(16#CAFEBABE))
        is it again; MD34 TO_LITTLE_ENDIAN(16#01020304): as it is here;
        MD38 REAL 1.5 through TO_ and FROM_LITTLE_ENDIAN: 1.5; MD42 REAL
        1.5 TO_BIG_ENDIAN: its bytes 3f c0 00 00; MW46 WCHAR 'A' (16#0041)
        TO_BIG_ENDIAN, as a WORD: 00 41; MB48 TIME T#1ms TO_BIG_ENDIAN is
        16#01000000 ms, T#4h39m37s216ms: 1.
   MB50 Rec through TO_BIG_ENDIAN, packed into the image: a 11 22 33 44,
        f 01 as it is, c 01 02 03 04. MB60 Frame (relative locations)
        through TO_BIG_ENDIAN: h a1 b2, d 01 02 03 04. MB66: an
        enumerated value (bit 0) and a STRING (bit 1) as they are, Rec
        through TO_LITTLE_ENDIAN as it is here (bit 2): 2#111, 07.
   The infinity is flagged (IECF_FAULT_OVERFLOW, line 75), and stays the
   first flag; the BCD and endianness edge cases' flags are
   runtime/tests/arith.c's.
   reference-skip: the reference compiler (Ed.2) has neither IS_VALID,
   IS_VALID_BCD, BCD of a signed integer (BCD_TO_INT) nor the
   endianness functions; the BCD conversions it has are bcd.st's,
   checked against it. *)
TYPE
  Rec : STRUCT a : DWORD; f : BOOL; c : ARRAY[0..1] OF INT; END_STRUCT;
  Frame : STRUCT h AT %B0 : WORD; d AT %B2 : DINT; END_STRUCT;
  Mode : (lo, hi);
END_TYPE
PROGRAM main
VAR
  rin : Rec := (a := 16#11223344, f := TRUE, c := [16#0102, 16#0304]);
  rle : Rec;
  fin : Frame := (h := 16#A1B2, d := 16#01020304);
  m : Mode := hi;
  str : STRING := 'ab';
  big : REAL := 1.0E38;
  r1, r2, r3 : REAL;
  half : LREAL := 2.5;
  rx : REAL := 1.5;
  t : TIME;
END_VAR
VAR
  q_valid AT %MB0 : BYTE;
  q_bcd AT %MB1 : BYTE;
  u369 AT %MW2 : UINT;
  i9999 AT %MW4 : INT;
  b25 AT %MB6 : BYTE;
  w1234 AT %MW8 : WORD;
  d AT %MD10 : UDINT;
  l AT %ML14 : ULINT;
  e32 AT %MD22 : DWORD;
  e16 AT %MW26 : INT;
  e8 AT %MB28 : BYTE;
  back AT %MD30 : DWORD;
  lit AT %MD34 : DWORD;
  rr AT %MD38 : REAL;
  rb AT %MD42 : REAL;
  wc AT %MW46 : WORD;
  q_time AT %MB48 : BYTE;
  rout AT %MB50 : Rec;
  fout AT %MB60 : Frame;
  q_more AT %MB66 : BYTE;
END_VAR
r1 := big;
r2 := big * 10.0;
r3 := r2 - r2;
q_valid := 0;
IF IS_VALID(r1) THEN q_valid := q_valid OR 1; END_IF;
IF IS_VALID(r2) THEN q_valid := q_valid OR 2; END_IF;
IF IS_VALID(r3) THEN q_valid := q_valid OR 4; END_IF;
IF IS_VALID(half) THEN q_valid := q_valid OR 8; END_IF;
q_bcd := 0;
IF IS_VALID_BCD(WORD#16#1234) THEN q_bcd := q_bcd OR 1; END_IF;
IF IS_VALID_BCD(WORD#16#12A4) THEN q_bcd := q_bcd OR 2; END_IF;
IF IS_VALID_BCD(BYTE#16#99) THEN q_bcd := q_bcd OR 4; END_IF;
IF IS_VALID_BCD(BYTE#16#9A) THEN q_bcd := q_bcd OR 8; END_IF;
IF IS_VALID_BCD(LWORD#16#9999999999999999) THEN q_bcd := q_bcd OR 16; END_IF;
IF IS_VALID_BCD(DWORD#16#F000) THEN q_bcd := q_bcd OR 32; END_IF;
u369 := WORD_BCD_TO_UINT(WORD#16#0369);
i9999 := BCD_TO_INT(WORD#16#9999);
b25 := USINT_TO_BCD_BYTE(USINT#25);
w1234 := TO_BCD_WORD(UINT#1234);
d := DWORD_BCD_TO_UDINT(DWORD#16#12345678);
l := LWORD_BCD_TO_ULINT(LWORD#16#9999999999999999);
e32 := TO_BIG_ENDIAN(DWORD#16#12345678);
e16 := TO_BIG_ENDIAN(INT#16#0102);
e8 := TO_BIG_ENDIAN(BYTE#16#12);
back := FROM_BIG_ENDIAN(TO_BIG_ENDIAN(DWORD#16#CAFEBABE));
lit := TO_LITTLE_ENDIAN(DWORD#16#01020304);
rr := FROM_LITTLE_ENDIAN(TO_LITTLE_ENDIAN(rx));
rb := TO_BIG_ENDIAN(rx);
wc := WCHAR_TO_WORD(TO_BIG_ENDIAN(WCHAR#"A"));
t := TO_BIG_ENDIAN(T#1ms);
q_time := 0;
IF t = T#4h39m37s216ms THEN q_time := 1; END_IF;
rout := TO_BIG_ENDIAN(rin);
fout := TO_BIG_ENDIAN(fin);
rle := TO_LITTLE_ENDIAN(rin);
q_more := 0;
IF TO_BIG_ENDIAN(m) = hi THEN q_more := q_more OR 1; END_IF;
IF TO_BIG_ENDIAN(str) = 'ab' THEN q_more := q_more OR 2; END_IF;
IF rle.a = rin.a AND rle.c[1] = rin.c[1] THEN q_more := q_more OR 4; END_IF;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE R ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM P WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
