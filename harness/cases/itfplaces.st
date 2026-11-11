(* Interfaces wherever a variable can be (Ed.3 6.6.6.5.1: "This rule
   applies for all cases where variables may be used"): array elements
   and structure fields, a VAR_GLOBAL, initial values that are instances
   (a local's, a global's), a FUNCTION's result, output and input with an
   initial value, a block's output (taken into an element and read from
   outside), a method's result and output, a method through an
   interface's table giving an interface, ?= into an element, = of
   elements, an element's method called, NULL there faulting (OO.md D8).
   Worked by hand, k %IB0 and stop %IX1.0; ID_OF gives a ROOM's id, 0
   for NULL; l1 is 1, gl 2, dm 3:
   tick 0, k 0: arr [l1, dm, l1] (arr[0] := l1 after dm; arr[2] the
   block's output, h.r = l1); hs[1].r = g = gl; SWAP's b not given, so
   hs[2].r NULL and tmp l1; d ?= arr[1] dm; darr[1] ?= arr[0] NULL.
   qa..qi 1 3 1 2 0 1 3 0 1; tmp := darr[0].ME() dm: qj 3; dm.GIVE puts
   dm in hs[2].r: qk 3; ql LEVEL(0) 0; arr[0] = arr[1] FALSE, hs[1].r =
   rg TRUE, darr[1] = NULL TRUE: Q byte 0 16#06.
   Tick 1, k 1: arr [dm, l1, l1]; tmp dm; d ?= l1 NULL; darr[1] ?= dm.
   qa..qi 3 1 1 2 0 3 0 3 1, qj 3, qk 3, ql 10; byte 0 16#02.
   Tick 2, k 2: arr [dm, dm, l1]; d dm, darr[1] dm. qa..qi 3 3 1 2 0 3 3
   3 1, qj 3, qk 3, ql 20; byte 0 16#03.
   Tick 3, stop TRUE: as tick 2, then hs[2].r NULL, its DAYTIME() faults
   NULL, the call not made.
   reference-skip: INTERFACE and CLASS are Ed.3's; the reference
   compiler (Ed.2) has neither. *)
INTERFACE ROOM
  METHOD DAYTIME
  END_METHOD
  METHOD NAME : SINT
  END_METHOD
END_INTERFACE

INTERFACE DIMMED EXTENDS ROOM
  METHOD LEVEL : INT
    VAR_INPUT pct : INT; END_VAR
  END_METHOD
  METHOD ME : DIMMED
  END_METHOD
END_INTERFACE

CLASS LIGHTROOM IMPLEMENTS ROOM
  VAR PUBLIC id : SINT; LIGHT : BOOL; END_VAR
  METHOD PUBLIC DAYTIME
    LIGHT := NOT LIGHT;
  END_METHOD
  METHOD PUBLIC NAME : SINT
    NAME := id;
  END_METHOD
END_CLASS

CLASS DIMMER IMPLEMENTS DIMMED
  VAR PUBLIC id : SINT; lvl : INT; END_VAR
  METHOD PUBLIC DAYTIME
    lvl := 0;
  END_METHOD
  METHOD PUBLIC NAME : SINT
    NAME := id;
  END_METHOD
  METHOD PUBLIC LEVEL : INT
    VAR_INPUT pct : INT; END_VAR
    lvl := pct;
    LEVEL := lvl;
  END_METHOD
  METHOD PUBLIC ME : DIMMED
    ME := THIS;
  END_METHOD
  METHOD PUBLIC GIVE
    VAR_OUTPUT r : ROOM; END_VAR
    r := THIS;
  END_METHOD
END_CLASS

TYPE HOLDER : STRUCT
  r : ROOM;
  n : INT;
END_STRUCT;
END_TYPE

FUNCTION_BLOCK PICKER
  VAR_INPUT a : ROOM; END_VAR
  VAR_OUTPUT o : ROOM; END_VAR
  o := a;
END_FUNCTION_BLOCK

FUNCTION PICK : ROOM
  VAR_INPUT a : ROOM; END_VAR
  PICK := a;
END_FUNCTION

FUNCTION SWAP
  VAR_INPUT a : ROOM; b : ROOM := NULL; END_VAR
  VAR_OUTPUT x : ROOM; y : ROOM; END_VAR
  x := b;
  y := a;
END_FUNCTION

FUNCTION ID_OF : SINT
  VAR_INPUT r : ROOM; END_VAR
  IF r = NULL THEN
    ID_OF := 0;
  ELSE
    ID_OF := r.NAME();
  END_IF;
END_FUNCTION

CONFIGURATION plant
  VAR_GLOBAL
    gl : LIGHTROOM := (id := 2);
    g : ROOM := gl;
  END_VAR
  RESOURCE cpu ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION

PROGRAM main
  VAR_EXTERNAL
    g : ROOM;
    gl : LIGHTROOM;
  END_VAR
  VAR
    l1 : LIGHTROOM := (id := 1);
    dm : DIMMER := (id := 3);
    arr : ARRAY[0..2] OF ROOM;
    darr : ARRAY[0..1] OF DIMMED := [dm, NULL];
    h : HOLDER := (r := l1, n := 3);
    hs : ARRAY[1..2] OF HOLDER;
    pk : PICKER;
    i : INT;
    d : DIMMED;
    tmp : ROOM;
    rg : ROOM := gl;
  END_VAR
  VAR
    k AT %IB0 : USINT;
    stop AT %IX1.0 : BOOL;
    qx0 AT %QX0.0 : BOOL;
    qx1 AT %QX0.1 : BOOL;
    qx2 AT %QX0.2 : BOOL;
    qa AT %QB1 : SINT;
    qb AT %QB2 : SINT;
    qc AT %QB3 : SINT;
    qd AT %QB4 : SINT;
    qe AT %QB5 : SINT;
    qf AT %QB6 : SINT;
    qg AT %QB7 : SINT;
    qh AT %QB8 : SINT;
    qi AT %QB9 : SINT;
    qj AT %QB10 : SINT;
    qk AT %QB11 : SINT;
    ql AT %QW12 : INT;
  END_VAR
  i := k;
  arr[0] := dm;
  arr[1] := darr[0];
  arr[i] := l1;
  hs[1].r := g;
  pk(a := h.r, o => arr[2]);
  SWAP(a := arr[0], x => hs[2].r, y => tmp);
  d ?= arr[1];
  darr[1] ?= arr[0];
  qa := ID_OF(arr[0]);
  qb := ID_OF(arr[1]);
  qc := ID_OF(arr[2]);
  qd := ID_OF(hs[1].r);
  qe := ID_OF(hs[2].r);
  qf := ID_OF(tmp);
  qg := ID_OF(d);
  qh := ID_OF(darr[1]);
  qi := ID_OF(PICK(pk.o));
  tmp := darr[0].ME();
  qj := ID_OF(tmp);
  dm.GIVE(r => hs[2].r);
  qk := ID_OF(hs[2].r);
  ql := darr[0].LEVEL(pct := i * 10);
  qx0 := arr[0] = arr[1];
  qx1 := hs[1].r = rg;
  qx2 := darr[1] = NULL;
  IF stop THEN
    hs[2].r := NULL;
    hs[2].r.DAYTIME();
  END_IF;
END_PROGRAM
