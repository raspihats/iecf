(* A PROGRAM's inputs and outputs of an INTERFACE, connected in the
   CONFIGURATION (Ed.3 6.8.2) as elementary ones are: an input set from a
   global instance, from a global interface, or NULL before each run; an
   output stored into a global interface after it, a DIMMED one into a
   ROOM global. ID_OF gives a ROOM's id, 0 for NULL; gl is 2, dm 3.
   Worked by hand, k %IX0.0:
   tick 0: a gl, qa 2; b is go, NULL until the first run stores o: qb
   0; c NULL, qc 0; gr NULL until od is stored after the run, qd 0;
   o := a, so go gl; od dm, so gr dm.
   Tick 1, k TRUE: qa 2; b gl, qb 2; qd 3; o := dm, go dm.
   Tick 2, k FALSE: qb 3 (dm); o := a, go gl again.
   Tick 3: qb 2.
   reference-skip: INTERFACE and CLASS are Ed.3's, and the reference
   compiler's program connections do not compile (PROPOSAL.md §1b). *)
INTERFACE ROOM
  METHOD NAME : SINT
  END_METHOD
END_INTERFACE

INTERFACE DIMMED EXTENDS ROOM
END_INTERFACE

CLASS LIGHTROOM IMPLEMENTS ROOM
  VAR PUBLIC id : SINT; END_VAR
  METHOD PUBLIC NAME : SINT
    NAME := id;
  END_METHOD
END_CLASS

CLASS DIMMER IMPLEMENTS DIMMED
  VAR PUBLIC id : SINT; END_VAR
  METHOD PUBLIC NAME : SINT
    NAME := id;
  END_METHOD
END_CLASS

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
    dm : DIMMER := (id := 3);
    go : ROOM;
    gr : ROOM;
  END_VAR
  RESOURCE cpu ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main(a := gl, b := go, c := NULL, o => go, od => gr);
  END_RESOURCE
END_CONFIGURATION

PROGRAM main
  VAR_INPUT
    a : ROOM;
    b : ROOM;
    c : ROOM;
  END_VAR
  VAR_OUTPUT
    o : ROOM;
    od : DIMMED;
  END_VAR
  VAR_EXTERNAL
    dm : DIMMER;
    gr : ROOM;
  END_VAR
  VAR
    k AT %IX0.0 : BOOL;
    qa AT %QB0 : SINT;
    qb AT %QB1 : SINT;
    qc AT %QB2 : SINT;
    qd AT %QB3 : SINT;
  END_VAR
  qa := ID_OF(a);
  qb := ID_OF(b);
  qc := ID_OF(c);
  qd := ID_OF(gr);
  IF k THEN
    o := dm;
  ELSE
    o := a;
  END_IF;
  od := dm;
END_PROGRAM
