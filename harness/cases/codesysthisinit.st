(* CODESYS's and TwinCAT's THIS^ in an initial value (codesys-oo), as
   TcOpen writes `(Parent := THIS^)`: each instance's structure holds
   that instance, through the interface. Worked by hand:
   %QW0 a.Ask(): a's Id, k 1, plus n 5: 6.
   %QW2 b.Ask(): b's own Parent, not a's: 2 + 5, 7.
   %QW4 s.Ask(): s is a Special, whose Id overrides: 3 * 100 + 5, 305.
   reference-skip: not IEC 61131-3. *)
INTERFACE IObj
  METHOD Id : INT
  END_METHOD
END_INTERFACE

TYPE
  Holder : STRUCT
    Parent : IObj;
    n : INT;
  END_STRUCT;
END_TYPE

FUNCTION_BLOCK Comp IMPLEMENTS IObj
  VAR_INPUT k : INT; END_VAR
  VAR h : Holder := (Parent := THIS^, n := 5); END_VAR
  METHOD PUBLIC Id : INT
    Id := k;
  END_METHOD
  METHOD PUBLIC Ask : INT
    Ask := h.Parent.Id() + h.n;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Special EXTENDS Comp
  METHOD PUBLIC Id : INT
    Id := k * 100;
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR a : Comp; b : Comp; s : Special; END_VAR
  VAR qa AT %QW0 : INT; qb AT %QW2 : INT; qs AT %QW4 : INT; END_VAR
  a(k := 1);
  b(k := 2);
  s(k := 3);
  qa := a.Ask();
  qb := b.Ask();
  qs := s.Ask();
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
