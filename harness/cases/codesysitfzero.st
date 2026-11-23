(* An interface compared with 0 (codesys-oo), CODESYS's test of one for
   NULL, as TcOpen tests its parent before reaching through it.
   Worked by hand:
   cycle 0: ref NULL: %QW0 (ref = 0) 1; the IF's AND_THEN calls no method
     of NULL: %QW2 0; ref := blk: %QW4 (ref <> 0) 1; %QW6 0 <> ref, 1.
   cycle 1: ref is blk's from cycle 0: %QW0 0; %QW2 blk.Get() 5; 1; 1.
   0 given to an interface (codesys-oo), its NULL: r2 starts 0, is blk
   in the cycle, and 0 again at its end. Each cycle: %QW8 (r2 = 0) 1;
   %QW10 (r2 <> 0) once r2 := blk, 1; %QW12 Has(i := 0) + 10 *
   Has(i := blk) = 0 + 10 = 10; %QW14 (Pick() = 0) 1; %QW16 r2's
   method not called once r2 := 0: 7.
   reference-skip: not IEC 61131-3. *)
INTERFACE I
METHOD Get : INT
END_METHOD
END_INTERFACE

FUNCTION Has : INT
  VAR_INPUT i : I; END_VAR
  Has := BOOL_TO_INT(i <> 0);
END_FUNCTION

FUNCTION Pick : I
  Pick := 0;
END_FUNCTION

FUNCTION_BLOCK A IMPLEMENTS I
METHOD PUBLIC Get : INT
Get := 5;
END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR blk : A; ref : I; r2 : I := 0; END_VAR
  VAR x AT %QW0 : INT; y AT %QW2 : INT; z AT %QW4 : INT; w AT %QW6 : INT; END_VAR
  VAR v AT %QW8 : INT; q AT %QW10 : INT; u AT %QW12 : INT; t AT %QW14 : INT; s AT %QW16 : INT; END_VAR
  v := BOOL_TO_INT(r2 = 0);
  r2 := blk;
  q := BOOL_TO_INT(r2 <> 0);
  u := Has(i := 0) + 10 * Has(i := blk);
  t := BOOL_TO_INT(Pick() = 0);
  r2 := 0;
  s := 7;
  IF r2 <> 0 THEN
    s := r2.Get();
  END_IF;
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
