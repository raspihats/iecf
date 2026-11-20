(* A named value as a CASE label (Ed.3 6.4.4.3, 7.3.3.3.3): a constant,
   its number compared with the selector's. Found in TcOpen, where it
   was E0032. Worked by hand:
   %QW0: d is eDist#Y (20): the label eDist#Y, 2.
   %QW2: n is eNum#Q (5): the label eNum#Q, 4.
   %QW4: d's number through its base type, 20.
   reference-skip: matiec has no named values with values given. *)
TYPE eDist : (X := 10, Y := 20, Z := 30); END_TYPE
TYPE eNum : INT (P := 1, Q := 5); END_TYPE

PROGRAM main
  VAR d : eDist := eDist#Y; n : eNum := eNum#Q; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; END_VAR
  CASE d OF
    eDist#X: q0 := 1;
    eDist#Y: q0 := 2;
  ELSE
    q0 := 3;
  END_CASE;
  CASE n OF
    eNum#P: q1 := 3;
    eNum#Q: q1 := 4;
  END_CASE;
  q2 := TO_INT(d);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
