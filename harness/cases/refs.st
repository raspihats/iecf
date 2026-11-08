(* References (Ed.3 6.4.4.10): REF_TO an elementary type, through a
   type of its own; to an array element by a computed index, a whole
   array, a structure; REF() and ^ to read, and to write a field or an
   element; = and <> between two references and with NULL; a reference
   as a function block's input, the block reading through it. Each tick
   %IW0 picks what r references: a, b, arr[2], or nothing.
   `r^ := ...` itself is refimage's: the reference compiler's C for it
   does not compile. *)
TYPE
  pair : STRUCT a : INT; b : INT; END_STRUCT;
  rint : REF_TO INT;
  quad : ARRAY[0..3] OF INT;
END_TYPE

FUNCTION_BLOCK bump
VAR_INPUT r : REF_TO INT; inc : INT; END_VAR
VAR_OUTPUT seen : INT; none : BOOL; END_VAR
none := r = NULL;
IF NOT none THEN
  seen := r^ + inc;
END_IF;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  v AT %IW0 : INT;
END_VAR
VAR
  o_a AT %MW0 : INT;
  o_b AT %MW2 : INT;
  o_arr AT %MW4 : INT;
  o_pa AT %MW6 : INT;
  o_pb AT %MW8 : INT;
  o_seen AT %MW10 : INT;
  o_same AT %MX12.0 : BOOL;
  o_null AT %MX12.1 : BOOL;
  o_none AT %MX12.2 : BOOL;
END_VAR
VAR
  a, b, k : INT;
  arr : quad;
  p : pair;
  r, ra : rint;
  rp : REF_TO pair;
  rarr : REF_TO quad;
  f : bump;
  same, isnull : BOOL;
END_VAR
k := v MOD 4;
IF k < 0 THEN
  k := k + 4;
END_IF;
CASE k OF
  0: r := REF(a);
  1: r := REF(b);
  2: r := REF(arr[k]);
ELSE
  r := NULL;
END_CASE;
isnull := r = NULL;
IF r <> NULL THEN
  a := r^ + a + 1;
END_IF;
ra := REF(a);
same := r = ra;
rp := REF(p);
rp^.a := rp^.a + k;
rp^.b := ra^;
rarr := REF(arr);
rarr^[k] := rarr^[k] + 2;
f(r := r, inc := 3);
o_a := a;
o_b := b;
o_arr := arr[2];
o_pa := p.a;
o_pb := p.b;
o_seen := f.seen;
o_same := same;
o_null := isnull;
o_none := f.none;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM inst WITH t : main;
  END_RESOURCE
END_CONFIGURATION
