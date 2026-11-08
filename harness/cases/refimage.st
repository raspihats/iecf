(* References into the process image, to function blocks, and to
   references (Ed.3 6.4.4.10; docs/c-output.md §4): REF() of a variable
   at an address - a word, a BOOL at a bit, a BOOL taking a whole byte,
   an array, a structure, a STRING, an input - read and written through
   `^` as the variable itself is; REF() as an initial value; `r^ := ...`;
   an array of references and a reference to one, `rr^^`; a block, the
   user's and a standard one, called and read through a reference; a
   structure referencing its own type; `=` with REF(). At tick 5 %IW0 is
   99 and a NULL reference is dereferenced: a fault (IECF_FAULT_NULL), the
   statement stores nothing, and the task stops, as at tick 6.
   reference-skip: the reference compiler's C for `r^ := ...` does not
   compile, REF_TO a function block crashes it, and it has no REF() in an
   initial value or a comparison. *)
TYPE
  pair : STRUCT a : INT; b : BYTE; END_STRUCT;
  rint : REF_TO INT;
  node : STRUCT v : INT; next : REF_TO node; END_STRUCT;
END_TYPE

FUNCTION_BLOCK acc
VAR_INPUT x : INT; END_VAR
VAR_OUTPUT total : INT; END_VAR
total := total + x;
END_FUNCTION_BLOCK

PROGRAM main
VAR
  vin AT %IW0 : INT;
END_VAR
VAR
  w AT %MW0 : INT;
  bx AT %MX2.3 : BOOL;
  bb AT %MB3 : BOOL;
  la AT %MW4 : ARRAY[0..2] OF INT;
  ls AT %MW10 : pair;
  s AT %MB13 : STRING[3];
  o_tot AT %MW17 : INT;
  o_cnt AT %MW19 : INT;
  o_q AT %MX21.0 : BOOL;
  o_eq AT %MX21.1 : BOOL;
  o_node AT %MW22 : INT;
  o_pick AT %MW24 : INT;
  o_kept AT %MW26 : INT;
END_VAR
VAR
  rw : rint := REF(w);
  rin : REF_TO INT := REF(vin);
  rbx, rbb : REF_TO BOOL;
  rla : REF_TO ARRAY[0..2] OF INT;
  rls : REF_TO pair;
  rstr : REF_TO STRING[3];
  refs : ARRAY[0..2] OF rint;
  rr : REF_TO rint;
  k, kept : INT;
  f : acc;
  rf : REF_TO acc;
  c : CTU;
  rc : REF_TO CTU;
  n1, n2 : node;
  rn : REF_TO node;
END_VAR
k := vin MOD 3;
IF k < 0 THEN
  k := k + 3;
END_IF;
rw^ := rw^ + rin^;
rbx := REF(bx);
rbx^ := NOT rbx^;
rbb := REF(bb);
rbb^ := vin > 1;
rla := REF(la);
rla^[k] := rla^[k] + 10;
rls := REF(ls);
rls^.a := w;
rls^.b := INT_TO_BYTE(k);
rstr := REF(s);
IF k = 0 THEN
  rstr^ := 'ab';
ELSIF k = 1 THEN
  rstr^ := 'xyz';
ELSE
  rstr^ := '';
END_IF;
refs[0] := REF(la[0]);
refs[1] := REF(la[1]);
refs[2] := REF(la[2]);
rr := REF(refs[k]);
o_pick := rr^^;
rf := REF(f);
rf^(x := k + 1);
o_tot := rf^.total;
rc := REF(c);
rc^(CU := NOT rc^.CU, PV := 3);
o_cnt := rc^.CV;
o_q := rc^.Q;
o_eq := rw = REF(w);
n1.next := REF(n2);
n1.next^.v := n1.next^.v + k;
o_node := n2.v;
IF vin = 99 THEN
  rn := NULL;
ELSE
  rn := REF(n1);
END_IF;
kept := kept + 1;
o_kept := kept;
o_kept := rn^.v;
END_PROGRAM

CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM inst WITH t : main;
  END_RESOURCE
END_CONFIGURATION
