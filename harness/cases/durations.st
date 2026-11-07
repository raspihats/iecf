(* A duration times or divided by a number (Ed.3 Table 30: MUL_TIME,
   DIV_TIME, * and /): by an integer exact, by a REAL rounded to whole
   milliseconds half to even, as REAL_TO_DINT rounds (docs/dependencies.md).
   The factor n comes from %IW0, 1 to 4 over the ticks; each bit is a
   comparison. iecf's TIME counts whole milliseconds (dividing like DINT,
   toward 0); matiec's holds nanoseconds, so its T#10s / 3 is
   3.333333333s, T#3ms * 1.5 is 4.5ms and T#-7ms / 2 is -3.5ms, none
   equal to the whole milliseconds they are compared with: iecf's trace
   is durations.iecf.trace. LTIME, which
   matiec lacks, MUL_TIME and DIV_TIME (matiec has Ed.2's MULTIME and
   DIVTIME) and a number times a duration, n * T#1s (matiec's C passes
   __time_mul its arguments the wrong way round and does not compile),
   are the generator's tests'. *)
PROGRAM main
VAR
  n AT %IW0 : INT;
  r_bits AT %MW0 : WORD;
END_VAR
VAR
  t : TIME;
  f : REAL;
END_VAR
r_bits := 0;
t := T#1s500ms * n;
IF t = T#1500ms * INT_TO_DINT(n) AND t > T#1s THEN r_bits := r_bits OR 16#0001; END_IF;
IF T#250ms * n = T#1s THEN r_bits := r_bits OR 16#0002; END_IF;
IF T#1s / n = T#250ms THEN r_bits := r_bits OR 16#0004; END_IF;
IF T#10s / n = T#3s333ms THEN r_bits := r_bits OR 16#0008; END_IF;
f := INT_TO_REAL(n) / 2.0;
IF T#1s * f = T#2s THEN r_bits := r_bits OR 16#0010; END_IF;
IF T#3ms * f = T#4ms THEN r_bits := r_bits OR 16#0020; END_IF;
IF MUL(T#100ms, n) = T#300ms THEN r_bits := r_bits OR 16#0040; END_IF;
IF DIV(T#1m, n) = T#20s THEN r_bits := r_bits OR 16#0080; END_IF;
IF T#1s / f = T#500ms THEN r_bits := r_bits OR 16#0100; END_IF;
IF T#-7ms / n = T#-3ms THEN r_bits := r_bits OR 16#0200; END_IF;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
