(* A method's instance variables (var-inst), as TcOpen's messenger keeps
   the cycle its StringBuilder was last read in: one for each instance,
   set up once, kept from call to call. Worked by hand, two instances:
   cycle 0: a.Next() 10 -> 11, a.Next() 12; b.Next() 11: own count;
     a.Calls (a property's GET counting its reads) 1, b.Calls 1:
     %QW0 a.Next 12, %QW2 b 11, %QW4 a.Calls + 10 * b.Calls 11;
     d, derived, its own from 10: %QW6 11.
   cycle 1: a 13, 14; b 12; Calls 2 and 2: 22; d 12.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Seq
METHOD PUBLIC Next : INT
VAR_INST
  n : INT := 10;
END_VAR
n := n + 1;
Next := n;
END_METHOD
PROPERTY PUBLIC Calls : INT
GET
VAR_INST
  reads : INT;
END_VAR
reads := reads + 1;
Calls := reads;
END_GET
END_PROPERTY
END_FUNCTION_BLOCK

FUNCTION_BLOCK Derived EXTENDS Seq
END_FUNCTION_BLOCK

PROGRAM main
  VAR a, b : Seq; d : Derived; t : INT; END_VAR
  VAR x AT %QW0 : INT; y AT %QW2 : INT; z AT %QW4 : INT; w AT %QW6 : INT; END_VAR
  t := a.Next();
  x := a.Next();
  y := b.Next();
  z := a.Calls + 10 * b.Calls;
  w := d.Next();
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
