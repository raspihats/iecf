(* Variable-length arrays (Ed.3 6.5.3) and named constants in bounds
   (Ed.3 6.6.1.2.4). SUM is Ed.3's Example 2: it adds an ARRAY[*] whatever
   its bounds. FILL writes a 2-dimensional one, M[i, j] := 10 * i + j.
   TWICE passes its ARRAY[*] on to SUM. A CLASS's method takes one too.
   W, a VAR_GLOBAL CONSTANT, bounds a TYPE; N, the program's own
   CONSTANT, bounds an array and a STRING; a subrange is 0..N.
   Worked by hand, k %IB0:
   a: 1..5 (N), [1, 2, 3, 4, 5]: SUM 15, %QB0 15. b: -2..2, [10, 20, 30,
   40, 50]: SUM 150, %QB1 150. TWICE(a) 2 * 15 = 30, %QB2 30.
   m: 1..2, -1..1 filled: m[1, -1] 9, m[2, 1] 21: %QB3 9, %QB4 21.
   LOWER_BOUND(b, 1) -2, %QB5 16#fe; UPPER_BOUND(m, 2) 1, %QB6 1.
   row: ROW, 0..W (W 3), [1, 1, 1, 1]: the method's COUNT gives 4,
   %QB7 4. LEN(s), s : STRING[N] := 'abcde': %QB8 5.
   lim : INT(0..N) := N: %QB9 5.
   Each tick, %QB10 := UPPER_BOUND(m, k): k 1 at tick 0, 2; k 2 at
   tick 1, 1. At tick 2, k 3: m has 2 dimensions, a fault (index), the
   statement stopped and %QB10 kept 1.
   A FUNCTION_BLOCK's in-outs, set by each call before the statement
   that faults: sc doubles c's elements, an ARRAY[*] ([1, 2, 3] at 0..2):
   c[2] 6, 12, 24 at ticks 0, 1, 2 (%QB11); f[2] := f[1] + f[3], 5
   (%QB12); t := CONCAT(t, 'x') on st, 'ab': LEN 3, 4, 5 (%QB13); p.a
   + 1 on pr: 1, 2, 3 (%QB14).
   A CASE's labels are constant expressions too (Ed.3 7.3.3.3.3): N - 4
   is 1, 2..W - 1 is 2..2: k 1, 2, 3 give %QB15 10, 20 and the ELSE's 30.
   A row of an array of arrays is an array (Ed.3 Example 2's SUM(A2[2])
   on the form the vendors take, docs/dependencies.md): aa, [[1, 2, 3],
   [4, 5, 6]]: SUM(aa[2]) 15 (%QB16); SUM(aa[k MOD 2 + 1]), the row
   computed: rows 2, 1, 2, so 15, 6, 15 (%QB17).
   reference-skip: ARRAY[*], LOWER_BOUND and UPPER_BOUND are Ed.3's, and
   CLASS; the reference compiler (Ed.2) has none. *)
TYPE ROW : ARRAY [0..W] OF SINT; END_TYPE
TYPE PAIR : STRUCT a : SINT; b : SINT; END_STRUCT; END_TYPE

FUNCTION_BLOCK SCALE
  VAR_INPUT k : INT; END_VAR
  VAR_IN_OUT
    v : ARRAY [*] OF INT;
    f : ARRAY [1..3] OF INT;
    t : STRING[5];
    p : PAIR;
  END_VAR
  VAR i : DINT; END_VAR
  FOR i := LOWER_BOUND(v, 1) TO UPPER_BOUND(v, 1) DO
    v[i] := v[i] * k;
  END_FOR;
  f[2] := f[1] + f[3];
  t := CONCAT(t, 'x');
  p.a := p.a + 1;
END_FUNCTION_BLOCK

FUNCTION SUM : DINT
  VAR_IN_OUT A : ARRAY [*] OF INT; END_VAR
  VAR i, s : DINT; END_VAR
  s := 0;
  FOR i := LOWER_BOUND(A, 1) TO UPPER_BOUND(A, 1) DO
    s := s + A[i];
  END_FOR;
  SUM := s;
