(* SFC (Ed.3 6.7; docs/behaviour/sfc.md): one scan runs the action
   control, the actions (those ending first), then the transitions of the
   steps active when it began. S1 holds every kind of association: a BOOL
   variable as an action (lamp, N), S and R (Sto, reset in S4), L and D of
   20 ms (Lim, Del), P1 and P0 (Ent, Ext); its transition waits for
   S1.T >= T#40ms, then divides into S2 and S3, which converge into S4.
   From S4 two transitions: (PRIORITY := 1) back to S0 when back, else
   (PRIORITY := 2) to S5. Ent RETURNs after counting: only the action
   ends. The x outputs show the active steps (BOOL variables as N
   actions). A scan is 10 ms. matiec tests transitions first and has no
   final scan, nor PRIORITY: this case is iecf's alone (matiec-skip),
   its trace checked by hand. *)
PROGRAM main
VAR
  go AT %IX0.0 : BOOL;
  back AT %IX0.1 : BOOL;
  lamp AT %QX0.0 : BOOL;
  stored AT %QX0.1 : BOOL;
  limited AT %QX0.2 : BOOL;
  delayed AT %QX0.3 : BOOL;
  ent AT %QB1 : USINT;
  ext AT %QB2 : USINT;
  x0 AT %QX3.0 : BOOL;
  x1 AT %QX3.1 : BOOL;
  x2 AT %QX3.2 : BOOL;
  x3 AT %QX3.3 : BOOL;
  x4 AT %QX3.4 : BOOL;
  x5 AT %QX3.5 : BOOL;
END_VAR

INITIAL_STEP S0:
  x0(N);
END_STEP
STEP S1:
  x1(N);
  lamp(N);
  Sto(S);
  Lim(L, T#20ms);
  Del(D, T#20ms);
  Ent(P1);
  Ext(P0);
END_STEP
STEP S2:
  x2(N);
END_STEP
STEP S2b:
END_STEP
STEP S3:
  x3(N);
END_STEP
STEP S4:
  x4(N);
  Sto(R);
END_STEP
STEP S5:
  x5(N);
END_STEP

TRANSITION FROM S0 TO S1 := go; END_TRANSITION
TRANSITION FROM S1 TO (S2, S3) := S1.T >= T#40ms; END_TRANSITION
TRANSITION FROM S2 TO S2b := TRUE; END_TRANSITION
TRANSITION FROM (S2b, S3) TO S4 := TRUE; END_TRANSITION
TRANSITION (PRIORITY := 2) FROM S4 TO S5 := TRUE; END_TRANSITION
TRANSITION (PRIORITY := 1) FROM S4 TO S0 := back; END_TRANSITION
TRANSITION FROM S5 TO S0 := TRUE; END_TRANSITION

ACTION Sto:
  stored := Sto.Q;
END_ACTION
ACTION Lim:
  limited := Lim.Q;
END_ACTION
ACTION Del:
  delayed := Del.Q;
END_ACTION
ACTION Ent:
  ent := ent + 1;
  RETURN;
  ent := ent + 100;
END_ACTION
ACTION Ext:
  ext := ext + 1;
END_ACTION
END_PROGRAM

CONFIGURATION plant
  RESOURCE res ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
