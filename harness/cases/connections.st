(* Program connections (Ed.3 6.8.2): a program instance's inputs given a
   literal, a global or an address, its outputs written to a global or an
   address. Each input is set before the instance runs, each output
   taken after it. matiec crashes on a literal input and writes C that
   does not compile for the rest (PROPOSAL.md §1b): the trace is iecf's
   alone, connections.iecf.trace, checked by hand.

   Instance a adds 1 a tick, b adds the global ten (10), c the input
   global k (at %IW0). a's total goes to the global sum, b's to the
   global out_b (at %MD4), c's to out_c (at %MD8); a's done, inverted,
   straight to the address %QX0.0, a bare BOOL. *)
PROGRAM counter
VAR_INPUT inc : INT; END_VAR
VAR_OUTPUT total : DINT; done : BOOL; END_VAR
total := total + inc;
done := total >= 3;
END_PROGRAM

PROGRAM report
VAR_EXTERNAL sum : DINT; END_VAR
VAR r_sum AT %MD0 : DINT; END_VAR
r_sum := sum;
END_PROGRAM

CONFIGURATION plant
  VAR_GLOBAL
    ten : INT := 10;
    sum : DINT;
  END_VAR
  VAR_GLOBAL
    k AT %IW0 : INT;
    out_b AT %MD4 : DINT;
    out_c AT %MD8 : DINT;
  END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM a WITH fast : counter(inc := 1, total => sum, NOT done => %QX0.0);
    PROGRAM b WITH fast : counter(inc := ten, total => out_b);
    PROGRAM c WITH fast : counter(inc := k, total => out_c);
    PROGRAM rep WITH fast : report;
  END_RESOURCE
END_CONFIGURATION
