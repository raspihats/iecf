(* Methods and classes, bound statically (Ed.3 6.6.5, 6.6.7; OO.md step
   A): Ed.3's COUNTER (6.6.5.4.3 Example 2), UP5 calling THIS.UP; a
   method's result, its output taken, one without a result as a
   statement, one in an IF; a PUBLIC variable read and written from
   outside; a FUNCTION_BLOCK with a body and methods, the body calling
   THIS.Toggle, a method with in-outs; an input set from outside before
   the call; a method of an instance in an instance; EN on a method call.
   Worked amount hand, the input go at %IX0.0:
   tick 0, go FALSE: UP(3) CV 3 hits 1, q 3; UP5 CV 8 hits 2, q5 8, qf
   FALSE; not Full; qh 2, hits 12; L: lit FALSE (sw FALSE), then sw
   TRUE; Swap x 2 y 1, swaps.n 1, qx 21; Add(2) n 3, qs 3; EN FALSE: UP
   not called, qe stays 0.
   Tick 1, go TRUE: CV 11, 16, hits 13, 14; qh 14, hits 24; ql TRUE;
   qx 12, qs 6; UP(1) CV 17 hits 25, qe 17.
   Tick 2, go TRUE: UP(3) CV 20, q 20; UP5: 20 > 15, QU TRUE, q5 20, qf
   TRUE; Full: Reset, CV 0; hits 26, 27, qh 27, hits 37; qx 21, qs 9;
   UP(1) CV 1, qe 1.
   Tick 3, go FALSE: from CV 1, q 4, q5 9, qf FALSE, qh 40, ql FALSE, qx
   12, qs 12;
   qe stays 1.
   reference-skip: classes and methods are Ed.3's; the reference
   compiler (Ed.2) has neither. *)
CLASS COUNTER
  VAR CV : UINT; Top : UINT := 20; END_VAR
  VAR PUBLIC hits : INT; END_VAR
  METHOD PUBLIC UP : UINT
    VAR_INPUT INC : UINT; END_VAR
    VAR_OUTPUT QU : BOOL; END_VAR
    IF CV <= Top - INC THEN
      CV := CV + INC;
      QU := FALSE;
    ELSE
      QU := TRUE;
    END_IF;
    UP := CV;
    hits := hits + 1;
  END_METHOD
  METHOD PUBLIC UP5 : UINT
    VAR_OUTPUT QU : BOOL; END_VAR
    UP5 := THIS.UP(INC := 5, QU => QU);
  END_METHOD
  METHOD PUBLIC Full : BOOL
    Full := CV >= Top;
  END_METHOD
  METHOD PUBLIC Reset
    THIS.CV := 0;
  END_METHOD
END_CLASS

CLASS Tally
  VAR PUBLIC n : INT; END_VAR
  METHOD PUBLIC Add
    VAR_INPUT amount : INT := 1; END_VAR
    n := n + amount;
  END_METHOD
END_CLASS

FUNCTION_BLOCK Lamp
  VAR_INPUT sw : BOOL; END_VAR
  VAR_OUTPUT lit : BOOL; END_VAR
  VAR PUBLIC swaps : Tally; END_VAR
  METHOD PUBLIC Toggle
    sw := NOT sw;
  END_METHOD
  METHOD PUBLIC Swap
    VAR_IN_OUT a, b : INT; END_VAR
    VAR t : INT; END_VAR
    t := a;
    a := b;
    b := t;
    swaps.Add();
  END_METHOD
  lit := sw;
  THIS.Toggle();
END_FUNCTION_BLOCK

PROGRAM main
VAR
  CT : COUNTER;
  L : Lamp;
  lim : BOOL;
  x : INT := 1;
  y : INT := 2;
END_VAR
VAR
  go AT %IX0.0 : BOOL;
  q AT %QW0 : UINT;
  q5 AT %QW2 : UINT;
  qh AT %QW4 : INT;
  qx AT %QW6 : INT;
  qs AT %QW8 : INT;
  qf AT %QX10.0 : BOOL;
  ql AT %QX10.1 : BOOL;
  qe AT %QW11 : UINT;
END_VAR
q := CT.UP(INC := 3, QU => lim);
q5 := CT.UP5(QU => qf);
IF CT.Full() THEN
  CT.Reset();
END_IF;
qh := CT.hits;
CT.hits := CT.hits + 10;
L.sw := go;
L();
ql := L.lit;
L.Swap(a := x, b := y);
qx := x * 10 + y;
L.swaps.Add(amount := 2);
qs := L.swaps.n;
qe := CT.UP(EN := go, INC := 1);
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
