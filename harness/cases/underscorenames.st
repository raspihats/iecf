(* Names starting with one underscore (Ed.3 6.1.2 allows it), in a
   namespace: iecf keys LIB._BASE as LIB___BASE and a method as its
   type's key, `__` and its name, so the namespace and the type are
   found at the `__` before the underscore. INTERNAL called from a type
   of the namespace, a method's input given the type's constant, an
   override called through THIS, SUPER, a REF_TO the base, and a method
   through an interface.
   Worked by hand, one tick: d._Run(): THIS._Bump() is _DERIVED's, amount
   := _k 5, SUPER._Bump(amount := 6): n 6; THIS._Close() n 12.
   b._Bump(): amount 5, b.n 5. r := REF(d); r^._Bump(): _DERIVED's: d.n 18.
   i := d; i._Get() 18.
   %QW0 18, %QW2 5, %QW4 18.
   reference-skip: NAMESPACE, METHOD, INTERFACE are Ed.3's; the
   reference compiler (Ed.2) has none. *)
NAMESPACE Lib
  INTERFACE _Itf
    METHOD _Get : INT
    END_METHOD
  END_INTERFACE

  FUNCTION_BLOCK _Base
    VAR CONSTANT _k : INT := 5; END_VAR
    VAR_OUTPUT n : INT; END_VAR
    METHOD PUBLIC _Bump
      VAR_INPUT amount : INT := _k; END_VAR
      n := n + amount;
    END_METHOD
    METHOD INTERNAL _Close
      n := n * 2;
    END_METHOD
  END_FUNCTION_BLOCK

  FUNCTION_BLOCK _Derived EXTENDS _Base IMPLEMENTS _Itf
    METHOD PUBLIC OVERRIDE _Bump
      VAR_INPUT amount : INT := _k; END_VAR
      SUPER._Bump(amount := amount + 1);
    END_METHOD
    METHOD PUBLIC _Get : INT
      _Get := n;
    END_METHOD
    METHOD PUBLIC _Run
      THIS._Bump();
      THIS._Close();
    END_METHOD
  END_FUNCTION_BLOCK
END_NAMESPACE

PROGRAM main
  VAR
    d : Lib._Derived; b : Lib._Base; r : REF_TO Lib._Base; i : Lib._Itf;
  END_VAR
  VAR
    q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT;
  END_VAR
  d._Run();
  b._Bump();
  r := REF(d);
  r^._Bump();
  i := d;
  q0 := i._Get();
  q1 := b.n;
  q2 := d.n;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
