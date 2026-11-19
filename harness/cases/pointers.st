(* Pointers (the pointers extension), in the ways the OSCAT BASIC library
   uses them. Worked by hand:
   %QB0: StrLen walks a STRING's bytes through a POINTER TO BYTE to its
   0: 'hello', 5.
   %QB1: Sum takes ADR(a) and SIZEOF(a) of an ARRAY[0..3] OF REAL, as
   OSCAT's array functions do, and counts SIZEOF(a) / SIZEOF(pt^[0]) = 4
   elements: 1.5 + 2.5 + 3.0 + 4.0 = 11.0, as SINT 11.
   %QB2: Clear zeroes a BYTE buffer by pointer arithmetic, pt < stop,
   pt := pt + 1, from ADR(buf) to ADR(buf) + SIZEOF(buf); then buf[0] +
   buf[7], 0.
   %QB3: four bytes 16#11 16#22 16#33 16#44 read as one DWORD through a
   POINTER TO DWORD: 16#44332211 on a little-endian platform, its low
   byte 16#11.
   %QB4: a structure's field through a pointer, s.b := 7 by ps^.b := 7;
   and ps^.a + ps^.b with a 2: 9.
   %QB5: two pointers into one array subtracted: ADR(a[3]) - ADR(a[0]),
   3 * SIZEOF(REAL), 12.
   reference-skip: pointers are CODESYS's, not IEC 61131-3. *)
TYPE Pair : STRUCT a : SINT; b : SINT; END_STRUCT; END_TYPE

FUNCTION StrLen : INT
  VAR_INPUT pt : POINTER TO BYTE; END_VAR
  StrLen := 0;
  WHILE pt^ <> 0 DO
    StrLen := StrLen + 1;
    pt := pt + 1;
  END_WHILE;
END_FUNCTION

FUNCTION Sum : REAL
  VAR_INPUT pt : POINTER TO ARRAY[0..32000] OF REAL; size : UINT; END_VAR
  VAR i : INT; END_VAR
  Sum := 0.0;
  FOR i := 0 TO UINT_TO_INT(size / SIZEOF(pt^[0])) - 1 DO
    Sum := Sum + pt^[i];
  END_FOR;
END_FUNCTION

FUNCTION Clear : BOOL
  VAR_INPUT pt : POINTER TO BYTE; size : UINT; END_VAR
  VAR stop : POINTER TO BYTE; END_VAR
  stop := pt + size;
  WHILE pt < stop DO
    pt^ := 0;
    pt := pt + 1;
  END_WHILE;
  Clear := TRUE;
END_FUNCTION

PROGRAM main
  VAR
    s : STRING[20] := 'hello';
    a : ARRAY[0..3] OF REAL := [1.5, 2.5, 3.0, 4.0];
    buf : ARRAY[0..7] OF BYTE := [8(16#FF)];
    bytes : ARRAY[0..3] OF BYTE := [16#11, 16#22, 16#33, 16#44];
    pd : POINTER TO DWORD;
    pr : Pair := (a := 2);
    ps : POINTER TO Pair;
    ok : BOOL;
  END_VAR
  VAR
    q0 AT %QB0 : SINT; q1 AT %QB1 : SINT; q2 AT %QB2 : BYTE; q3 AT %QB3 : BYTE;
    q4 AT %QB4 : SINT; q5 AT %QB5 : SINT;
  END_VAR
  q0 := INT_TO_SINT(StrLen(ADR(s)));
  q1 := REAL_TO_SINT(Sum(ADR(a), SIZEOF(a)));
  ok := Clear(ADR(buf), SIZEOF(buf));
  q2 := buf[0] OR buf[7];
  pd := ADR(bytes);
  q3 := DWORD_TO_BYTE(pd^);
  ps := ADR(pr);
  ps^.b := 7;
  q4 := ps^.a + ps^.b;
  q5 := DINT_TO_SINT(ADR(a[3]) - ADR(a[0]));
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
