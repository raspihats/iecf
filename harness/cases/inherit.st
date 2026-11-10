(* Inheritance and dynamic binding (Ed.3 6.6.5.5 to 6.6.5.8, 6.6.7.2.7
   to 6.6.7.3; OO.md step B): 6.6.5.6's CIRCLE, whose CF calls THIS.PI()
   and so in a CIRCLE2 runs CIRCLE2's PI; LIGHT2ROOM with SUPER.DAYTIME()
   (6.6.5.7.3), behind a LIGHTROOM in-out, and a block extending it;
   Figure 21's SUPER() chain, BASE, DERIVED_1, DERIVED_2; a DERIVED_1
   given to a BASE in-out and referenced by a REF_TO BASE, its body run
   (6.6.7.2.9 rule 4), its added input keeping its value (OO.md D7); an
   ABSTRACT SHAPE as a FUNCTION's in-out; a method called through a NULL
   reference (D8).
   Worked by hand, the inputs go %IX0.0 and night %IX0.1:
   tick 0, go FALSE, night FALSE: qa 300 (CIRCLE, PI 3.0), qb 314
   (CIRCLE2, PI 3.14), qr 30 (rr is c1); room1 day: LIGHT FALSE; room2
   day: LIGHT FALSE, LIGHT2 TRUE, Both FALSE; fr night: LIGHT TRUE,
   LIGHT2 FALSE (q byte 6 16#14); d2: x 3*(2+1)+0 9, c 9/5 1; qu1 6 (bb:
   x 5+1), qu2 19 (d1: SUPER() x 6, then 3*6+b, b 1); rb is bb, a 0:
   qref 1; qarea 1.
   Tick 1, go TRUE: qr 31 (rr is c2, 31.4); d2 x 10, c 10/1 10; rb is
   d1, a 1: x 3*2+1, qref 7; qarea 4.
   Tick 2, night TRUE: rooms night, fr day: byte 6 16#23; d2 x 11, c 1;
   qref 3*3+1 10; qarea 9.
   Tick 3, go FALSE: qr 30; d2 x 12, c 12; rb is bb, qref 4; qarea 16;
   qnull 1, then rn^.CF() faults NULL at line 194: the statement and the
   rest of the tick not run.
   reference-skip: EXTENDS, OVERRIDE and SUPER are Ed.3's; the
   reference compiler (Ed.2) has none. *)
CLASS CIRCLE
  METHOD PUBLIC PI : REAL
    PI := 3.0;
  END_METHOD
  METHOD PUBLIC CF : REAL
    VAR_INPUT d : REAL; END_VAR
    CF := THIS.PI() * d;
  END_METHOD
END_CLASS

CLASS CIRCLE2 EXTENDS CIRCLE
  METHOD PUBLIC OVERRIDE PI : REAL
    PI := 3.14;
  END_METHOD
END_CLASS

CLASS LIGHTROOM
  VAR PUBLIC LIGHT : BOOL; END_VAR
  METHOD PUBLIC DAYTIME
    LIGHT := FALSE;
  END_METHOD
  METHOD PUBLIC NIGHTTIME
    LIGHT := TRUE;
  END_METHOD
END_CLASS

CLASS LIGHT2ROOM EXTENDS LIGHTROOM
  VAR PUBLIC LIGHT2 : BOOL; END_VAR
  METHOD PUBLIC OVERRIDE DAYTIME
    SUPER.DAYTIME();
    LIGHT2 := TRUE;
  END_METHOD
  METHOD PUBLIC OVERRIDE NIGHTTIME
    SUPER.NIGHTTIME();
    LIGHT2 := FALSE;
  END_METHOD
  METHOD PUBLIC Both : BOOL
    Both := LIGHT AND LIGHT2;
  END_METHOD
END_CLASS

FUNCTION_BLOCK ROOM_CTRL
  VAR_INPUT night : BOOL; END_VAR
  VAR_IN_OUT RM : LIGHTROOM; END_VAR
  IF night THEN
    RM.NIGHTTIME();
  ELSE
    RM.DAYTIME();
  END_IF;
END_FUNCTION_BLOCK

