(* An ARRAY[*] as a FUNCTION's and a method's input and output (Ed.3
   6.5.3): the caller's array itself, not a copy, as Siemens passes an
   FC's structured parameters (docs/dependencies.md).
   SUM adds an ARRAY[*] input. TWICE passes its input on to SUM. FILL
   writes all but the last element of an ARRAY[*] output; the output's
   elements are set to their initial value, 0, as the call starts. INC
   reads one ARRAY[*] and writes another. PICK reads one element.
   Worked by hand, k %IB0 (1, 2, 3):
   x: 1..3, [1, 2, 3]: SUM(x) 6, %QB0 6.
   y: 0..4, [9, 9, 9, 9, 9]. %QB1 := SUM(a := y), before FILL: 45 at
   tick 0. FILL(v := k, a => y) then makes y [k, k, k, k, 0]: %QB2 :=
   y[4] is 0 (it was 9: the output started from its initial value),
   %QB3 := y[0] is k: 1, 2, 3. So %QB1 is 4 at tick 1 and 8 at tick 2.
   TWICE(x) 12, %QB4 12.
   INC(src := x, dst => z), z: 1..3: z [2, 3, 4], %QB5 := z[3] 4.
   KC, a CONSTANT array [7, 8]: SUM(KC) 15, %QB6 15.
   The method TOP gives the last element: top.TOP(x) 3, %QB7 3.
   In order, the output after the inputs: FILL(5, w), w: 1..3 [5, 5,
   0]: %QB8 := w[1] 5, %QB9 := w[3] 0.
   A row of an array of arrays, aa [[1, 2, 3], [4, 5, 6]]: SUM(aa[2])
   15, %QB10 15.
   %QB11 := PICK(x, k + 1): x[2] 2, x[3] 3; at tick 2, x[4] is outside
   1..3, a fault (index), the statement stopped and %QB11 kept 3.
   reference-skip: ARRAY[*], LOWER_BOUND and UPPER_BOUND are Ed.3's, and
   CLASS; the reference compiler (Ed.2) has none. *)
FUNCTION SUM : DINT
  VAR_INPUT a : ARRAY [*] OF INT; END_VAR
  VAR i : DINT; END_VAR
  FOR i := LOWER_BOUND(a, 1) TO UPPER_BOUND(a, 1) DO
    SUM := SUM + a[i];
  END_FOR;
END_FUNCTION
FUNCTION TWICE : DINT
  VAR_INPUT a : ARRAY [*] OF INT; END_VAR
  TWICE := 2 * SUM(a);
END_FUNCTION
FUNCTION FILL
  VAR_INPUT v : INT; END_VAR
  VAR_OUTPUT a : ARRAY [*] OF INT; END_VAR
  VAR i : DINT; END_VAR
  FOR i := LOWER_BOUND(a, 1) TO UPPER_BOUND(a, 1) - 1 DO
    a[i] := v;
  END_FOR;
END_FUNCTION
FUNCTION INC
  VAR_INPUT src : ARRAY [*] OF INT; END_VAR
  VAR_OUTPUT dst : ARRAY [*] OF INT; END_VAR
  VAR i : DINT; END_VAR
  FOR i := LOWER_BOUND(src, 1) TO UPPER_BOUND(src, 1) DO
    dst[i] := src[i] + 1;
  END_FOR;
END_FUNCTION
FUNCTION PICK : INT
  VAR_INPUT a : ARRAY [*] OF INT; i : INT; END_VAR
  PICK := a[i];
END_FUNCTION
CLASS LAST
  METHOD PUBLIC TOP : INT
    VAR_INPUT a : ARRAY [*] OF INT; END_VAR
    TOP := a[UPPER_BOUND(a, 1)];
  END_METHOD
END_CLASS
CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
PROGRAM main
  VAR CONSTANT KC : ARRAY [1..2] OF INT := [7, 8]; END_VAR
  VAR
    x : ARRAY [1..3] OF INT := [1, 2, 3];
    y : ARRAY [0..4] OF INT := [5(9)];
    z : ARRAY [1..3] OF INT;
    w : ARRAY [1..3] OF INT := [3(1)];
    aa : ARRAY [1..2] OF ARRAY [0..2] OF INT := [[1, 2, 3], [4, 5, 6]];
    top : LAST;
  END_VAR
  VAR k AT %IB0 : SINT; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT;
    q6 AT %QB6 : SINT;
    q7 AT %QB7 : SINT;
    q8 AT %QB8 : SINT;
    q9 AT %QB9 : SINT;
    q10 AT %QB10 : SINT;
    q11 AT %QB11 : SINT;
  END_VAR
  q0 := DINT_TO_SINT(SUM(x));
  q1 := DINT_TO_SINT(SUM(a := y));
  FILL(v := k, a => y);
  q2 := INT_TO_SINT(y[4]);
  q3 := INT_TO_SINT(y[0]);
  q4 := DINT_TO_SINT(TWICE(x));
  INC(src := x, dst => z);
  q5 := INT_TO_SINT(z[3]);
  q6 := DINT_TO_SINT(SUM(KC));
  q7 := INT_TO_SINT(top.TOP(x));
  FILL(5, w);
  q8 := INT_TO_SINT(w[1]);
  q9 := INT_TO_SINT(w[3]);
  q10 := DINT_TO_SINT(SUM(aa[2]));
  q11 := INT_TO_SINT(PICK(x, k + 1));
END_PROGRAM
