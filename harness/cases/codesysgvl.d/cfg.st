(* The global variable list CFG, qualified-only: CFG.k, never k. Its own
   variables are named bare inside it. *)
VAR_GLOBAL CONSTANT
  N : INT := 4;
END_VAR
VAR_GLOBAL
  k : INT := 7;
  t : ARRAY[1..N] OF INT;
END_VAR
