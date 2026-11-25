(* REAL and LREAL chains tested once (docs/c-output.md §5, chains.rs),
   through a standard function, a conversion, an array element and a
   divisor that is itself a result: an infinity or NaN at each, one a
   tick, the flag cleared before each (`unflag`), flagged as each
   operator tested on its own flags it - by its code and line. The
   trace is the one iecf gave before chains took these in.
   tick 0 the set-up's overflow at line 26 (x * x), the rest finite.
   tick 1 SIN of an overflowing product: overflow (flag 2), line 30.
   tick 2 LN(0.0) plus one: -infinity, overflow, line 31.
   tick 3 SQRT(-1.0) times two: NaN (flag 9), line 32.
   tick 4 EXP of minus an infinite input: 0, nothing flagged.
   tick 5 LREAL_TO_REAL(1.0E300) plus one: overflow, line 34.
   tick 6 an infinite array element times 2 plus 1: overflow, line 35.
   tick 7 divided by an overflowing product: overflow, line 36; the
          quotient 0, %QD0 2000.
   tick 8 ABS and ATAN of a leaf, COS, TAN, ASIN(2.0): NaN, line 37.
   tick 9 DINT_TO_LREAL(i) times 1.0E308 times 10: overflow, line 38.
   tick 10 divided by a difference of 0: the fault, line 39; no tick
          runs after it.
   %QD0 a finite witness, %QB4 the tick.
   reference-skip: its own flags; the reference compiler has none. *)
PROGRAM main
  VAR k : USINT; x : LREAL := 1.0E200; y : LREAL; inf : LREAL; i : DINT := 7;
      r : REAL; a : ARRAY[1..3] OF LREAL; n : INT := 2; zero : LREAL; w : LREAL; END_VAR
  VAR q0 AT %QD0 : DINT; q1 AT %QB4 : USINT; END_VAR
  IF k = 0 THEN inf := x * x; END_IF;
  IF k = 0 THEN a[n] := x * x; END_IF;
  CASE k OF
    0: y := SIN(0.5) * COS(0.5) + SQRT(4.0) / (1.0 + 2.0 * 3.0); w := y;
    1: y := SIN(x * x) + 1.0;
    2: y := LN(zero) + 1.0;
    3: y := SQRT(-1.0) * 2.0;
    4: y := EXP(-inf) * 2.0 + 1.0; w := y;
    5: r := LREAL_TO_REAL(1.0E300) + 1.0;
    6: y := a[n] * 2.0 + 1.0;
    7: y := 1.0 / (x * x) + 2.0; w := y;
    8: y := ABS(-x) * 0.0 + ATAN(2.0) + COS(0.1) * TAN(0.2) + ASIN(2.0);
    9: y := DINT_TO_LREAL(i) * 1.0E308 * 10.0;
    10: y := 1.0 / (x - x) + 2.0;
  END_CASE;
  q0 := LREAL_TO_DINT(w * 1000.0);
  q1 := k;
  k := k + 1;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