FUNCTION_BLOCK FB_ROOM EXTENDS LIGHT2ROOM
  VAR_INPUT night : BOOL; END_VAR
  IF night THEN
    THIS.NIGHTTIME();
  ELSE
    THIS.DAYTIME();
  END_IF;
END_FUNCTION_BLOCK

FUNCTION_BLOCK BASE
  VAR_INPUT a : INT; END_VAR
  VAR_OUTPUT x : INT; END_VAR
  x := a + 1;
END_FUNCTION_BLOCK

FUNCTION_BLOCK DERIVED_1 EXTENDS BASE
  VAR_INPUT b : INT; END_VAR
  SUPER();
  x := 3 * x + b;
END_FUNCTION_BLOCK

FUNCTION_BLOCK DERIVED_2 EXTENDS DERIVED_1
  VAR_IN_OUT c : INT; END_VAR
  SUPER();
  c := x / c;
END_FUNCTION_BLOCK

FUNCTION_BLOCK USE
  VAR_IN_OUT io : BASE; END_VAR
  VAR_OUTPUT y : INT; END_VAR
  io(a := 5);
  y := io.x;
END_FUNCTION_BLOCK

CLASS ABSTRACT SHAPE
  METHOD PUBLIC ABSTRACT AREA : REAL
  END_METHOD
END_CLASS

CLASS SQUARE EXTENDS SHAPE
  VAR PUBLIC side : REAL; END_VAR
  METHOD PUBLIC OVERRIDE AREA : REAL
    AREA := side * side;
  END_METHOD
END_CLASS

FUNCTION total : INT
  VAR_IN_OUT s : SHAPE; END_VAR
  total := REAL_TO_INT(s.AREA());
END_FUNCTION

PROGRAM main
  VAR
    go AT %IX0.0 : BOOL;
    night AT %IX0.1 : BOOL;
    qa AT %QW0 : INT;
    qb AT %QW2 : INT;
    qr AT %QW4 : INT;
    ql1 AT %QX6.0 : BOOL;
    ql2 AT %QX6.1 : BOOL;
    ql3 AT %QX6.2 : BOOL;
    ql4 AT %QX6.3 : BOOL;
    ql5 AT %QX6.4 : BOOL;
    ql6 AT %QX6.5 : BOOL;
    qx AT %QW8 : INT;
    qc AT %QW10 : INT;
    qu1 AT %QW12 : INT;
    qu2 AT %QW14 : INT;
    qref AT %QW16 : INT;
    qarea AT %QW18 : INT;
    qnull AT %QB20 : BYTE;
  END_VAR
  VAR
    c1 : CIRCLE;
    c2 : CIRCLE2;
    rr, rn : REF_TO CIRCLE;
    room1 : LIGHTROOM;
    room2 : LIGHT2ROOM;
    ctrl : ROOM_CTRL;
    fr : FB_ROOM;
    d2 : DERIVED_2;
    cv : INT := 5;
    bb : BASE;
    d1 : DERIVED_1;
    u : USE;
    rb : REF_TO BASE;
    sq : SQUARE;
    n : INT;
  END_VAR
  qa := REAL_TO_INT(c1.CF(d := 100.0));
  qb := REAL_TO_INT(c2.CF(d := 100.0));
  IF go THEN rr := REF(c2); ELSE rr := REF(c1); END_IF;
  qr := REAL_TO_INT(rr^.CF(d := 10.0));
  ctrl(night := night, RM := room1);
  ctrl(night := night, RM := room2);
  fr(night := NOT night);
  ql1 := room1.LIGHT;
  ql2 := room2.LIGHT;
  ql3 := room2.LIGHT2;
  ql4 := room2.Both();
  ql5 := fr.LIGHT;
  ql6 := fr.LIGHT2;
  d2(a := 2, b := n, c := cv);
  qx := d2.x;
  qc := cv;
  d1.b := 1;
  u(io := bb);
  qu1 := u.y;
  u(io := d1);
  qu2 := u.y;
  IF go THEN rb := REF(d1); ELSE rb := REF(bb); END_IF;
  rb^(a := n);
  qref := rb^.x;
  sq.side := INT_TO_REAL(n + 1);
  qarea := total(s := sq);
  IF n = 3 THEN
    qnull := 1;
    qr := REAL_TO_INT(rn^.CF(d := 1.0));
  END_IF;
  n := n + 1;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
