(* Arithmetic on bit strings (the bit-string-arithmetic extension): a
   bit string counts as the unsigned integer of its width. Worked by
   hand, d 16#FFFF_FFF0, w 300, b 200, b2 100, i -5, x 513.7:
   %QD0  d + 16#20: DWORD, 16#1_0000_0010 wraps to 16#10, flagged.
   %QW4  w * 2 + b: WORD, 800 (16#0320); b widens to WORD.
   %QB6  b + b2: BYTE, 300 wraps to 44 (16#2C), flagged.
   %QW8  i + b: INT, -5 + 200 = 195; b counts as USINT, which INT holds.
   %QD12 BYTE_TO_REAL(b) + WORD_TO_REAL(w) / 3.0: 200.0 + 100.0 = 300.0
         (16#43960000), by value.
   %QB16 REAL_TO_BYTE(x / 10.0): 51.37 rounds to 51 (16#33).
   %QW18 w MOD 7 + b / 3: WORD 6 and BYTE 66 (division toward 0), as
         WORD 72 (16#48), into a UINT.
   %QB20 b - b2 * 2: 200 - 200 = 0.
   %QW22 a[b]: the array indexed by a BYTE, a[200] = 7.
   %QD24 DWORD_TO_REAL(16#3F800000) > 0.5: Ed.3's binary transfer is
         kept, 1.0 > 0.5, TRUE, 1.
   reference-skip: arithmetic on bit strings is CODESYS's, not IEC 61131-3. *)
PROGRAM main
  VAR
    d : DWORD := 16#FFFF_FFF0; w : WORD := 300; b : BYTE := 200; b2 : BYTE := 100;
    i : INT := -5; x : REAL := 513.7;
    a : ARRAY[0..255] OF INT;
  END_VAR
  VAR
    q0 AT %QD0 : DWORD; q1 AT %QW4 : WORD; q2 AT %QB6 : BYTE; q3 AT %QW8 : INT;
    q4 AT %QD12 : REAL; q5 AT %QB16 : BYTE; q6 AT %QW18 : UINT; q7 AT %QB20 : BYTE;
    q8 AT %QW22 : INT; q9 AT %QD24 : DWORD;
  END_VAR
  a[b] := 7;
  q0 := d + 16#20;
  q1 := w * 2 + b;
  q2 := b + b2;
  q3 := i + b;
  q4 := BYTE_TO_REAL(b) + WORD_TO_REAL(w) / 3.0;
  q5 := REAL_TO_BYTE(x / 10.0);
  q6 := w MOD 7 + b / 3;
  q7 := b - b2 * 2;
  q8 := a[b];
  q9 := DWORD_TO_REAL(16#3F800000) > 0.5;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
