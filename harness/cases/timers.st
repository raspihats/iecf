(* The timers of Ed.3 Table 46 against the time the platform passes in:
   TON, TOF and TP, started, held, released and restarted. ET is read in
   milliseconds with TIME arithmetic only: TIME_TO_DINT is not in Ed.3
   Figure 11. *)
FUNCTION ms : INT
VAR_INPUT
  t : TIME;
END_VAR
VAR
  i : INT;
  d : TIME;
END_VAR
ms := 0;
d := T#0ms;
FOR i := 1 TO 200 DO
  d := d + T#1ms;
  IF t >= d THEN
    ms := i;
  END_IF;
END_FOR;
END_FUNCTION

PROGRAM main
VAR
  run AT %IX0.0 : BOOL;
  pulse AT %IX0.1 : BOOL;
  on_q AT %QX0.0 : BOOL;
  off_q AT %QX0.1 : BOOL;
  tp_q AT %QX0.2 : BOOL;
  on_et AT %MW0 : INT;
  off_et AT %MW2 : INT;
  tp_et AT %MW4 : INT;
END_VAR
VAR
  t_on : TON;
  t_off : TOF;
  t_p : TP;
END_VAR
t_on(IN := run, PT := T#35ms);
t_off(IN := run, PT := T#20ms);
t_p(IN := pulse, PT := T#30ms);
on_q := t_on.Q;
off_q := t_off.Q;
tp_q := t_p.Q;
on_et := ms(t_on.ET);
off_et := ms(t_off.ET);
tp_et := ms(t_p.ET);
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
