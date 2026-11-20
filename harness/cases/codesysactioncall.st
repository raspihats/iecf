(* CODESYS's and TwinCAT's direct call of an ACTION (action-call):
   counter's own tick, which s0 runs every scan (N, never leaving it),
   calls inc bare; main calls inc too, through the instance, from
   outside. Neither call is tick's: a step runs that one, and a direct
   call of it is refused (crates/iecf-sema/tests/check.rs).
   Worked by hand: each tick, c() runs tick (+1) then main calls
   inc() itself (+1): n grows by 2 a tick, q := c.n after both.
   %QW0: tick 0 q 2, tick 1 q 4, tick 2 q 6.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK counter
  VAR_OUTPUT n : INT; END_VAR
  INITIAL_STEP s0 : tick(N); END_STEP
  TRANSITION FROM s0 TO s0 := FALSE; END_TRANSITION
  ACTION tick : inc(); END_ACTION
  ACTION inc : n := n + 1; END_ACTION
END_FUNCTION_BLOCK

PROGRAM main
  VAR c : counter; END_VAR
  VAR q AT %QW0 : INT; END_VAR
  c();
  c.inc();
  q := c.n;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
