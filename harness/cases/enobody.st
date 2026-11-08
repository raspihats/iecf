(* ENO set in a POU's own body (Ed.3 6.6.1.5 rules 2, 3 and 7;
   docs/dependencies.md): ENO is there in every FUNCTION and
   FUNCTION_BLOCK without a declaration, TRUE as each run starts, and the
   caller's ENO is FALSE when the body set it FALSE or a flag was raised
   while it ran. `lim` clears ENO above 100, `half` below 0; `acc` sets
   ENO TRUE at its end, which cannot hide the overflow of its fifth run
   (tick 4): iecf's ENO is FALSE there, the reference compiler's TRUE, as
   it has no flag; iecf's trace is enobody.iecf.trace. The input v is
   5, 200, -4, 30000, 30000. r_bits: bit 0 lim's ENO, 1 half's, 2 acc's. *)
FUNCTION_BLOCK lim
VAR_INPUT x : INT; END_VAR
VAR_OUTPUT y : INT; END_VAR
y := x;
IF x > 100 THEN ENO := FALSE; END_IF;
END_FUNCTION_BLOCK

FUNCTION half : INT
VAR_INPUT x : INT; END_VAR
half := x / 2;
ENO := x >= 0;
END_FUNCTION

FUNCTION_BLOCK acc
VAR_INPUT x : INT; END_VAR
VAR_OUTPUT total : INT; END_VAR
total := total + x;
ENO := TRUE;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  v AT %IW0 : INT;
  r_bits AT %MB0 : BYTE;
  r_half AT %MB1 : SINT;
  r_y AT %MW2 : INT;
  r_total AT %MW4 : INT;
END_VAR
VAR
  l : lim;
  a : acc;
  ok1, ok2, ok3 : BOOL;
  h : INT;
END_VAR
l(x := v, y => r_y, ENO => ok1);
h := half(x := v, ENO => ok2);
r_half := INT_TO_SINT(h / 1000);
a(x := v, total => r_total, ENO => ok3);
r_bits := 0;
IF ok1 THEN r_bits := r_bits OR 16#01; END_IF;
IF ok2 THEN r_bits := r_bits OR 16#02; END_IF;
IF ok3 THEN r_bits := r_bits OR 16#04; END_IF;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
