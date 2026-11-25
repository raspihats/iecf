(* method-in-outs: a method's own in-out bound by REF= to a reference
   of its block, which outlives the call, as TcOpen's
   TcoInspector.UpdateComprehensiveResult(VAR_IN_OUT inoResult) keeps
   `_refOverallResult REF= inoResult`: kept past the method's call, and
   each call of the method gives a variable that lives as long as the
   program, two methods deep (Pass gives y to Update's x).
   Worked by hand, g 1: tick 0: k.Pass(y := g) binds k.r to g;
   k.Tick() adds 5 through it: g 6. Ticks 1, 2, Pass not called: 11,
   16. %QW0 6, 11, 16.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Keeper
  VAR r : REFERENCE TO INT; END_VAR
  METHOD PUBLIC Update
    VAR_IN_OUT x : INT; END_VAR
    r REF= x;
  END_METHOD
  METHOD PUBLIC Pass
    VAR_IN_OUT y : INT; END_VAR
    THIS^.Update(x := y);
  END_METHOD
  METHOD PUBLIC Tick
    IF __ISVALIDREF(r) THEN
      r := r + 5;
    END_IF
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR k : Keeper; g : INT := 1; started : BOOL; END_VAR
  VAR q0 AT %QW0 : INT; END_VAR
  IF NOT started THEN
    k.Pass(y := g);
    started := TRUE;
  END_IF
  k.Tick();
  q0 := g;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
