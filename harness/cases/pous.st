(* The user's POUs: a FUNCTION with inputs by name and by position, a
   FUNCTION_BLOCK keeping state across ticks, VAR_IN_OUT, VAR_OUTPUT read
   from outside, an FB instance inside an FB, and globals of the
   CONFIGURATION through VAR_EXTERNAL. *)
FUNCTION scale : INT
VAR_INPUT
  x : INT;
  num : INT := 1;
  den : INT := 1;
END_VAR
scale := x * num / den;
END_FUNCTION

FUNCTION_BLOCK average
VAR_INPUT
  sample : INT;
END_VAR
VAR_OUTPUT
  avg : INT;
  count : INT;
END_VAR
VAR
  sum : DINT;
END_VAR
sum := sum + INT_TO_DINT(sample);
count := count + 1;
avg := DINT_TO_INT(sum / INT_TO_DINT(count));
END_FUNCTION_BLOCK

FUNCTION_BLOCK twice
VAR_IN_OUT
  v : INT;
END_VAR
VAR_OUTPUT
  inner_avg : INT;
END_VAR
VAR
  a : average;
END_VAR
v := v * 2;
a(sample := v);
inner_avg := a.avg;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  inp AT %IW0 : INT;
  r_pos AT %MW0 : INT;
  r_named AT %MW2 : INT;
  r_default AT %MW4 : INT;
  r_avg AT %MW6 : INT;
  r_count AT %MW8 : INT;
  r_inout AT %MW10 : INT;
  r_inner AT %MW12 : INT;
  r_global AT %MW14 : INT;
END_VAR
VAR
  avg1 : average;
  tw : twice;
  local : INT := 1;
END_VAR
VAR_EXTERNAL
  g_total : INT;
END_VAR
VAR_EXTERNAL CONSTANT
  g_limit : INT;
END_VAR
r_pos := scale(inp, 3, 2);
r_named := scale(den := 4, x := inp, num := 2);
r_default := scale(x := inp);
avg1(sample := inp);
r_avg := avg1.avg;
r_count := avg1.count;
tw(v := local);
r_inout := local;
r_inner := tw.inner_avg;
IF g_total < g_limit THEN
  g_total := g_total + inp;
END_IF;
r_global := g_total;
END_PROGRAM

CONFIGURATION plant
  VAR_GLOBAL
    g_total : INT := 0;
  END_VAR
  VAR_GLOBAL CONSTANT
    g_limit : INT := 50;
  END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
