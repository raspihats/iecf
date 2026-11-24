(* A block with methods used as CODESYS and TwinCAT let one be:
   declared in a method's or a FUNCTION's VAR, set up at each call
   (temp-instances), and, of its own type only, copied with := and
   given as an input (codesys-oo), as TcOpen's `_signalInfo :
   TcoSignalInfo` in a method, `_content := inoContent`, `inSignalInfo :
   TcoSignalInfo`.
   Kid EXTENDS Info and overrides Inc (+10, not +1); Info.Twice calls
   Inc through THIS^, so a Kid set up at each call must have Kid's
   methods: Twice gives 20, never 2, and starts from n = 0 each call.
   Worked by hand, each tick: Local and Fun give 20 every tick (a kept
   instance would give 40, 60); src is bumped, copied into u.inInfo by
   u(inInfo := src), bumped again, which the copy does not see, then
   copied into u.viaRef through a REFERENCE TO; u.kept := u.inInfo.
   Tick t: src.n is 2t-1 when copied in, 2t when taken by reference.
   Tick 1: 20, 20, 1, 1, 2; tick 2: 20, 20, 3, 3, 4.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Info
  VAR_OUTPUT n : INT; END_VAR
  METHOD Inc
    n := n + 1;
  END_METHOD
  METHOD Twice : INT
    THIS^.Inc();
    THIS^.Inc();
    Twice := n;
  END_METHOD
  METHOD Bump
    n := n + 1;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Kid EXTENDS Info
  METHOD Inc
    n := n + 10;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION Fun : INT
  VAR k : Kid; END_VAR
  Fun := k.Twice();
END_FUNCTION

FUNCTION_BLOCK User
  VAR_INPUT inInfo : Info; END_VAR
  VAR kept : Info; viaRef : Info; END_VAR
  METHOD Local : INT
    VAR k : Kid; END_VAR
    Local := k.Twice();
  END_METHOD
  METHOD Take
    VAR_INPUT other : REFERENCE TO Info; END_VAR
    viaRef := other;
  END_METHOD
  kept := inInfo;
END_FUNCTION_BLOCK

PROGRAM main
  VAR u : User; src : Info; END_VAR
  VAR q0 AT %QW0 : INT; q2 AT %QW2 : INT; q4 AT %QW4 : INT; q6 AT %QW6 : INT; q8 AT %QW8 : INT; END_VAR
  src.Bump();
  u(inInfo := src);
  src.Bump();
  u.Take(other := src);
  q0 := u.Local();
  q2 := Fun();
  q4 := u.inInfo.n;
  q6 := u.kept.n;
  q8 := u.viaRef.n;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
