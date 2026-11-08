(* Arrays, structures and STRINGs at an address (Ed.3 6.5.5.3;
   docs/dependencies.md): from the address, their parts back to back
   with no gap, each its type's width, little-endian, a BOOL a byte, a
   STRING[n] n + 1 bytes. A structure `reading` is 11 bytes: ok at 0, raw
   at 1, at_ms at 3, label at 7. Elements read and written by computed
   index, whole arrays and structures copied in and out of the image,
   into and out of a function block, image to image, and a global at an
   address used through VAR_EXTERNAL. The initial value of `out` is
   written at the start. iecf only (reference-skip: the reference
   compiler refuses an array at an address, "Bit size of data type is
   incompatible with bit size of location", and a STRING[n] field). *)
TYPE reading :
  STRUCT
    ok : BOOL;
    raw : INT;
    at_ms : TIME;
    label : STRING[3];
  END_STRUCT;
END_TYPE

FUNCTION_BLOCK sum3
VAR_INPUT v : ARRAY[0..2] OF INT; END_VAR
VAR_OUTPUT total : INT; copy : ARRAY[0..2] OF INT; END_VAR
total := v[0] + v[1] + v[2];
copy := v;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  inp AT %IW0 : ARRAY[0..2] OF INT;
  out AT %QW0 : ARRAY[0..2] OF INT := [7, 8, 9];
  flags AT %QB6 : ARRAY[0..3] OF BOOL;
  r AT %MB0 : reading;
  hist AT %MB11 : ARRAY[0..1] OF reading;
  grid AT %MB33 : ARRAY[1..2, 1..3] OF SINT;
  back AT %MW39 : ARRAY[0..2] OF INT;
END_VAR
VAR
  f : sum3;
  n, n2 : INT;
  plain : ARRAY[0..2] OF INT;
END_VAR
VAR_EXTERNAL
  gp : reading;
END_VAR
n := n + 1;
f(v := inp);
out := f.copy;
out[n MOD 3] := f.total;
r.ok := NOT r.ok;
r.raw := f.total;
r.at_ms := T#1ms * n;
IF n = 2 THEN r.label := 'xyz'; ELSE r.label := 'ab'; END_IF;
grid[2, 3] := INT_TO_SINT(n);
grid[1, n MOD 3 + 1] := -1;
hist[n MOD 2] := r;
flags[n MOD 4] := NOT flags[n MOD 4];
plain := out;
f(v := plain, total => n2, copy => back);
gp := hist[0];
END_PROGRAM

CONFIGURATION plant
  VAR_GLOBAL
    gp AT %MB45 : reading;
  END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
