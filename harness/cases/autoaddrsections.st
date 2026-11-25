(* TwinCAT's `*` structures where Ed.3 6.4.4.8 refuses them
   (auto-addresses, W0020), as TcOpen's TcoEthercatMasterDevice: a
   block's VAR_INPUT, placed per instance as a VAR's, and a method's
   VAR, plain memory of the call; and its VAR_OUTPUT, which Ed.3
   allows, placed per instance.
   Placed by path, nothing stated: M: P.M.INPUTS.A 0, P.SRC.A 2.
   Q: P.M.INPUTS.B 0, P.M.OUTPUTS.Q 2, P.SRC.B 4.
   Worked by hand, k counting 1, 2: src.a := k, src.b := k * 10; the
   call m(Inputs := src) writes m.Inputs' places; Sum copies Inputs into
   its own copy, adds 100 to the copy's a (not the image's), gives
   a + b to Outputs.q.
   Tick 0: Q 10, 111, 10; M 1, 1. Tick 1: Q 20, 122, 20; M 2, 2.
   reference-skip: not IEC 61131-3. *)
TYPE Io : STRUCT
  a AT %M* : INT;
  b AT %Q* : INT;
END_STRUCT;
END_TYPE
TYPE Out : STRUCT
  q AT %Q* : INT;
END_STRUCT;
END_TYPE

FUNCTION_BLOCK Master
  VAR_INPUT Inputs : Io; END_VAR
  VAR_OUTPUT Outputs : Out; END_VAR
  METHOD Sum : INT
    VAR copy : Io; END_VAR
    copy := Inputs;
    copy.a := copy.a + 100;
    Sum := copy.a + copy.b;
  END_METHOD
  Outputs.q := THIS.Sum();
END_FUNCTION_BLOCK

PROGRAM main
  VAR k : INT; src : Io; m : Master; END_VAR
  k := k + 1;
  src.a := k;
  src.b := k * 10;
  m(Inputs := src);
END_PROGRAM

CONFIGURATION cf
  RESOURCE rs ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
