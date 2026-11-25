(* CODESYS's and TwinCAT's IMPLEMENTS naming an interface twice
   (codesys-oo), as TcOpen's TcoInputDialog writes `IMPLEMENTS
   ITcoInputDialogAnswer, ITcoInputDialogAnswer`: it counts once,
   W0019. Worked by hand:
   %QW0 through I, d is a D, whose Answer overrides B's: 2, then 4.
   reference-skip: not IEC 61131-3. *)
INTERFACE I
  METHOD Answer : INT
  END_METHOD
END_INTERFACE
FUNCTION_BLOCK B IMPLEMENTS I, I
  VAR n : INT; END_VAR
  METHOD PUBLIC Answer : INT
    n := n + 1;
    Answer := n;
  END_METHOD
END_FUNCTION_BLOCK
FUNCTION_BLOCK D EXTENDS B
  METHOD PUBLIC Answer : INT
    n := n + 2;
    Answer := n;
  END_METHOD
END_FUNCTION_BLOCK
PROGRAM main
  VAR d : D; i : I; END_VAR
  VAR q AT %QW0 : INT; END_VAR
  i := d;
  q := i.Answer();
END_PROGRAM
CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
