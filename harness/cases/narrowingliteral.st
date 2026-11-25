(* implicit-narrowing: an integer literal outside the integer or bit
   string type it goes into keeps its low bits, as a conversion of a
   typed one would, W0010 at each - as TcOpen's `_testId := -1` of a
   ULINT and `_theOrderOfThePreviouslyExecutedStep := -1` of a UINT.
   Worked by hand:
   u := -1, a UINT: 65535, 16#FFFF (%QW0).
   g(300), to a USINT input: 300 - 256, 44, 16#2C (%QB2).
   s := 200, a SINT: 200 - 256, -56, 16#C8 (%QB3).
   w := 16#12345, a WORD: 16#2345 (%QW4).
   l := -1, a ULINT: 2^64 - 1, all ones (%QL8).
   i, initialised to -2, a UDINT: 2^32 - 2, 16#FFFFFFFE (%QD16).
   reference-skip: implicit narrowing is CODESYS's, not IEC 61131-3. *)
FUNCTION g : USINT
  VAR_INPUT x : USINT; END_VAR
  g := x;
END_FUNCTION

PROGRAM main
  VAR
    u : UINT; s : SINT; w : WORD; l : ULINT; i : UDINT := -2;
  END_VAR
  VAR
    q0 AT %QW0 : UINT; q1 AT %QB2 : USINT; q2 AT %QB3 : SINT; q3 AT %QW4 : WORD;
    q4 AT %QL8 : ULINT; q5 AT %QD16 : UDINT;
  END_VAR
  u := -1;
  q0 := u;
  q1 := g(300);
  s := 200;
  q2 := s;
  w := 16#12345;
  q3 := w;
  l := -1;
  q4 := l;
  q5 := i;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