END_FUNCTION

FUNCTION TWICE : DINT
  VAR_IN_OUT A : ARRAY [*] OF INT; END_VAR
  TWICE := 2 * SUM(A);
END_FUNCTION

FUNCTION FILL
  VAR_IN_OUT M : ARRAY [*, *] OF INT; END_VAR
  VAR i, j : DINT; END_VAR
  FOR i := LOWER_BOUND(ARR := M, DIM := 1) TO UPPER_BOUND(ARR := M, DIM := 1) DO
    FOR j := LOWER_BOUND(M, 2) TO UPPER_BOUND(M, 2) DO
      M[i, j] := DINT_TO_INT(10 * i + j);
    END_FOR;
  END_FOR;
END_FUNCTION

CLASS COUNTER
  METHOD PUBLIC COUNT : SINT
    VAR_IN_OUT r : ARRAY [*] OF SINT; END_VAR
    VAR i : DINT; END_VAR
    COUNT := 0;
    FOR i := LOWER_BOUND(r, 1) TO UPPER_BOUND(r, 1) DO
      COUNT := COUNT + r[i];
    END_FOR;
  END_METHOD
END_CLASS

CONFIGURATION c
  VAR_GLOBAL CONSTANT W : INT := 3; END_VAR
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION

PROGRAM main
  VAR CONSTANT N : INT := 5; END_VAR
  VAR_EXTERNAL CONSTANT W : INT; END_VAR
  VAR
    a : ARRAY [1..N] OF INT := [1, 2, 3, 4, 5];
    b : ARRAY [-2..2] OF INT := [10, 20, 30, 40, 50];
    m : ARRAY [1..2, -1..1] OF INT;
    row : ROW := [4(1)];
    cnt : COUNTER;
    s : STRING[N] := 'abcde';
    lim : INT (0..N) := N;
    c : ARRAY [0..2] OF INT := [1, 2, 3];
    f : ARRAY [1..3] OF INT := [1, 0, 4];
    st : STRING[5] := 'ab';
    pr : PAIR;
    sc : SCALE;
    aa : ARRAY [1..2] OF ARRAY [0..2] OF INT := [[1, 2, 3], [4, 5, 6]];
  END_VAR
  VAR k AT %IB0 : SINT; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : USINT;
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
    q12 AT %QB12 : SINT;
    q13 AT %QB13 : SINT;
    q14 AT %QB14 : SINT;
    q15 AT %QB15 : SINT;
    q16 AT %QB16 : SINT;
    q17 AT %QB17 : SINT;
  END_VAR
  q0 := DINT_TO_SINT(SUM(a));
  q1 := DINT_TO_USINT(SUM(b));
  q2 := DINT_TO_SINT(TWICE(a));
  FILL(m);
  q3 := INT_TO_SINT(m[1, -1]);
  q4 := INT_TO_SINT(m[2, 1]);
  q5 := DINT_TO_SINT(LOWER_BOUND(b, 1));
  q6 := DINT_TO_SINT(UPPER_BOUND(m, 2));
  q7 := cnt.COUNT(row);
  q8 := INT_TO_SINT(LEN(s));
  q9 := INT_TO_SINT(lim);
  sc(k := 2, v := c, f := f, t := st, p := pr);
  q11 := INT_TO_SINT(c[2]);
  q12 := INT_TO_SINT(f[2]);
  q13 := INT_TO_SINT(LEN(st));
  q14 := pr.a;
  CASE k OF
    N - 4: q15 := 10;
    2..W - 1: q15 := 20;
  ELSE
    q15 := 30;
  END_CASE;
  q16 := DINT_TO_SINT(SUM(aa[2]));
  q17 := DINT_TO_SINT(SUM(aa[k MOD 2 + 1]));
  q10 := DINT_TO_SINT(UPPER_BOUND(m, k));
END_PROGRAM
