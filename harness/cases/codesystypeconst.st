(* A block's constant read through its type's name (type-constants), as
   TcOpen sizes its messages' strings by TcoMessengerEnv's. Worked by
   hand: Env.N 8, Env.TWICE N * 2 = 16.
   %QW0 Env.TWICE + the upper bound of a, ARRAY[1..Env.TWICE]: 32;
   %QW2 s, a STRING[Env.N], given 11 characters: cut to 8 (flagged), LEN 8;
   %QW4 Env.HALF, an LREAL 3.5, * 2: 7. The same each cycle.
   reference-skip: not IEC 61131-3. *)
FUNCTION_BLOCK Env
VAR CONSTANT
  N : INT := 8;
  TWICE : INT := N * 2;
  HALF : LREAL := 3.5;
END_VAR
END_FUNCTION_BLOCK

PROGRAM main
  VAR s : STRING[Env.N]; a : ARRAY[1..Env.TWICE] OF INT; END_VAR
  VAR x AT %QW0 : INT; y AT %QW2 : INT; z AT %QW4 : INT; END_VAR
  x := Env.TWICE + DINT_TO_INT(UPPER_BOUND(a, 1));
  s := 'abcdefghijk';
  y := LEN(s);
  z := LREAL_TO_INT(Env.HALF * 2.0);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
