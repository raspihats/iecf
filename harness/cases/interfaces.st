(* Interfaces (Ed.3 6.6.6; OO.md step C): 6.6.6.5.3's ROOM and its
   ROOM_CTRL, which takes any ROOM as an input: a LIGHTROOM, a
   LIGHT2ROOM through THIS in a method of its base (its own overrides
   run), a FULL whose ABSTRACT base HALF implements DAYTIME only, a
   DIMMER of DIMMED, an interface extending ROOM; one ROOM holding a
   DIMMER or a LIGHT2ROOM, and ?= from it to a DIMMED and to a REF_TO
   DIMMER (6.6.6.7), NULL when it holds the other; ?= from a REF_TO
   LIGHTROOM to a REF_TO LIGHT2ROOM; = of interfaces; a FUNCTION taking a
   DIMMED, a ROOM of its own given it; a method with an output and ENO
   through its interface; a method called through a NULL interface
   (OO.md D8).
   Worked by hand, the inputs night %IX0.0, pick %IX0.1, stop %IX0.2,
   pct %IB1:
   tick 0, all FALSE, pct 0: room1 day: LIGHT FALSE; room2 through THIS
   night: LIGHT TRUE, LIGHT2 FALSE; full day: day TRUE (HALF's); dm day:
   lvl 0; r is room2, so d ?= r NULL (qnull), rd ?= r NULL; qof -1 (the
   FUNCTION's ROOM NULL); qx 0; qsame FALSE; rl is room1, rl2 ?= rl NULL;
   qcalls 4. Q byte 0: 16#1A.
   Tick 1, night TRUE, pick TRUE, pct 30: room1 night LIGHT TRUE; room2
   day: LIGHT FALSE, LIGHT2 TRUE; full night: day FALSE (FULL's); dm
   night lvl 100; r is dm: d and rd it; qof 30 (LEVEL 30); qx 80 (LEVEL
   80, not clipped); qsame TRUE; rl is room2, rl2 it; qlvl 80; qcalls 8.
   Q byte 0: 16#E5.
   Tick 2, pct 80: as tick 1, qof 80, then LEVEL 130 clipped: lvl 100,
   qx 100, qclip TRUE; qcalls 12.
   Tick 3, night FALSE, pick FALSE, stop TRUE: as tick 0, qcalls 16;
   then none.DAYTIME() faults NULL at line 200, the call not made.
   reference-skip: INTERFACE, IMPLEMENTS and ?= are Ed.3's; the
   reference compiler (Ed.2) has none. *)
INTERFACE ROOM
  METHOD DAYTIME
  END_METHOD
  METHOD NIGHTTIME
  END_METHOD
END_INTERFACE

INTERFACE DIMMED EXTENDS ROOM
  METHOD LEVEL : INT
    VAR_INPUT pct : INT; END_VAR
    VAR_OUTPUT clipped : BOOL; END_VAR
  END_METHOD
END_INTERFACE

FUNCTION_BLOCK ROOM_CTRL
  VAR_INPUT RM : ROOM; night : BOOL; END_VAR
  VAR_OUTPUT calls : INT; END_VAR
  IF RM = NULL THEN
    RETURN;
  END_IF;
  IF night THEN
    RM.NIGHTTIME();
  ELSE
    RM.DAYTIME();
  END_IF;
  calls := calls + 1;
END_FUNCTION_BLOCK

CLASS LIGHTROOM IMPLEMENTS ROOM
  VAR PUBLIC LIGHT : BOOL; END_VAR
  METHOD PUBLIC DAYTIME
    LIGHT := FALSE;
  END_METHOD
  METHOD PUBLIC NIGHTTIME
    LIGHT := TRUE;
  END_METHOD
  METHOD PUBLIC SELF_CTRL
    VAR_IN_OUT c : ROOM_CTRL; END_VAR
    VAR_INPUT n : BOOL; END_VAR
    c(RM := THIS, night := n);
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
END_CLASS

CLASS DIMMER IMPLEMENTS DIMMED
  VAR PUBLIC lvl : INT; END_VAR
  METHOD PUBLIC DAYTIME
    lvl := 0;
  END_METHOD
  METHOD PUBLIC NIGHTTIME
    lvl := 100;
  END_METHOD
  METHOD PUBLIC LEVEL : INT
    VAR_INPUT pct : INT; END_VAR
    VAR_OUTPUT clipped : BOOL; END_VAR
    clipped := pct > 100;
    ENO := NOT clipped;
    IF clipped THEN
      lvl := 100;
    ELSE
      lvl := pct;
    END_IF;
    LEVEL := lvl;
  END_METHOD
END_CLASS

FUNCTION_BLOCK ABSTRACT HALF IMPLEMENTS ROOM
  VAR_OUTPUT day : BOOL; END_VAR
  METHOD PUBLIC DAYTIME
    day := TRUE;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK FULL EXTENDS HALF
  METHOD PUBLIC NIGHTTIME
    day := FALSE;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION LEVEL_OF : INT
  VAR_INPUT d : DIMMED; pct : INT; END_VAR
  VAR r : ROOM; END_VAR
  r := d;
  IF r = NULL THEN
    LEVEL_OF := -1;
    RETURN;
  END_IF;
  LEVEL_OF := d.LEVEL(pct := pct);
END_FUNCTION

PROGRAM main
  VAR
    night AT %IX0.0 : BOOL;
    pick AT %IX0.1 : BOOL;
    stop AT %IX0.2 : BOOL;
    pct AT %IB1 : USINT;
    ql1 AT %QX0.0 : BOOL;
    ql2 AT %QX0.1 : BOOL;
    ql3 AT %QX0.2 : BOOL;
    qday AT %QX0.3 : BOOL;
    qnull AT %QX0.4 : BOOL;
    qrd AT %QX0.5 : BOOL;
    qsame AT %QX0.6 : BOOL;
    qdown AT %QX0.7 : BOOL;
    qlvl AT %QW2 : INT;
    qof AT %QW4 : INT;
    qx AT %QW6 : INT;
    qclip AT %QX8.0 : BOOL;
    qcalls AT %QW10 : INT;
  END_VAR
  VAR
    room1 : LIGHTROOM;
    room2 : LIGHT2ROOM;
    dm : DIMMER;
    full : FULL;
    ctrl : ROOM_CTRL;
    r : ROOM;
    d : DIMMED;
    none : ROOM;
    rd : REF_TO DIMMER;
    rl : REF_TO LIGHTROOM;
    rl2 : REF_TO LIGHT2ROOM;
    cl : BOOL;
  END_VAR
  ctrl(RM := room1, night := night);
  room2.SELF_CTRL(c := ctrl, n := NOT night);
  ctrl(RM := full, night := night);
  ctrl(RM := dm, night := night);
  IF pick THEN
    r := dm;
  ELSE
    r := room2;
  END_IF;
  d ?= r;
  qnull := d = NULL;
  rd ?= r;
  qrd := rd <> NULL;
  qof := LEVEL_OF(d := d, pct := pct);
  qx := 0;
  qclip := FALSE;
  IF d <> NULL THEN
    qx := d.LEVEL(pct := USINT_TO_INT(pct) + 50, clipped => cl);
    qclip := cl;
  END_IF;
  qsame := r = d;
  IF night THEN
    rl := REF(room2);
  ELSE
    rl := REF(room1);
  END_IF;
  rl2 ?= rl;
  qdown := rl2 <> NULL;
  ql1 := room1.LIGHT;
  ql2 := room2.LIGHT;
  ql3 := room2.LIGHT2;
  qday := full.day;
  qlvl := dm.lvl;
  qcalls := ctrl.calls;
  IF stop THEN
    none.DAYTIME();
  END_IF;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
