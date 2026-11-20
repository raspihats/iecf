(* A structure extending another (struct-extends), as TcOpen's I/O types
   do: TYPE X EXTENDS Base : STRUCT, the base's fields first, at any
   depth, their initial values with them. Worked by hand:
   %QW0: t.a := 1, then a + b + c + d: 1 + 2 + 3 + 4 = 10.
   %QW2: a Top given to a Middle's field by field: Middle.c, 3.
   reference-skip: not IEC 61131-3. *)
TYPE
  Base : STRUCT a : INT; b : INT := 2; END_STRUCT;
  Middle EXTENDS Base : STRUCT c : INT := 3; END_STRUCT;
  Top EXTENDS Middle : STRUCT d : INT := 4; END_STRUCT;
END_TYPE

PROGRAM main
  VAR t : Top; m : Middle; END_VAR
  VAR q AT %QW0 : INT; r AT %QW2 : INT; END_VAR
  t.a := 1;
  q := t.a + t.b + t.c + t.d;
  m.c := t.c;
  r := m.c;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
