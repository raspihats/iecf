(* A library's PROGRAMs in its namespace, as CODESYS and TwinCAT have
   them: the library A (codesyslibprograms.d/a.st) and B, qualified-only
   (b.st), each with a MAIN of its own, beside this project's MAIN. A's
   Counter is called and read by its short name, through A's USING,
   from here and from A's own block User, and its method called;
   A.MAIN and B.MAIN by the library's name (program-instances).
   Worked by hand:
   cycle 0: Counter() v 10, %QW0 10; A.MAIN() n 1, %QW2 1; B.MAIN()
     x 100, %QW4 100; u(), Counter() again, %QW6 20; A.Counter.Bump(5),
     %QW8 25.
   cycle 1: 35; 2; 200; 45; 50.
   reference-skip: not IEC 61131-3. *)
PROGRAM MAIN
  VAR u : User; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT; q4 AT %QW8 : INT; END_VAR
  Counter();
  q0 := Counter.v;
  A.MAIN();
  q1 := A.MAIN.n;
  B.MAIN();
  q2 := B.MAIN.x;
  u();
  q3 := Counter.v;
  A.Counter.Bump(5);
  q4 := A.Counter.v;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : MAIN;
  END_RESOURCE
END_CONFIGURATION
