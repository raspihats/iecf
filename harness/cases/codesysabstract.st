(* An ABSTRACT function block with no ABSTRACT method (codesys-oo), as
   TcOpen's TcoObject and TcoStateBase: ABSTRACT says only that it has
   no instances. Base's Run and Name are implemented; Job EXTENDS Base,
   overrides Name, and is reached through a REFERENCE TO Base, its
   override dynamically bound.
   Worked by hand, each tick: j.count + 1 through Base's Run, Name of
   Job 7 (Base's would be 1). Tick 1: 1, 7; tick 2: 2, 7.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK ABSTRACT Base
  VAR_OUTPUT count : INT; END_VAR
  METHOD Run
    count := count + 1;
  END_METHOD
  METHOD Name : INT
    Name := 1;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Job EXTENDS Base
  METHOD Name : INT
    Name := 7;
  END_METHOD
END_FUNCTION_BLOCK

PROGRAM main
  VAR j : Job; r : REFERENCE TO Base; END_VAR
  VAR q0 AT %QW0 : INT; q2 AT %QW2 : INT; END_VAR
  r REF= j;
  r.Run();
  q0 := j.count;
  q2 := r.Name();
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
