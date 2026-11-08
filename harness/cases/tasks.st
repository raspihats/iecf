(* Two tasks of different intervals: which program instances run on which
   tick, in the order the configuration lists them, and two instances of
   one program, each with its own state.

   No connections to the programs' inputs and outputs: the reference compiler 7949c0b
   crashes (SIGSEGV) on a literal given to a program input,
   `counter(inc := 1)`, and the C it writes for a connection to a global
   does not compile (`B.INC = *TEN`). Ed.2 allows both. *)
PROGRAM bump
VAR
  mine : INT;
END_VAR
VAR_EXTERNAL
  total : INT;
  order : INT;
END_VAR
mine := mine + 1;
total := total + 1;
order := (order MOD 1000) * 10 + mine;
END_PROGRAM

PROGRAM report
VAR
  runs AT %MW4 : INT;
END_VAR
VAR_EXTERNAL
  total : INT;
  seen : INT;
END_VAR
runs := runs + 1;
seen := total;
END_PROGRAM

CONFIGURATION plant
  VAR_GLOBAL
    total AT %MW0 : INT;
    order AT %MW2 : INT;
    seen AT %MW6 : INT;
  END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    TASK slow(INTERVAL := T#30ms, PRIORITY := 2);
    PROGRAM a WITH fast : bump;
    PROGRAM b WITH slow : bump;
    PROGRAM c WITH slow : report;
  END_RESOURCE
END_CONFIGURATION
