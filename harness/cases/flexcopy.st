(* A whole ARRAY[*] assigned (Ed.3 6.5.3; docs/dependencies.md): from
   another ARRAY[*] or from an array of fixed bounds, of as many
   elements in each dimension - which only the run knows, so it is
   checked as the assignment runs: a fault (index) and nothing copied
   when they differ. The bounds themselves need not match.
   COPY does dst := src. FILL3 does dst := three, its own [7, 8, 9].
   GRID does dst := src for two dimensions.
   Worked by hand, k %IB0 1, 2, 3:
   a: 1..3, [1, 2, 3]; b: -1..1, [0, 0, 0]. COPY(b, a) at tick 0: b
   [1, 2, 3]: b[-1] 1 (%QB0), b[1] 3 (%QB1).
   FILL3(a): a [7, 8, 9]: a[2] 8 (%QB2); then a[1] := k. So at the
   ticks after the first, COPY(b, a) gives b [k of the tick before, 8,
   9]: %QB0 1, 1, 2 and %QB1 3, 9, 9.
   m: 1..2, 0..1, [[1, 2], [3, 4]]; n: 0..1, 5..6. GRID(n, m): n[1, 6]
   4 (%QB3).
   Onto itself: COPY(a, a) leaves a as it is: a[3] 9 (%QB4).
   At tick 2, COPY(w, a) with w of 2 elements: 2 is not 3, a fault
   (index) in COPY at its assignment; w is as it was, w[1] 0 (%QB5),
   read before the call.
   reference-skip: ARRAY[*] is Ed.3's; the reference compiler (Ed.2)
   has none. *)
FUNCTION COPY
  VAR_IN_OUT dst, src : ARRAY [*] OF INT; END_VAR
  dst := src;
END_FUNCTION
FUNCTION FILL3
  VAR_IN_OUT dst : ARRAY [*] OF INT; END_VAR
  VAR three : ARRAY [1..3] OF INT := [7, 8, 9]; END_VAR
  dst := three;
END_FUNCTION
FUNCTION GRID
  VAR_IN_OUT dst, src : ARRAY [*, *] OF INT; END_VAR
  dst := src;
END_FUNCTION
CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR
    a : ARRAY [1..3] OF INT := [1, 2, 3];
    b : ARRAY [-1..1] OF INT;
    m : ARRAY [1..2, 0..1] OF INT := [1, 2, 3, 4];
    n : ARRAY [0..1, 5..6] OF INT;
    w : ARRAY [0..1] OF INT;
  END_VAR
  VAR k AT %IB0 : SINT; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
  END_VAR
  COPY(b, a);
  q0 := INT_TO_SINT(b[-1]);
  q1 := INT_TO_SINT(b[1]);
  FILL3(a);
  q2 := INT_TO_SINT(a[2]);
  a[1] := k;
  GRID(n, m);
  q3 := INT_TO_SINT(n[1, 6]);
  COPY(a, a);
  q4 := INT_TO_SINT(a[3]);
  q5 := INT_TO_SINT(w[1]);
  IF k = 3 THEN
    COPY(w, a);
  END_IF;
END_PROGRAM
