(* pointers: a pointer given as an input kept in a VAR only assigned and
   compared with <> (W0018), as TcOpen's TcoSignalInfo remembers which
   signal it last looked up.
   Worked by hand, n counting the changes: tick 0: Get(ADR(g)) n 1,
   Get(ADR(g)) the same, n 1; Get(ADR(h)) n 2. Tick 1: Get(ADR(h)) the
   same, n 2; Get(ADR(g)) n 3; Get(ADR(g)) n 3.
   %QW0 2, then 3.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Info
  VAR pLast : POINTER TO INT; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  METHOD PUBLIC Get
    VAR_INPUT p : POINTER TO INT; END_VAR
    IF pLast <> p THEN
      n := n + 1;
      pLast := p;
    END_IF;
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR i : Info; g : INT; h : INT; t : INT; END_VAR
  VAR q0 AT %QW0 : INT; END_VAR
  IF t = 0 THEN
    i.Get(p := ADR(g));
    i.Get(p := ADR(g));
    i.Get(p := ADR(h));
  ELSE
    i.Get(p := ADR(h));
    i.Get(p := ADR(g));
    i.Get(p := ADR(g));
  END_IF;
  t := t + 1;
  q0 := i.n;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
