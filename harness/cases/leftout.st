(* A standard function called by name with inputs left out (Ed.3
   6.6.1.4.2: "Any variable to which no value is assigned in the list
   shall have ... the default value for the associated data type"):
   each takes its type's default, 0, FALSE, '' or D#1970-01-01. An
   extensible function has as many inputs as the highest name given
   (6.6.2.5.1); those below it not given are left out. Worked by hand,
   the same at every tick, v 7, m -2, t TRUE, w 16#00F0:
   LIMIT(IN := v, MX := 5) 5, MN 0 (Table 19, NOTE 3) (%QB0);
   LIMIT(IN := m, MX := 5) 0 (%QB1); MUL(IN2 := v) 0 * 7 = 0 (%QB2);
   MAX(IN2 := m) 0 (%QB3); MIN(IN2 := v) 0 (%QB4); SUB(IN2 := v) -7
   (%QB5); MUX_INT(K := 3, IN0 := 1, IN2 := 2, IN4 := 3) 0, IN3 left out
   (6.6.2.5.1, EXAMPLE 2) (%QB6); SEL(G := t, IN0 := v) 0 (%QB7);
   ADD(IN3 := v) 0 + 0 + 7 = 7 (%QB8); the constant k, LIMIT(IN := 9,
   MX := 5), 5 (%QB9). AND(IN2 := t) FALSE (%QX10.0); OR(IN3 := t) TRUE
   (%QX10.1); XOR(IN2 := t) TRUE (%QX10.2); GT(IN3 := m) 0 > 0 > -2
   FALSE (%QX10.3); GT(IN2 := m) 0 > -2 TRUE (%QX10.4); EQ(IN2 := 0) TRUE
   (%QX10.5); SHL(IN := w) w (%QX10.6); LEN() 0 (%QX10.7).
   LEFT(IN := 'abc') '' (%QX11.0); DELETE(IN := 'abc', P := 2) 'abc',
   L 0 (%QX11.1); INSERT(IN1 := 'ab', P := 1) 'ab' (%QX11.2);
   CONCAT(IN2 := 'x') 'x' (%QX11.3); FIND(IN1 := 'abc') 0 (%QX11.4);
   EXPT(IN1 := 2.0) 1.0 (%QX11.5); CONCAT_TOD(HOUR := 1) TOD#01:00:00
   (%QX11.6); DINT_TO_INT() 0 (%QX11.7). SPLIT_DATE(YEAR => y) of
   D#1970-01-01: y 1970 (%QW12, 16#07B2).
   reference-skip: the reference compiler's C for an extensible function
   called by name without IN1, MUL(IN2 := v), does not compile (too few
   arguments); it has no LEN() or DINT_TO_INT() without an input, no
   MUX_INT, none of Table 36's. *)
PROGRAM main
  VAR
    v : INT := 7;
    m : INT := -2;
    t : BOOL := TRUE;
    w : WORD := 16#00F0;
    s : STRING;
    r : REAL;
    td : TOD;
    y : UINT;
  END_VAR
  VAR CONSTANT
    k : INT := LIMIT(IN := 9, MX := 5);
  END_VAR
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
    b0 AT %QX10.0 : BOOL;
    b1 AT %QX10.1 : BOOL;
    b2 AT %QX10.2 : BOOL;
    b3 AT %QX10.3 : BOOL;
    b4 AT %QX10.4 : BOOL;
    b5 AT %QX10.5 : BOOL;
    b6 AT %QX10.6 : BOOL;
    b7 AT %QX10.7 : BOOL;
    c0 AT %QX11.0 : BOOL;
    c1 AT %QX11.1 : BOOL;
    c2 AT %QX11.2 : BOOL;
    c3 AT %QX11.3 : BOOL;
    c4 AT %QX11.4 : BOOL;
    c5 AT %QX11.5 : BOOL;
    c6 AT %QX11.6 : BOOL;
    c7 AT %QX11.7 : BOOL;
    qy AT %QW12 : UINT;
  END_VAR
  q0 := INT_TO_SINT(LIMIT(IN := v, MX := 5));
  q1 := INT_TO_SINT(LIMIT(IN := m, MX := 5));
  q2 := INT_TO_SINT(MUL(IN2 := v));
  q3 := INT_TO_SINT(MAX(IN2 := m));
  q4 := INT_TO_SINT(MIN(IN2 := v));
  q5 := INT_TO_SINT(SUB(IN2 := v));
  q6 := INT_TO_SINT(MUX_INT(K := v - 4, IN0 := 1, IN2 := 2, IN4 := 3));
  q7 := INT_TO_SINT(SEL(G := t, IN0 := v));
  q8 := INT_TO_SINT(ADD(IN3 := v));
  q9 := INT_TO_SINT(k);
  b0 := AND(IN2 := t);
  b1 := OR(IN3 := t);
  b2 := XOR(IN2 := t);
  b3 := GT(IN3 := m);
  b4 := GT(IN2 := m);
  b5 := EQ(IN2 := 0);
  b6 := SHL(IN := w) = w;
  b7 := LEN() = 0;
  s := 'abc';
  c0 := LEFT(IN := s) = '';
  c1 := DELETE(IN := s, P := 2) = 'abc';
  c2 := INSERT(IN1 := 'ab', P := 1) = 'ab';
  c3 := CONCAT(IN2 := 'x') = 'x';
  c4 := FIND(IN1 := s) = 0;
  r := 2.0;
  c5 := EXPT(IN1 := r) = 1.0;
  td := CONCAT_TOD(HOUR := 1);
  c6 := td = TOD#01:00:00;
  c7 := DINT_TO_INT() = 0;
  SPLIT_DATE(YEAR => y);
  qy := y;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
