(* iecf's benchmark (docs/testing/performance.md): CODESYS's object
   orientation, iecf alone (--extension codesys): set `mode` to 0 for a
   method of an instance, 1 through an interface, 2 through a REFERENCE
   TO, 3 through a POINTER TO, 4 a property read through an interface. *)
INTERFACE ICounter
  METHOD Bump : DINT
    VAR_INPUT amount : DINT; END_VAR
  END_METHOD
  PROPERTY Value : DINT
  GET
  END_GET
  END_PROPERTY
END_INTERFACE
FUNCTION_BLOCK Base IMPLEMENTS ICounter
  VAR n : DINT; END_VAR
  METHOD PUBLIC Bump : DINT
    VAR_INPUT amount : DINT; END_VAR
    n := (n + amount) MOD 100003;
    Bump := n;
  END_METHOD
  PROPERTY PUBLIC Value : DINT
  GET
    Value := n;
  END_GET
  END_PROPERTY
END_FUNCTION_BLOCK
FUNCTION_BLOCK Derived EXTENDS Base
  METHOD PUBLIC Bump : DINT
    VAR_INPUT amount : DINT; END_VAR
    n := (n + amount * 2) MOD 100003;
    Bump := n;
  END_METHOD
END_FUNCTION_BLOCK
PROGRAM main
  VAR i : DINT; b : Base; d : Derived; c : ICounter; r : REFERENCE TO Base; p : POINTER TO Base;
      acc : DINT; mode : DINT; END_VAR
  VAR o1 AT %QD0 : DINT; END_VAR
  r REF= d;
  p := ADR(b);
  FOR i := 1 TO 20000 DO
    CASE mode OF
      0: acc := acc + b.Bump(amount := i MOD 7);
      1: c := d; acc := acc + c.Bump(amount := i MOD 7);
      2: acc := acc + r.Bump(amount := i MOD 7);
      3: acc := acc + p^.Bump(amount := i MOD 7);
      4: c := b; acc := acc + c.Value;
    END_CASE;
    acc := acc MOD 1000003;
  END_FOR;
  o1 := acc;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
