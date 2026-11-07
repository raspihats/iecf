(* A FUNCTION_BLOCK's inputs and outputs of an array, structure or
   STRING type: copied in by the call, copied out by `=>`, read from
   outside as f.hist[0] and f.text. Each tick the input %IW0 feeds an
   array and a structure; the block keeps a history of its sums. *)
TYPE
  pair : STRUCT a : INT; b : INT; END_STRUCT;
  three : ARRAY[1..3] OF INT;
  last3 : ARRAY[0..2] OF INT;
END_TYPE

FUNCTION_BLOCK acc
VAR_INPUT
  xs : three;
  p : pair;
  tag : STRING;
END_VAR
VAR_OUTPUT
  hist : last3;
  sum : INT;
  q : pair;
  text : STRING;
END_VAR
sum := xs[1] + xs[2] + xs[3];
hist[0] := hist[1];
hist[1] := hist[2];
hist[2] := sum;
q.a := p.b;
q.b := p.a;
text := CONCAT(tag, '!');
END_FUNCTION_BLOCK

PROGRAM main
VAR
  v AT %IW0 : INT;
  r_sum AT %MW0 : INT;
  r_h2 AT %MW2 : INT;
  r_h0 AT %MW4 : INT;
  r_qa AT %MW6 : INT;
  r_qb AT %MW8 : INT;
  r_len AT %MW10 : INT;
  r_same AT %MX12.0 : BOOL;
END_VAR
VAR
  xs : three;
  pr : pair;
  h : last3;
  qq : pair;
  lab : STRING;
  f : acc;
END_VAR
xs[1] := v;
xs[2] := v * 2;
xs[3] := 1;
pr.a := v;
pr.b := 5;
f(xs := xs, p := pr, tag := 'ab', hist => h, q => qq, text => lab);
r_sum := f.sum;
r_h2 := h[2];
r_h0 := f.hist[0];
r_qa := qq.a;
r_qb := qq.b;
r_len := LEN(lab);
r_same := f.text = lab;
END_PROGRAM

CONFIGURATION plant
  RESOURCE res ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
