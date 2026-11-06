(* An infinite or NaN REAL carries on and is flagged, not faulted
   (docs/c-output.md §5, decided 2026-11-06). At tick 1 the product
   overflows to +infinity; at tick 2 infinity minus infinity is a NaN.
   Both compilers write the same bytes and the program goes on counting;
   iecf's trace also shows ctx->flag: the first, an infinity (code 2) at
   line 21, stays until the platform clears it. *)
PROGRAM main
VAR
  k AT %IW0 : INT;
  r_mul AT %MD0 : REAL;
  r_diff AT %MD4 : REAL;
  ticks AT %MW8 : INT;
END_VAR
VAR
  x : REAL;
END_VAR
x := 1.0E5;
IF k >= 1 THEN
  x := 1.0E10;
END_IF;
r_mul := x * 1.0E30;
IF k >= 2 THEN
  r_diff := r_mul - r_mul;
END_IF;
ticks := ticks + 1;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
