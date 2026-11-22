(* CODESYS's and TwinCAT's PUBLIC and FINAL on an interface's method
   or property (codesys-oo), as TcOpen writes `METHOD PUBLIC FINAL
   StepIn` in ITcoSequencerModeController: PUBLIC is what a prototype
   is, FINAL has no effect. Worked by hand:
   %QW0 through I, d is a D, whose Advance overrides B's although I's is
        FINAL: 2; B's Speed 5: 7.
   reference-skip: not IEC 61131-3. *)
INTERFACE I
  METHOD PUBLIC FINAL Advance : INT
  END_METHOD
  PROPERTY PUBLIC Speed : INT
  GET
  END_GET
  END_PROPERTY
END_INTERFACE
FUNCTION_BLOCK B IMPLEMENTS I
  METHOD PUBLIC Advance : INT
    Advance := 1;
  END_METHOD
  PROPERTY PUBLIC Speed : INT
  GET
    Speed := 5;
  END_GET
  END_PROPERTY
END_FUNCTION_BLOCK
FUNCTION_BLOCK D EXTENDS B
  METHOD PUBLIC Advance : INT
    Advance := 2;
  END_METHOD
END_FUNCTION_BLOCK
PROGRAM main
  VAR d : D; i : I; END_VAR
  VAR n AT %QW0 : INT; END_VAR
  i := d;
  n := i.Advance() + i.Speed;
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
