(* method-in-outs: a method's own in-out given to a block's in-out that
   a method of the block uses after the call, kept past the method's
   call; the call of the method gives a variable that lives as long as
   the program, two methods deep (Pass gives y to Run's x, Run to the
   block's v).
   Worked by hand, g 10: tick 0: u.Pass(y := g) binds t.v to g; u.Tick()
   bumps it: g 11. Ticks 1, 2, Pass not called: g 12, 13.
   %QW0 11, 12, 13.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Holder
  VAR_IN_OUT v : INT; END_VAR
  METHOD PUBLIC Bump
    v := v + 1;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK User
  VAR t : Holder; END_VAR
  METHOD PUBLIC Run
    VAR_IN_OUT x : INT; END_VAR
    t(v := x);
  END_METHOD
  METHOD PUBLIC Pass
    VAR_IN_OUT y : INT; END_VAR
    THIS^.Run(x := y);
  END_METHOD
  METHOD PUBLIC Tick
    t.Bump();
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR u : User; g : INT := 10; started : BOOL; END_VAR
  VAR q0 AT %QW0 : INT; END_VAR
  IF NOT started THEN
    u.Pass(y := g);
    started := TRUE;
  END_IF
  u.Tick();
  q0 := g;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
