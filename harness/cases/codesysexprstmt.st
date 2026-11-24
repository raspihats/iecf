(* An expression standing as a statement (expression-statements), as
   TcOpen's tests read a property for what its GET does,
   `_sutNotInitialized.IsTrue;`, `_sut_A.Invoke().Done;`: every call in
   it made, its value unused; one with no call does nothing.
   Worked by hand, each tick: s.IsTrue's GET counts 1; t.Invoke() 1 and
   Done's GET 1, through the interface Invoke gives; Bump(n := k) + 1
   adds 1 to k (from 10); s.flag and arr[i], i out of bounds, nothing.
   Tick 1: 1, 1, 1, 11; tick 2: 2, 2, 2, 12.
   IT's GET holds an empty VAR ... END_VAR, as TwinCAT writes one
   (properties).
   reference-skip: not IEC 61131-3. *)
INTERFACE IT
  PROPERTY Done : BOOL
    GET
    VAR
    END_VAR
    END_GET
  END_PROPERTY
END_INTERFACE

FUNCTION_BLOCK Sig
  VAR_OUTPUT gets : INT; flag : BOOL; END_VAR
  PROPERTY IsTrue : BOOL
    GET
      gets := gets + 1;
      IsTrue := flag;
    END_GET
  END_PROPERTY
END_FUNCTION_BLOCK

FUNCTION_BLOCK Job IMPLEMENTS IT
  VAR_OUTPUT invokes : INT; dones : INT; END_VAR
  METHOD Invoke : IT
    invokes := invokes + 1;
    Invoke := THIS^;
  END_METHOD
  PROPERTY Done : BOOL
    GET
      dones := dones + 1;
      Done := TRUE;
    END_GET
  END_PROPERTY
END_FUNCTION_BLOCK

FUNCTION Bump : INT
  VAR_IN_OUT n : INT; END_VAR
  n := n + 1;
  Bump := n;
END_FUNCTION

PROGRAM main
  VAR s : Sig; t : Job; k : INT := 10; arr : ARRAY[1..2] OF INT; i : INT := 5; END_VAR
  VAR q0 AT %QW0 : INT; q2 AT %QW2 : INT; q4 AT %QW4 : INT; q6 AT %QW6 : INT; END_VAR
  s.IsTrue;
  t.Invoke().Done;
  Bump(n := k) + 1;
  s.flag;
  arr[i];
  q0 := s.gets;
  q2 := t.invokes;
  q4 := t.dones;
  q6 := k;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
