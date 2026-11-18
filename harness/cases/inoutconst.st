(* CODESYS's VAR_IN_OUT CONSTANT (the in-out-constant extension,
   docs/extensions.md): passed by reference and not written by the POU,
   given any value of its type - a literal, a constant, an expression, a
   variable - and a STRING of any length, read whole whatever the length
   declared. In a FUNCTION by name and in order, in a FUNCTION_BLOCK.
   Worked by hand, w = %IW0 each tick (2, 12, -3), lim clamped into 0..10
   and flagged as an input of a subrange is:
   r1 = addk(7, 'ab', q, arr, 3) = 7 + 2 + (1 + 2) + (10 + 30) + 3 = 55;
   r2 = addk(2w, long, KP, arr, w) = 2w + 10 + 7 + 40 + lim(w):
        63, 91 (12 clamped to 10, flagged), 51 (-3 to 0, flagged);
   r3 = addk(K5, CONCAT(long, 'xyz'), q, arr, w + 9) = 5 + 13 + 3 + 40
        + lim(w + 9): 71 (11 to 10, flagged), 71 (21, flagged), 67;
   f(inc := w + 1, tag := long): total 3, 16, 14; n = LEN(tag) = 10, a
   STRING[2] given ten characters. The flag is tick 0's, r3's line: the
   first stays until the platform clears it, and the host does not.
   reference-skip: VAR_IN_OUT CONSTANT is not IEC 61131-3. *)
TYPE pt : STRUCT x : INT; y : INT; END_STRUCT; END_TYPE
TYPE small : INT (0..10); END_TYPE

FUNCTION addk : INT
VAR_IN_OUT CONSTANT
  k : INT;
  s : STRING[4];
  p : pt;
  a : ARRAY[0..2] OF INT;
  lim : small;
END_VAR
addk := k + LEN(s) + p.x + p.y + a[0] + a[2] + lim;
END_FUNCTION

FUNCTION_BLOCK acc
VAR_IN_OUT CONSTANT inc : INT; tag : STRING[2]; END_VAR
VAR_OUTPUT total : INT; n : INT; END_VAR
total := total + inc;
n := LEN(tag);
END_FUNCTION_BLOCK

PROGRAM main
VAR
  v AT %IW0 : INT;
  r1 AT %QW0 : INT;
  r2 AT %QW2 : INT;
  r3 AT %QW4 : INT;
  r4 AT %QW6 : INT;
  r5 AT %QW8 : INT;
END_VAR
VAR
  long : STRING[20] := 'abcdefghij';
  q : pt := (x := 1, y := 2);
  arr : ARRAY[0..2] OF INT := [10, 20, 30];
  f : acc;
  w : INT;
END_VAR
VAR CONSTANT K5 : INT := 5; KP : pt := (x := 3, y := 4); END_VAR
w := v;
r1 := addk(k := 7, s := 'ab', p := q, a := arr, lim := 3);
r2 := addk(k := w * 2, s := long, p := KP, a := arr, lim := w);
r3 := addk(K5, CONCAT(long, 'xyz'), q, arr, w + 9);
f(inc := w + 1, tag := long);
r4 := f.total;
r5 := f.n;
END_PROGRAM

CONFIGURATION plant
  RESOURCE cpu ON PLC
    TASK t (INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
