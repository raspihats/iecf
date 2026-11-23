(* CODESYS's and TwinCAT's arrays of instances set up by their FB_init
   (fb-init), as TcOpen's `ARRAY[0..2] OF DiagnosticsMessenger(THIS^)`,
   and in-outs of a type with an FB_init, bound, not set up.
   Worked by hand: h's items, its first VAR, are set up first, each
   element's FB_init in index order given THIS^, h: each takes h.Next(),
   1 2 3; then grid, row by row, (1,-1) (1,0) (2,-1) (2,0): 4 5 6 7;
   then plain, with no arguments, each element's FB_init counting itself
   once, bInitRetains TRUE at the cold start; then d, a structure whose
   member cells is such an array, then nest, an array of arrays.
   %QW0 items[0..2] 1 2 3 as 123, %QW2 grid as 4567, %QW4 h.seq 7,
   %QW6 plain's counts summed, 3, %QW8 those cold, 3, %QW10 d.cells'
   counts summed, 2, and nest's, 4, times 10: 42; %QW12 h.u reading items[2] through its in-out, 3,
   %QW14 its method reading grid[2, 0] through one, 7.
   reference-skip: not IEC 61131-3. *)
INTERFACE ISeq
  METHOD Next : INT
  END_METHOD
END_INTERFACE

FUNCTION_BLOCK Item
  VAR_OUTPUT mine : INT; END_VAR
  METHOD FB_init : BOOL
    VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; s : ISeq; END_VAR
    mine := s.Next();
  END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Counted
  VAR_OUTPUT times : INT; cold : INT; END_VAR
  METHOD FB_init : BOOL
    VAR_INPUT bInitRetains : BOOL; bInCopyCode : BOOL; END_VAR
    times := times + 1;
    IF bInitRetains THEN cold := cold + 1; END_IF
  END_METHOD
END_FUNCTION_BLOCK

TYPE Cells : STRUCT
  tag : INT := 1;
  cells : ARRAY[0..1] OF Counted;
END_STRUCT
END_TYPE

FUNCTION_BLOCK User
  VAR_IN_OUT io : Item; END_VAR
  VAR_OUTPUT o : INT; END_VAR
  METHOD Read : INT
    VAR_IN_OUT other : Item; END_VAR
    Read := other.mine;
  END_METHOD
  o := io.mine;
END_FUNCTION_BLOCK

FUNCTION_BLOCK Holder IMPLEMENTS ISeq
  VAR
    seq : INT;
    items : ARRAY[0..2] OF Item(THIS^);
    grid : ARRAY[1..2, -1..0] OF Item(THIS^);
    plain : ARRAY[0..2] OF Counted;
    d : Cells;
    nest : ARRAY[0..1] OF ARRAY[0..1] OF Counted;
    u : User;
  END_VAR
  VAR_OUTPUT o1 : INT; o2 : INT; o3 : INT; o4 : INT; o5 : INT; o6 : INT; o7 : INT; o8 : INT; END_VAR
  VAR_TEMP k : INT; END_VAR
  METHOD PUBLIC Next : INT
    seq := seq + 1;
    Next := seq;
  END_METHOD
  o1 := items[0].mine * 100 + items[1].mine * 10 + items[2].mine;
  o2 := grid[1, -1].mine * 1000 + grid[1, 0].mine * 100 + grid[2, -1].mine * 10 + grid[2, 0].mine;
  o3 := seq;
  o4 := 0; o5 := 0;
  FOR k := 0 TO 2 DO
    o4 := o4 + plain[k].times;
    o5 := o5 + plain[k].cold;
  END_FOR
  o6 := d.cells[0].times + d.cells[1].times;
  FOR k := 0 TO 1 DO
    o6 := o6 + 10 * (nest[k][0].times + nest[k][1].times);
  END_FOR
  u(io := items[2]);
  o7 := u.o;
  o8 := u.Read(other := grid[2, 0]);
END_FUNCTION_BLOCK

PROGRAM main
  VAR h : Holder; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT; q4 AT %QW8 : INT;
      q5 AT %QW10 : INT; q6 AT %QW12 : INT; q7 AT %QW14 : INT; END_VAR
  h();
  q0 := h.o1; q1 := h.o2; q2 := h.o3; q3 := h.o4; q4 := h.o5; q5 := h.o6;
  q6 := h.o7; q7 := h.o8;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
