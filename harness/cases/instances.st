(* Function block and class instances besides a POU's own variable
   (Ed.3 6.6.3.3, 6.6.5.3, Table 11 rows 5a-7b, Figure 8 d and e): an
   ARRAY OF TON with [2(PT := T#20ms)], called as ts[i](...); an array of
   the user's CNT, each with its own initial value (an input, an output,
   a PUBLIC variable), called through an index from the input image and
   faulting INDEX when it is out of bounds; an array of a CLASS, its
   methods called on an element and an INTERFACE given one; a STRUCT with
   a TOF field whose PT the type gives (Table 11 7a), called as
   c.cooling(...); a FUNCTION_BLOCK holding an array of CNT; global
   instances, one named by a block's VAR_EXTERNAL, an array named by the
   program's; KEEPER, which keeps a RETAIN count, in a RETAIN array and a
   plain one, and in a plain global, across a warm restart; a RETAIN
   structure, its plain part kept too.
   Worked by hand, the inputs go %IX0.0, idx %IB1, ticks of 10 ms; USER
   calls the CNT it is given, so cs[idx] counts twice a tick:
   tick 0, go FALSE, idx 1: qtemp 16#11 (rc.temp 1, c.temp 1); ks[0] and
   kp[0] read 0, then count 1; b idle, sum 0; gc[0] 4; accs 11 and 11,
   qacc 11; cs[1] 10 (inc 5), cs[2] 50 (its n given), cs[3] 0.
   Tick 1, go TRUE, idx 2: ts[1] starts; c.cooling Q TRUE (qcool); qtemp
   16#22; qkr qkp 1; b counts cs 2 and 3, sum 5, gk 1; gc 8; accs 12, 23;
   cs[2] 52.
   Tick 2, idx 3: qtemp 16#33; qkr qkp 2; sum 10, gk 2; gc 12; accs 13,
   36; cs[3] 2.
   Tick 3, idx 1: ts[1] Q TRUE (ET 20 ms), ts[2] just started, Q FALSE
   (its PT 20 ms, not 0); qtemp 16#44; qkr qkp 3; sum 15, gk 3; gc 16;
   accs 14, 50; cs[1] 12, its lim (15 and 17 held there).
   Tick 4, go FALSE, idx 2: ts both FALSE; c.cooling still TRUE, PT not
   passed since IN fell; qtemp 16#55; qkr qkp 4; sum 15, gk 3; gc 20;
   accs 15, 65; cs[2] 54.
   A warm restart before tick 5: everything set anew but ks (RETAIN),
   rc (RETAIN, rc.temp 5 kept), and the RETAIN count of kp[0] and gk.
   Tick 5, idx 3: qcool FALSE (c anew); qtemp 16#61 (rc.temp 6, c.temp
   1); qkr 5 (ks[0].total kept), qkp 0 (kp[0].total anew); sum 0, gk.total
   0; gc 4; accs 11, 11; cs[1] 0, cs[2] 50, cs[3] 2.
   Tick 6, go TRUE, idx 4: qcool TRUE; qtemp 16#72; qkr qkp 6 (kp[0]'s
   count was kept); b counts 2 and 3, sum 5, gk 4 (its count kept); gc 8;
   accs 12, 23; then cs[4] given to u faults INDEX at line 158, u not
   called, qc1..qc3 as at tick 5.
   reference-skip: an array of instances, a CLASS, an INTERFACE and a
   PUBLIC variable are Ed.3's; the reference compiler (Ed.2) has none. *)
FUNCTION_BLOCK CNT
  VAR_INPUT inc : INT := 1; END_VAR
  VAR_OUTPUT n : INT; END_VAR
  VAR PUBLIC lim : INT := 100; END_VAR
  n := n + inc;
  IF n > lim THEN
    n := lim;
  END_IF;
END_FUNCTION_BLOCK

FUNCTION_BLOCK KEEPER
  VAR_OUTPUT total : INT; END_VAR
  VAR RETAIN kept : INT; END_VAR
  kept := kept + 1;
  total := kept;
