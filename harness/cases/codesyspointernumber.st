(* A pointer converted to a ULINT or LWORD (pointers, W0017): a number
   naming the place it points at, 0 for NULL - as TcOpen's TcoObject
   keeps `_Identity := ADR(THIS^)` and compares identities. The numbers
   are addresses, so only how they compare is traced:
   ADR(x) twice, once through p: equal (%QX0.0).
   ADR(x) and ADR(y): not equal (%QX0.1).
   NULL: 0 (%QX0.2).
   ADR(THIS^) in a's method, and ADR(a) outside: equal (%QX0.3).
   a's and b's: not equal (%QX0.4).
   As an LWORD, the same number as the ULINT (%QX0.5).
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Obj
  VAR id : ULINT; END_VAR
  METHOD Identity : ULINT
    id := ADR(THIS^);
    Identity := id;
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR
    x : INT; y : INT; p : POINTER TO INT; n : POINTER TO INT;
    i1 : ULINT; i2 : ULINT; i3 : ULINT; z : ULINT; w : LWORD;
    a : Obj; b : Obj;
  END_VAR
  VAR
    q0 AT %QX0.0 : BOOL; q1 AT %QX0.1 : BOOL; q2 AT %QX0.2 : BOOL;
    q3 AT %QX0.3 : BOOL; q4 AT %QX0.4 : BOOL; q5 AT %QX0.5 : BOOL;
  END_VAR
  p := ADR(x);
  i1 := ADR(x);
  i2 := p;
  i3 := ADR(y);
  z := n;
  w := p;
  q0 := i1 = i2;
  q1 := i1 <> i3;
  q2 := z = 0;
  i1 := ADR(a);
  q3 := a.Identity() = i1;
  q4 := a.Identity() <> b.Identity();
  q5 := LWORD_TO_ULINT(w) = i2;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
