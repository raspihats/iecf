(* CODESYS's __QUERYINTERFACE (query-interface), as TcOpen asks its
   parent for a drive: `__QUERYINTERFACE(Parent, d)` is `d ?= Parent`,
   TRUE when d then holds the instance. Worked by hand:
   %QW0 the motor implements IDrive: TRUE, its Speed 7: 7.
   %QW2 the lamp does not: FALSE, -1; and d, which held the motor, is
        NULL after: + 100, 99.
   %QW4 no parent: FALSE, 0 stays.
   %QW6 a lamp's derived type implements IDrive, asked through IObj:
        its Speed 42.
   %QW8 in an expression, ANDed with what reads d after it: TRUE AND
        TRUE, 10; the lamp's FALSE, and d NULL after it, 1: 11.
   reference-skip: not IEC 61131-3. *)
INTERFACE IObj EXTENDS __SYSTEM.IQueryInterface
  METHOD Id : INT
  END_METHOD
END_INTERFACE

INTERFACE IDrive EXTENDS __System.IQueryInterface
  METHOD Speed : INT
  END_METHOD
END_INTERFACE

FUNCTION_BLOCK Motor IMPLEMENTS IObj, IDrive
  METHOD PUBLIC Id : INT
    Id := 1;
  END_METHOD
  METHOD PUBLIC Speed : INT
    Speed := 7;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Lamp IMPLEMENTS IObj
  METHOD PUBLIC Id : INT
    Id := 2;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK FastLamp EXTENDS Lamp IMPLEMENTS IDrive
  METHOD PUBLIC Speed : INT
    Speed := 42;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Child
  VAR_INPUT Parent : IObj; END_VAR
  VAR_OUTPUT q : INT; END_VAR
  VAR d : IDrive; END_VAR
  IF __QUERYINTERFACE(THIS^.Parent, d) THEN
    q := d.Speed();
  ELSE
    q := -1;
  END_IF
END_FUNCTION_BLOCK

PROGRAM main
  VAR m : Motor; l : Lamp; f : FastLamp; c : Child; o : IObj; d : IDrive; ok : BOOL; END_VAR
  VAR q0 AT %QW0 : INT; q2 AT %QW2 : INT; q4 AT %QW4 : INT; q6 AT %QW6 : INT; q8 AT %QW8 : INT; END_VAR
  c(Parent := m);
  q0 := c.q;
  o := l;
  d := m;
  IF __QUERYINTERFACE(o, d) THEN
    q2 := 1;
  ELSE
    q2 := -1;
  END_IF
  IF d = NULL THEN
    q2 := q2 + 100;
  END_IF
  o := NULL;
  IF __QUERYINTERFACE(o, d) THEN
    q4 := 1;
  END_IF
  c(Parent := f);
  q6 := c.q;
  o := m;
  ok := __QUERYINTERFACE(o, d) AND d.Speed() = 7;
  q8 := 10 * BOOL_TO_INT(ok);
  o := l;
  ok := NOT __QUERYINTERFACE(o, d) AND d = NULL;
  q8 := q8 + BOOL_TO_INT(ok);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
