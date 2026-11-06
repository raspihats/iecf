(* Bit strings and BOOL: AND OR XOR NOT, the shifts and rotations, and
   BOOL expressions with short and full evaluation alike. *)
PROGRAM main
VAR
  x AT %IW0 : WORD;
  n AT %IB2 : BYTE;
  b0 AT %IX3.0 : BOOL;
  b1 AT %IX3.1 : BOOL;
  r_and AT %MW0 : WORD;
  r_or AT %MW2 : WORD;
  r_xor AT %MW4 : WORD;
  r_not AT %MW6 : WORD;
  r_shl AT %MW8 : WORD;
  r_shr AT %MW10 : WORD;
  r_rol AT %MW12 : WORD;
  r_ror AT %MW14 : WORD;
  r_dw AT %MD16 : DWORD;
  r_lw AT %ML20 : LWORD;
  r_by AT %MB28 : BYTE;
  q0 AT %QX0.0 : BOOL;
  q1 AT %QX0.1 : BOOL;
  q2 AT %QX0.2 : BOOL;
  q3 AT %QX0.3 : BOOL;
  q4 AT %QX0.4 : BOOL;
END_VAR
r_and := x AND 16#0FF0;
r_or := x OR 16#8001;
r_xor := x XOR 16#FFFF;
r_not := NOT x;
r_shl := SHL(x, BYTE_TO_INT(n));
r_shr := SHR(x, BYTE_TO_INT(n));
r_rol := ROL(x, BYTE_TO_INT(n));
r_ror := ROR(x, BYTE_TO_INT(n));
r_dw := ROL(WORD_TO_DWORD(x), 20);
r_lw := SHL(WORD_TO_LWORD(x), 48) OR 16#FF;
r_by := NOT n XOR 16#0F;
q0 := b0 AND b1;
q1 := b0 OR b1;
q2 := b0 XOR b1;
q3 := NOT b0 & b1;
q4 := (b0 OR NOT b1) AND x <> 0;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