END_FUNCTION_BLOCK

INTERFACE SHOW
  METHOD GET : INT
  END_METHOD
END_INTERFACE

CLASS ACC IMPLEMENTS SHOW
  VAR PUBLIC v : INT; END_VAR
  METHOD PUBLIC ADD
    VAR_INPUT x : INT; END_VAR
    v := v + x;
  END_METHOD
  METHOD PUBLIC GET : INT
    GET := v;
  END_METHOD
END_CLASS

TYPE COOLER :
  STRUCT
    temp : INT;
    cooling : TOF := (PT := T#20ms);
  END_STRUCT;
END_TYPE

FUNCTION_BLOCK USER
  VAR_IN_OUT c : CNT; END_VAR
  c();
END_FUNCTION_BLOCK

FUNCTION_BLOCK BANK
  VAR_INPUT go : BOOL; END_VAR
  VAR_OUTPUT sum : INT; END_VAR
  VAR cs : ARRAY[0..1] OF CNT := [(inc := 2), (inc := 3)]; END_VAR
  VAR_EXTERNAL gk : KEEPER; END_VAR
  IF go THEN
    cs[0]();
    cs[1]();
    gk();
  END_IF;
  sum := cs[0].n + cs[1].n;
END_FUNCTION_BLOCK

PROGRAM main
  VAR
    go AT %IX0.0 : BOOL;
    idx AT %IB1 : USINT;
    qt1 AT %QX0.0 : BOOL;
    qt2 AT %QX0.1 : BOOL;
    qcool AT %QX0.2 : BOOL;
    qc1 AT %QW2 : INT;
    qc2 AT %QW4 : INT;
    qc3 AT %QW6 : INT;
    qacc AT %QW8 : INT;
    qbank AT %QW10 : INT;
    qkr AT %QW12 : INT;
    qkp AT %QW14 : INT;
    qg AT %QW16 : INT;
    qgc AT %QW18 : INT;
    qtemp AT %QW20 : INT;
  END_VAR
  VAR
    ts : ARRAY[1..2] OF TON := [2(PT := T#20ms)];
    cs : ARRAY[1..3] OF CNT := [(inc := 5, lim := 12), (n := 50)];
    accs : ARRAY[0..1] OF ACC := [(v := 10)];
    c : COOLER;
    b : BANK;
    s : SHOW;
    u : USER;
    kp : ARRAY[0..1] OF KEEPER;
  END_VAR
  VAR RETAIN
    ks : ARRAY[0..1] OF KEEPER;
    rc : COOLER;
  END_VAR
  VAR_EXTERNAL
    gk : KEEPER;
    gc : ARRAY[0..1] OF CNT;
  END_VAR
  ts[1](IN := go);
  ts[2](IN := ts[1].Q);
  qt1 := ts[1].Q;
  qt2 := ts[2].Q;
  c.cooling(IN := go);
  qcool := c.cooling.Q;
  c.temp := c.temp + 1;
  rc.temp := rc.temp + 1;
  qtemp := rc.temp * 16 + c.temp;
  qkr := ks[0].total;
  qkp := kp[0].total;
  ks[0]();
  kp[0]();
  b(go := go);
  qbank := b.sum;
  qg := gk.total;
  gc[0]();
  qgc := gc[0].n;
  (* An index the generator does not fold: checked, as idx is. *)
  accs[idx - idx].ADD(x := 1);
  accs[1].ADD(x := accs[0].v);
  s := accs[1];
  qacc := s.GET();
  u(c := cs[idx]);
  cs[idx]();
  qc1 := cs[1].n;
  qc2 := cs[2].n;
  qc3 := cs[3].n;
END_PROGRAM

CONFIGURATION Plant
  VAR_GLOBAL
    gk : KEEPER;
    gc : ARRAY[0..1] OF CNT := [(inc := 4)];
  END_VAR
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
