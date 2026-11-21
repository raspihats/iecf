(* CODESYS's and TwinCAT's BIT (bit): a structure's BITs packed into
   bytes from bit 0 up, a member of another type at the next whole byte
   aligned to its size; a function block's BIT a BOOL. Worked by hand:
   S: a %X0.0, b %X0.1, c %X0.2, n %B2 (INT, aligned to 2), d %X4.0.
   a, c and d set, n := 5: its bytes, read through V's raw,
   %QB0..4 05 00 05 00 01.
   un.d := 16#00000105: byte 0 is 2#101, byte 1 is 1.
   %QB5 bits[0].b0 01, %QB6 bits[0].b2 01, %QB7 bits[1].b0 01,
   %QB8 bits[0].b1 00, %QB9 one (a BIT in a UNION, bit 0) 01,
   %QB10 f.y, NOT bits[0].b1, 01; %QB11 00.
   un.bits[3].b7 := TRUE sets the DWORD's top bit:
   %QD12 16#80000105, 05 01 00 80.
   reference-skip: not IEC 61131-3. *)
TYPE
  S : STRUCT
    a : BIT;
    b, c : BIT;
    n : INT;
    d : BIT;
  END_STRUCT;
  V : UNION
    s : S;
    raw : ARRAY[0..4] OF BYTE;
  END_UNION;
  B8 : STRUCT
    b0 : BIT; b1 : BIT; b2 : BIT; b3 : BIT;
    b4 : BIT; b5 : BIT; b6 : BIT; b7 : BIT;
  END_STRUCT;
  U : UNION
    d : DWORD;
    bits : ARRAY[0..3] OF B8;
    one : BIT;
  END_UNION;
END_TYPE

FUNCTION_BLOCK F
  VAR_INPUT x : BIT; END_VAR
  VAR_OUTPUT y : BOOL; END_VAR
  y := NOT x;
END_FUNCTION_BLOCK

PROGRAM main
  VAR v : V; un : U; f : F; END_VAR
  VAR
    q0 AT %QB0 : BYTE; q1 AT %QB1 : BYTE; q2 AT %QB2 : BYTE; q3 AT %QB3 : BYTE;
    q4 AT %QB4 : BYTE; q5 AT %QB5 : BYTE; q6 AT %QB6 : BYTE; q7 AT %QB7 : BYTE;
    q8 AT %QB8 : BYTE; q9 AT %QB9 : BYTE; q10 AT %QB10 : BYTE; qd AT %QD12 : DWORD;
  END_VAR
  v.s.a := TRUE;
  v.s.c := TRUE;
  v.s.n := 5;
  v.s.d := TRUE;
  q0 := v.raw[0]; q1 := v.raw[1]; q2 := v.raw[2]; q3 := v.raw[3]; q4 := v.raw[4];
  un.d := 16#00000105;
  q5 := BOOL_TO_BYTE(un.bits[0].b0);
  q6 := BOOL_TO_BYTE(un.bits[0].b2);
  q7 := BOOL_TO_BYTE(un.bits[1].b0);
  q8 := BOOL_TO_BYTE(un.bits[0].b1);
  q9 := BOOL_TO_BYTE(un.one);
  f(x := un.bits[0].b1);
  q10 := BOOL_TO_BYTE(f.y);
  un.bits[3].b7 := TRUE;
  qd := un.d;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
