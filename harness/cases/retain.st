(* RETAIN (Ed.3 6.5.6): a warm restart sets every variable to its initial
   value but the RETAIN ones, which keep theirs (docs/c-output.md §7); a
   cold start sets them too. Globals and a program's variables, a whole
   function block instance among them, count each tick. At tick 3 a warm
   restart: the plain counters start again, the RETAIN ones go on.
   The reference compiler has no warm restart of its own; its host starts everything
   anew. iecf's trace is retain.iecf.trace. *)
PROGRAM main
VAR_EXTERNAL
  g_plain : INT;
  g_kept : INT;
END_VAR
VAR
  r_plain AT %MW0 : INT;
  r_kept AT %MW2 : INT;
  r_g_plain AT %MW4 : INT;
  r_g_kept AT %MW6 : INT;
  r_ctu AT %MW8 : INT;
  r_arr AT %MW10 : INT;
END_VAR
VAR
  plain : INT := 100;
END_VAR
VAR RETAIN
  kept : INT := 100;
  c : CTU;
  arr : ARRAY [1..2] OF INT := [7, 8];
END_VAR
plain := plain + 1;
kept := kept + 1;
g_plain := g_plain + 1;
g_kept := g_kept + 1;
c(CU := NOT c.CU, PV := 1000);
arr[2] := arr[2] + 1;
r_plain := plain;
r_kept := kept;
r_g_plain := g_plain;
r_g_kept := g_kept;
r_ctu := c.CV;
r_arr := arr[2];
END_PROGRAM

CONFIGURATION plant
  VAR_GLOBAL g_plain : INT; END_VAR
  VAR_GLOBAL RETAIN g_kept : INT := 50; END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
