(* Partial access of bit strings (Ed.3 6.6.1.3, Table 17): a bit, byte,
   word or double word of a BYTE, WORD, DWORD or LWORD, numbered from
   the least significant; read, and written into its whole. Worked by
   hand:
   w 16#1234: w.2 is TRUE (%QX0.0). w.%X0 := TRUE makes 16#1235;
   w.%B1 is 16#12 (%QB1). w.%B0 := 16#FF makes 16#12FF: %QB2 FF, %QB3
   12.
   dw 16#AABBCCDD: w2 := dw.%W1 is 16#AABB: %QB4 BB, %QB5 AA.
   dw.%W0 := 16#1122 makes 16#AABB1122: dw.%B0 22 (%QB6).
   l 0: l.63 := TRUE, l.%D0 := 16#01020304: l.%B7 80 (%QB7), l.%B2 02
   (%QB8).
   a[i].3 := TRUE with i 2: a[2] 8 (%QB9).
   m AT %MW0: m.%B1 := 16#AB writes its high byte, %MB1 (the M bytes
   00 AB); m.15 is TRUE (%QX10.0).
   r := REF(w); r^.%B1 := 16#77: w 16#77FF, w.%B1 77 (%QB11).
   o(q => w.0), q FALSE: w 16#77FE, w.%B0 FE (%QB12).
   Last, a[i].0 := TRUE with i 3: the index faults (INDEX) before the
   store.
   reference-skip: the reference compiler (Ed.2) has no partial
   access. *)
FUNCTION_BLOCK OT
  VAR_OUTPUT q : BOOL; END_VAR
  q := FALSE;
END_FUNCTION_BLOCK
PROGRAM main
  VAR
    w : WORD := 16#1234;
    w2 : WORD;
    dw : DWORD := 16#AABBCCDD;
    l : LWORD;
    a : ARRAY [1..2] OF WORD;
    i : INT := 2;
    r : REF_TO WORD;
    o : OT;
  END_VAR
  VAR
    m AT %MW0 : WORD;
    q00 AT %QX0.0 : BOOL;
    q1 AT %QB1 : BYTE;
    q2 AT %QB2 : BYTE;
    q3 AT %QB3 : BYTE;
    q4 AT %QB4 : BYTE;
    q5 AT %QB5 : BYTE;
    q6 AT %QB6 : BYTE;
    q7 AT %QB7 : BYTE;
    q8 AT %QB8 : BYTE;
    q9 AT %QB9 : BYTE;
    q10 AT %QX10.0 : BOOL;
    q11 AT %QB11 : BYTE;
    q12 AT %QB12 : BYTE;
  END_VAR
  q00 := w.2;
  w.%X0 := TRUE;
  q1 := w.%B1;
  w.%B0 := 16#FF;
  q2 := w.%B0;
  q3 := w.%B1;
  w2 := dw.%W1;
  q4 := w2.%B0;
  q5 := w2.%B1;
  dw.%W0 := 16#1122;
  q6 := dw.%B0;
  l.63 := TRUE;
  l.%D0 := 16#01020304;
  q7 := l.%B7;
  q8 := l.%B2;
  a[i].3 := TRUE;
  q9 := a[2].%B0;
  m.%B1 := 16#AB;
  q10 := m.15;
  r := REF(w);
  r^.%B1 := 16#77;
  q11 := w.%B1;
  o(q => w.0);
  q12 := w.%B0;
  i := 3;
  a[i].0 := TRUE;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
