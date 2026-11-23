(* __QUERYPOINTER (the query-pointer extension): the instance an
   interface refers to as a POINTER TO its block - a reference to it -
   its type checked, where CODESYS leaves it to the program. Worked by
   hand: i a Lamp, w 3: TRUE, p^.w 3 (%QB0 1, %QB1 3); a BigLamp, derived
   from Lamp, w 5: TRUE, 5 (%QB2 5); a Motor, another type: FALSE and p
   0 (%QB3 10); NULL: FALSE (%QB4 0); q := ADR(l), q^.w := 9 writes l's w
   (%QB5 9); then p, 0, read: IECF_FAULT_NULL on line 50, %QB6 left 1.
   reference-skip: __QUERYPOINTER is CODESYS's, not IEC 61131-3. *)
INTERFACE IShape
METHOD Area : INT
END_METHOD
END_INTERFACE

FUNCTION_BLOCK Lamp IMPLEMENTS IShape
VAR_INPUT w : INT; END_VAR
METHOD PUBLIC Area : INT
Area := w;
END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK BigLamp EXTENDS Lamp
END_FUNCTION_BLOCK

FUNCTION_BLOCK Motor IMPLEMENTS IShape
VAR_INPUT r : INT; END_VAR
METHOD PUBLIC Area : INT
Area := r;
END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
VAR l : Lamp := (w := 3); b : BigLamp := (w := 5); m : Motor := (r := 7); i : IShape;
    p, q : POINTER TO Lamp; ok : BOOL; END_VAR
VAR q0 AT %QB0 : SINT; q1 AT %QB1 : SINT; q2 AT %QB2 : SINT; q3 AT %QB3 : SINT; q4 AT %QB4 : SINT;
    q5 AT %QB5 : SINT; q6 AT %QB6 : SINT; END_VAR
i := l;
q0 := BOOL_TO_SINT(__QUERYPOINTER(i, p));
q1 := INT_TO_SINT(p^.w);
i := b;
IF __QUERYPOINTER(i, p) THEN q2 := INT_TO_SINT(p^.w); END_IF;
i := m;
ok := __QUERYPOINTER(i, p);
q3 := BOOL_TO_SINT(ok) + 10 * BOOL_TO_SINT(p = 0);
i := NULL;
q4 := BOOL_TO_SINT(__QUERYPOINTER(i, p));
q := ADR(l);
q^.w := 9;
q5 := INT_TO_SINT(l.w);
q6 := 1;
q6 := INT_TO_SINT(p^.w);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM pm WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
