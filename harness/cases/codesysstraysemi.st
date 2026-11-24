(* A `;` that ends nothing (stray-semicolons), as TcOpen writes
   `END_TYPE;` after its EtherCAT structures and `p : T;;` in a VAR
   section; END_PROGRAM; too: dropped, the declarations read as
   without it.
   Worked by hand, each tick: s.a + 1 and s.b + 2 from 0; k counts.
   Tick 1: 1, 2, 1; tick 2: 2, 4, 2.
   reference-skip: not IEC 61131-3. *)
TYPE Pair :
STRUCT
  a : INT;
  b : INT;
END_STRUCT
END_TYPE;

PROGRAM main
  VAR
    s : Pair;;
    ;
    k : INT;
  END_VAR
  VAR q0 AT %QW0 : INT; q2 AT %QW2 : INT; q4 AT %QW4 : INT; END_VAR
  s.a := s.a + 1;
  s.b := s.b + 2;
  k := k + 1;
  q0 := s.a;
  q2 := s.b;
  q4 := k;
END_PROGRAM;

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
