(* CODESYS's and TwinCAT's UNION (union), as a STRUCT OVERLAP whose every
   member is AT %B0: un set up through d alone, 1.0's bits (16#3F800000);
   r, declared first, is not defaulted over it. Worked by hand:
   %QD0: q := un.r before un.r is changed, 1.0's bytes, 00 00 80 3f.
   un.r := 2.0 changes the same bytes un.d reads.
   %QD4: w := un.d after that, 2.0's bits read as a DWORD, 00 00 00 40.
   reference-skip: not IEC 61131-3. *)
TYPE
  U : UNION
    r : REAL;
    d : DWORD;
  END_UNION;
END_TYPE

PROGRAM main
  VAR un : U := (d := 16#3F800000); END_VAR
  VAR q AT %QD0 : REAL; w AT %QD4 : DWORD; END_VAR
  q := un.r;
  un.r := 2.0;
  w := un.d;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
