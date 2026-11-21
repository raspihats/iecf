(* CODESYS's and TwinCAT's SUPER^ (super-calls), as TcOpen writes it.
   Worked by hand, d called once with x = 100:
   SUPER^(i := 3, io := io, o => got): the base's body with i = 3,
   io bound to itself: n 1, o 30, x 103; got 30.
   SUPER^(i := 4), a second call in the body: n 2, o 40, x 107.
   THIS^.Again(), a method calling SUPER^() on i = 4: n 3, o 40, x 111.
   SUPER^.Kind, the base's GET, not Derived's: 1; THIS^.Kind: 2.
   k := SUPER^.n, the instance's own n: 3; SUPER^.n := 10, m := n: 10.
   %QW0 got 30, %QW2 o 40, %QW4 x 111, %QW6 1, %QW8 2, %QW10 3, %QW12 10.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Base
  VAR_INPUT i : INT; END_VAR
  VAR_IN_OUT io : INT; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  VAR n : INT; END_VAR
  PROPERTY PUBLIC Kind : INT
    GET
      Kind := 1;
    END_GET
  END_PROPERTY
  n := n + 1;
  o := i * 10;
  io := io + i;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Derived EXTENDS Base
  VAR_OUTPUT got : INT; viaSuper : INT; own : INT; k : INT; m : INT; END_VAR
  PROPERTY PUBLIC Kind : INT
    GET
      Kind := 2;
    END_GET
  END_PROPERTY
  METHOD PUBLIC Again
    SUPER^();
  END_METHOD
  SUPER^(i := 3, io := io, o => got);
  SUPER^(i := 4);
  THIS^.Again();
  viaSuper := SUPER^.Kind;
  own := THIS^.Kind;
  k := SUPER^.n;
  SUPER^.n := 10;
  m := n;
END_FUNCTION_BLOCK

PROGRAM main
  VAR d : Derived; x : INT := 100; END_VAR
  VAR
    q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT;
    q4 AT %QW8 : INT; q5 AT %QW10 : INT; q6 AT %QW12 : INT;
  END_VAR
  d(io := x);
  q0 := d.got; q1 := d.o; q2 := x; q3 := d.viaSuper; q4 := d.own; q5 := d.k; q6 := d.m;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
