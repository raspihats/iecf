(* A `;` closing a header before its declarations (header-semicolon),
   as TcOpen's TcoInsight writes `METHOD SetUserDataAsByte : BOOL;`, and
   a method's variable named as its block's (shadowing), hiding it in
   the method, THIS^.n still the block's.
   Worked by hand, each tick: b(n := 100) sets the block's n, hits + 1;
   b.Add(n := 5) adds its own n, 5, to total and 1 to the block's n,
   101; b.Peek() reads the block's n, 101; F(a := 21) 42.
   Tick 1: Add 5, Peek 101, b.n 101, hits 1, F 42.
   Tick 2: Add 10, Peek 101, b.n 101, hits 2, F 42.
   reference-skip: not IEC 61131-3. *)
FUNCTION F : INT;
  VAR_INPUT a : INT; END_VAR
  F := a * 2;
END_FUNCTION

FUNCTION_BLOCK Blk
  VAR_INPUT n : INT; END_VAR
  VAR_OUTPUT hits : INT; END_VAR
  VAR total : INT; END_VAR
  METHOD Add : INT;
    VAR_INPUT n : INT; END_VAR
    total := total + n;
    THIS^.n := THIS^.n + 1;
    Add := total;
  END_METHOD
  METHOD Peek : INT;
    Peek := n;
  END_METHOD
  hits := hits + 1;
END_FUNCTION_BLOCK

PROGRAM main
  VAR b : Blk; END_VAR
  VAR q0 AT %QW0 : INT; q2 AT %QW2 : INT; q4 AT %QW4 : INT; q6 AT %QW6 : INT; q8 AT %QW8 : INT; END_VAR
  b(n := 100);
  q0 := b.Add(n := 5);
  q2 := b.Peek();
  q4 := b.n;
  q6 := b.hits;
  q8 := F(a := 21);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
