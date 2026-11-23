(* CODESYS's FB_init (fb-init), as TcOpen gives it: `child : T(THIS^,
   eMode.Keep)`. Worked by hand, at the cold start:
   %QW0: Base's FB_init, then Derived's, each its own: order 1 * 10 + 2,
   12.
   %QW2: the parent given, THIS^, Root: its Depth, 7.
   %QW4: the mode given, eMode.Keep, 1.
   %QX6.0: bInitRetains at a cold start: TRUE.
   %QW8: Counter's FB_init, the two flags alone, called once: 1.
   %QW10: Wider, derived from Derived, its FB_init naming the two
   inputs it shares p and m and adding level: Base's, Derived's and
   its own, 123; %QW12 level, 42; %QW14 Base's mode, given by place
   to each, 1.
   reference-skip: not IEC 61131-3. *)
INTERFACE IObject
  METHOD Depth : INT
  END_METHOD
END_INTERFACE

TYPE eMode : (None, Keep); END_TYPE

FUNCTION_BLOCK Base
  VAR _parent : IObject; _mode : eMode; order : INT; cold : BOOL; END_VAR
  METHOD FB_init : BOOL
    VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; parent : IObject; mode : eMode; END_VAR
    _parent := parent;
    _mode := mode;
    order := order * 10 + 1;
    cold := bInitRetains;
  END_METHOD
  METHOD ParentDepth : INT
    ParentDepth := _parent.Depth();
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Derived EXTENDS Base
  METHOD FB_init : BOOL
    VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; parent : IObject; mode : eMode; END_VAR
    order := order * 10 + 2;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Counter
  VAR_OUTPUT n : INT; END_VAR
  METHOD FB_init : BOOL
    VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; END_VAR
    n := n + 1;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Wider EXTENDS Derived
  VAR extra : INT; END_VAR
  METHOD FB_init : BOOL
    VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; p : IObject; m : eMode; level : INT; END_VAR
    extra := level;
    order := order * 10 + 3;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Root IMPLEMENTS IObject
  VAR child : Derived(THIS^, eMode.Keep); END_VAR
  VAR wide : Wider(THIS^, eMode.Keep, 42); END_VAR
  METHOD Depth : INT
    Depth := 7;
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR r : Root; plain : Counter; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QX6.0 : BOOL; q4 AT %QW8 : INT;
      q5 AT %QW10 : INT; q6 AT %QW12 : INT; q7 AT %QW14 : INT; END_VAR
  q0 := r.child.order;
  q1 := r.child.ParentDepth();
  q2 := TO_INT(r.child._mode);
  q3 := r.child.cold;
  q4 := plain.n;
  q5 := r.wide.order;
  q6 := r.wide.extra;
  q7 := TO_INT(r.wide._mode);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
