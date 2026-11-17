(* BCD conversions (Ed.3 6.6.2.5.2, Table 22 3a-4b, footnote d), by the
   names the reference compiler has too. Each nibble a decimal digit, the
   lowest the units. Inputs w (%IW0) and u (%IW2), so nothing is folded.
   Worked by hand:
   tick 0, w 16#0099, u 1234: a := WORD_BCD_TO_UINT(w) 99 (63 00);
     b := UINT_TO_BCD_WORD(u) 16#1234 (34 12); c, of w's low byte, 99
     (63); d := USINT_TO_BCD_BYTE(34) 16#34; e, of 16#12340099,
     12340099 (83 4b bc 00); f := ULINT_TO_BCD_LWORD(u * 10^11)
     16#0123400000000000; g := BCD_TO_UINT(w), the overloaded form, 99;
     h := WORD_BCD_TO_USINT(w) 99; k, f back, 123400000000000.
   tick 1, w 16#12A4, u 10000: w is not BCD (a nibble A), so a, c, e, g
     and h are 0, as the reference compiler gives (its ENO FALSE); iecf
     also flags it, "flag 4 MAIN 37" at a, the first. u has five digits,
     more than a WORD's four: b is every nibble F, 16#FFFF (ff ff),
     where the reference compiler keeps the low digits, 16#0000: iecf's
     choice, so its bytes are bcd.iecf.trace's (docs/dependencies.md). f is
     16#1000000000000000, k 10^15.
   tick 2, w 16#9999, u 99: a 9999 (0f 27), b 16#0099, c 99, d 16#99,
     e 12349999, f 16#0009900000000000, g 9999; h 9999 is beyond USINT,
     which keeps its low bits as any conversion does: 15 (0f). *)
PROGRAM main
VAR
  w AT %IW0 : WORD;
  u AT %IW2 : UINT;
END_VAR
VAR
  a AT %MW0 : UINT;
  b AT %MW2 : WORD;
  c AT %MB4 : USINT;
  d AT %MB5 : BYTE;
  e AT %MD6 : UDINT;
  f AT %ML10 : LWORD;
  g AT %MW18 : UINT;
  h AT %MB20 : USINT;
  k AT %ML24 : ULINT;
END_VAR
a := WORD_BCD_TO_UINT(w);
b := UINT_TO_BCD_WORD(u);
c := BYTE_BCD_TO_USINT(WORD_TO_BYTE(w));
d := USINT_TO_BCD_BYTE(UINT_TO_USINT(u MOD 100));
e := DWORD_BCD_TO_UDINT(WORD_TO_DWORD(w) OR 16#12340000);
f := ULINT_TO_BCD_LWORD(UINT_TO_ULINT(u) * 100000000000);
g := BCD_TO_UINT(w);
h := WORD_BCD_TO_USINT(w);
k := LWORD_BCD_TO_ULINT(f);
END_PROGRAM

CONFIGURATION Plant
  RESOURCE R ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM P WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
