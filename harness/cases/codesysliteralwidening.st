(* An untyped integer literal the other operand's type does not hold
   (literal-widening): the operation is computed in the narrowest type
   of the operand's kind that holds it, as CODESYS and TwinCAT compute
   intermediate results at least 32 bits wide. TcOpen's TcoBytesToReal
   writes `256 * _b0 + _b1` of BYTEs and `w * 65536` of a WORD, its
   TcoRealToBytes `2147483648 + ...` of DINTs.
   Worked by hand:
   256 * 16#12 + 16#34, in WORD: 16#1234 (4660).
   16#0003 * 65536 + 16#1234, in DWORD: 16#00031234 (201268).
   2147483648 + 100, in LINT: 2147483748 (16#80000064).
   5 * -1 of a UINT, in DINT: -5.
   30000 < 70000 of an INT, compared in DINT: TRUE.
   reference-skip: not IEC 61131-3. *)
PROGRAM main
  VAR
    b0 : BYTE := 16#12;
    b1 : BYTE := 16#34;
    w : WORD;
    w1 : WORD := 16#0003;
    k : INT := 30000;
    x : DINT := 100;
    u : UINT := 5;
  END_VAR
  VAR q0 AT %QD0 : DWORD; q4 AT %QD4 : DWORD; q8 AT %QL8 : LINT; q16 AT %QD16 : DINT; q20 AT %QX20.0 : BOOL; END_VAR
  w := 256 * b0 + b1;
  q0 := w;
  q4 := (w1 * 65536) + w;
  q8 := 2147483648 + x;
  q16 := u * -1;
  q20 := k < 70000;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
