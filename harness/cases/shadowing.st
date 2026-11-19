(* shadowing: a POU's variable named as a type, a POU or a standard
   block or function; in its POU the name is the variable. Worked by
   hand:
   Length, a FUNCTION_BLOCK, counts its calls in n; Speed, a TYPE, a
   structure. In main: length : INT (the block's name) := 7; speed :
   Speed, a variable of the type it is named as, speed.v := 3; tp : BOOL
   (TP, a standard block) beside t : TP, an instance of the block, which
   the name still means as a type; max : INT := 40 (MAX, a standard
   function).
   %QB0: length + speed.v, 10. %QB1: c, an instance of Length, called
   twice: its n is 2. %QB2: t's Q, TRUE on its first call (PT T#1s),
   copied into tp: 1. %QB3: max + 2, 42.
   reference-skip: shadowing is CODESYS's, not IEC 61131-3. *)
FUNCTION_BLOCK Length
  VAR_OUTPUT n : INT; END_VAR
  n := n + 1;
END_FUNCTION_BLOCK

TYPE Speed : STRUCT v : INT; END_STRUCT; END_TYPE

PROGRAM main
  VAR
    length : INT := 7;
    speed : Speed;
    c : Length;
    tp : BOOL;
    t : TP;
    max : INT := 40;
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
  END_VAR
  speed.v := 3;
  q0 := INT_TO_SINT(length + speed.v);
  c();
  c();
  q1 := INT_TO_SINT(c.n);
  t(IN := TRUE, PT := T#1s);
  tp := t.Q;
  q2 := SEL(tp, SINT#0, SINT#1);
  q3 := INT_TO_SINT(max + 2);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
