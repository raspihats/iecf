(* Pointers (the pointers extension): a STRING whole through a pointer,
   arrays and structures copied whole through one, ADR of a variable at
   an address, and ADR() as an initial value. Worked by hand:
   %QB0: ps := ADR(s); ps^ := 'abc' writes the STRING through ps; then
   LEN(s), 3.
   %QB1: t := ps^ reads it back whole; LEN(t) + LEN(ps^), 3 + 3 = 6.
   %QB2: pa := ADR(src); dst := pa^ copies the array [5, 6, 7] whole;
   dst[2], 7.
   %QB3: pq^ := pr copies a structure whole into r2 (pq := ADR(r2)), then
   r2.b, 9.
   %QB4: pin points at %IB2, ADR of a variable at an address; the
   input bytes are 0, so pin^ reads 0; pq0 points at the output %QB5 and
   writes 16#2A through it (%QB5), read back as 42 into %QB4.
   %QB6: pinit's initial value ADR(src[1]) points at src[1], 6.
   reference-skip: pointers are CODESYS's, not IEC 61131-3. *)
TYPE R : STRUCT a : SINT; b : SINT; END_STRUCT; END_TYPE

PROGRAM main
  VAR
    s : STRING[20];
    t : STRING[20];
    ps : POINTER TO STRING[20];
    src : ARRAY[0..2] OF SINT := [5, 6, 7];
    dst : ARRAY[0..2] OF SINT;
    pa : POINTER TO ARRAY[0..2] OF SINT;
    pr : R := (a := 1, b := 9);
    r2 : R;
    pq : POINTER TO R;
    pinit : POINTER TO SINT := ADR(src[1]);
    pin : POINTER TO BYTE;
    pq0 : POINTER TO BYTE;
  END_VAR
  VAR
    q0 AT %QB0 : SINT; q1 AT %QB1 : SINT; q2 AT %QB2 : SINT; q3 AT %QB3 : SINT;
    q4 AT %QB4 : BYTE; q5 AT %QB5 : BYTE; q6 AT %QB6 : SINT;
    i2 AT %IB2 : BYTE;
  END_VAR
  ps := ADR(s);
  ps^ := 'abc';
  q0 := INT_TO_SINT(LEN(s));
  t := ps^;
  q1 := INT_TO_SINT(LEN(t) + LEN(ps^));
  pa := ADR(src);
  dst := pa^;
  q2 := dst[2];
  pq := ADR(r2);
  pq^ := pr;
  q3 := r2.b;
  pin := ADR(i2);
  pq0 := ADR(q5);
  pq0^ := pin^ + 16#2A;
  q4 := q5;
  q6 := pinit^;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
