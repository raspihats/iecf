(* A block's members hide a library's elements of their names, as
   CODESYS's shadowing rules have it ("Local variables in the function
   block ... and in any base function blocks", "Local methods of the
   POU", before "POU or type names from a library"), as TcOpen's
   cylinders call their PROPERTY StopTask's Restore() beside
   TcoDrivesBeckhoff's FUNCTION_BLOCK StopTask. The library L
   (codesysmembershadow.d/lib.st) has a block StopTask and functions
   Helper and Twice; each 1000 or its double when reached by mistake.
   Worked by hand, each cycle:
   c (Cyl): its own PROPERTY StopTask, t's Restore() 7, %QW0 7; its own
     METHOD Helper 42, %QW2 42; L.Helper() still the library's,
     %QW4 1000; Twice(3), the library's, as nothing hides it, %QW6 6.
   d (Cyl2, from Middle, from Base): the inherited PROPERTY StopTask,
     Base's ABSTRACT one given by Middle, Restore() 7, %QW8 7; the
     inherited METHOD Helper 42, %QW10 42; an inherited variable Twice,
     an instance of T, Twice.Restore() 7, %QW12 7.
   reference-skip: not IEC 61131-3. *)
INTERFACE ITask
  METHOD Restore : INT
  END_METHOD
END_INTERFACE

FUNCTION_BLOCK T IMPLEMENTS ITask
  METHOD Restore : INT
    Restore := 7;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK ABSTRACT Base
  VAR t : T; Twice : T; END_VAR
  VAR_OUTPUT r1 : INT; r2 : INT; r3 : INT; r4 : INT; END_VAR
  PROPERTY ABSTRACT StopTask : ITask
    GET
    END_GET
  END_PROPERTY
  METHOD Helper : INT
    Helper := 42;
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Middle EXTENDS Base
  PROPERTY StopTask : ITask
    GET
      StopTask := t;
    END_GET
  END_PROPERTY
END_FUNCTION_BLOCK

FUNCTION_BLOCK Cyl
  VAR t : T; END_VAR
  VAR_OUTPUT r1 : INT; r2 : INT; r3 : INT; r4 : INT; END_VAR
  PROPERTY StopTask : ITask
    GET
      StopTask := t;
    END_GET
  END_PROPERTY
  METHOD Helper : INT
    Helper := 42;
  END_METHOD
  r1 := StopTask.Restore();
  r2 := Helper();
  r3 := L.Helper();
  r4 := Twice(3);
END_FUNCTION_BLOCK

FUNCTION_BLOCK Cyl2 EXTENDS Middle
  r1 := StopTask.Restore();
  r2 := Helper();
  r3 := Twice.Restore();
END_FUNCTION_BLOCK

PROGRAM main
  VAR c : Cyl; d : Cyl2; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT;
      q4 AT %QW8 : INT; q5 AT %QW10 : INT; q6 AT %QW12 : INT; END_VAR
  c();
  d();
  q0 := c.r1; q1 := c.r2; q2 := c.r3; q3 := c.r4;
  q4 := d.r1; q5 := d.r2; q6 := d.r3;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
