(* Program connections (Ed.3 6.8.2) of a reference, in and out, and of
   NULL; a CHAR output into a STRING global and a WCHAR one into a
   WSTRING global (Figure 11, implicit). The three programs run in
   order on one task. Worked by hand: setup points gr at gi (5). p1 gets
   gr, adds 1 to what it references and gives it back as ro, into gro;
   p2 gets NULL and does nothing, its ro NULL into gro2. Tick 0: gi 6
   (%QB0), gro^ 6 (%QB1), gro2 NULL (%QX2.0), gs 'k' (%QX2.1), gws "z"
   (%QX2.2). Tick 1: gi 7, gro^ 7.
   reference-skip: program connections, as connections; and REF_TO. *)
PROGRAM setup
  VAR_EXTERNAL gi : INT; gr : REF_TO INT; END_VAR
  gr := REF(gi);
END_PROGRAM

PROGRAM prod
  VAR_INPUT r : REF_TO INT; END_VAR
  VAR_OUTPUT ro : REF_TO INT; c : CHAR; wc : WCHAR; END_VAR
  IF r <> NULL THEN
    r^ := r^ + 1;
  END_IF;
  ro := r;
  c := 'k';
  wc := WCHAR#"z";
END_PROGRAM

PROGRAM show
  VAR_EXTERNAL gi : INT; gro, gro2 : REF_TO INT; gs : STRING[4]; gws : WSTRING[4]; END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    b0 AT %QX2.0 : BOOL;
    b1 AT %QX2.1 : BOOL;
    b2 AT %QX2.2 : BOOL;
  END_VAR
  q0 := INT_TO_SINT(gi);
  IF gro <> NULL THEN
    q1 := INT_TO_SINT(gro^);
  END_IF;
  b0 := gro2 = NULL;
  b1 := gs = 'k';
  b2 := gws = "z";
END_PROGRAM

CONFIGURATION cf
  VAR_GLOBAL
    gi : INT := 5;
    gr, gro, gro2 : REF_TO INT;
    gs : STRING[4];
    gws : WSTRING[4];
  END_VAR
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM s WITH t : setup;
    PROGRAM p1 WITH t : prod(r := gr, ro => gro, c => gs, wc => gws);
    PROGRAM p2 WITH t : prod(r := NULL, ro => gro2);
    PROGRAM v WITH t : show;
  END_RESOURCE
END_CONFIGURATION
