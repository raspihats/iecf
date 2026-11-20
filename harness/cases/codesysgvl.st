(* Global variable lists (--gvl, --qualified-gvl), as CODESYS and TwinCAT
   name them: IO (codesysgvl.d/io.st) and CFG, qualified-only
   (codesysgvl.d/cfg.st). Worked by hand:
   cycle 0: %QW0 IO.a + CFG.k = 3 + 7 = 10; r[CFG.N] := IO.a, %QW2 3;
     IO.a 4, read bare: %QW4 4; CFG.t[4] := 4 + 1, %QW6 5.
   cycle 1: 4 + 7 = 11; 4; 5; 6.
   reference-skip: not IEC 61131-3. *)
PROGRAM main
  VAR r : ARRAY[1..CFG.N] OF INT; END_VAR
  VAR x AT %QW0 : INT; y AT %QW2 : INT; z AT %QW4 : INT; w AT %QW6 : INT; END_VAR
  x := IO.a + CFG.k;
  r[CFG.N] := IO.a;
  y := r[4];
  IO.a := IO.a + 1;
  z := a;
  CFG.t[CFG.N] := a + 1;
  w := CFG.t[4];
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
