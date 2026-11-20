(* An interface compared with 0 (codesys-oo), CODESYS's test of one for
   NULL, as TcOpen tests its parent before reaching through it.
   Worked by hand:
   cycle 0: ref NULL: %QW0 (ref = 0) 1; the IF's AND_THEN calls no method
     of NULL: %QW2 0; ref := blk: %QW4 (ref <> 0) 1; %QW6 0 <> ref, 1.
   cycle 1: ref is blk's from cycle 0: %QW0 0; %QW2 blk.Get() 5; 1; 1.
   reference-skip: not IEC 61131-3. *)
INTERFACE I
METHOD Get : INT
END_METHOD
END_INTERFACE

FUNCTION_BLOCK A IMPLEMENTS I
METHOD PUBLIC Get : INT
Get := 5;
END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR blk : A; ref : I; END_VAR
  VAR x AT %QW0 : INT; y AT %QW2 : INT; z AT %QW4 : INT; w AT %QW6 : INT; END_VAR
  x := BOOL_TO_INT(ref = 0);
  y := 0;
  IF ref <> 0 AND_THEN ref.Get() > 0 THEN
    y := ref.Get();
  END_IF;
  ref := blk;
  z := BOOL_TO_INT(ref <> 0);
  w := BOOL_TO_INT(0 <> ref);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
