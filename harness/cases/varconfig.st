(* AT %I*, %Q*, %M* and VAR_CONFIG (Ed.3 6.5.5.4, 6.4.4.8, 6.8.2): each
   instance's variables where VAR_CONFIG puts them, and initial values
   given per instance, which prevail over the type's. Worked by hand,
   %IW0 2 and %IW2 1 throughout:

   init (cold): hw.out1 and hw2.out1 start at 200 (Q2, Q4: c8 00); lamp
   VAR_CONFIG's 9 (Q1 09) - a `*` address's initial value is given
   there, not where it is declared (Ed.3 Loc_Partly_Var); kept, RETAIN,
   VAR_CONFIG's 60 (M1 3c); t2.out 100 by VAR_CONFIG (Q11 64). Nothing else at a `*`
   address has an initial value: coils, count, arr, t1.out stay 0, and
   an input is the platform's. VAR_CONFIG sets fan.n 10, fan.top 12,
   hw.gain 9.

   tick 0: pump n 1 + 2 = 3, coil 3 > 3 FALSE; fan n 10 + 1 = 11, 11 >
   12 FALSE; st.inner (top 100) n 3, FALSE, st.count 1 (M0); h.v n 1 +
   1 = 2, FALSE: Q0 00. hw.out1 201 (c9 00); hw2 := hw copies the
   values from hw's addresses to hw2's: Q4 c9 00, gain 9. lamp 10, then
   Twice through an in-out: 20 (Q1 14). r := REF(arr[1]), r^ := 300
   (Q8 2c 01); arr[0] := hw2.gain, 9 (Q6 09 00). kept 61 (M1 3d). t1.out
   1, t2.out 101 (Q10 01, Q11 65).
   tick 1: pump n 5 > 3: coil TRUE (Q0 01); fan 12 > 12 FALSE; count 2;
   hw.out1 202 (ca 00), hw2 too; lamp 21, 42 (2a); kept 62; t1 2, t2
   102.
   warm, before tick 2: what is not RETAIN starts again - n 1, top 3,
   out1 200, gain 3 - then VAR_CONFIG's values but kept's, which
   the start keeps (62): fan.n 10, fan.top 12, hw.gain 9, lamp 9,
   t2.out 100. The coils, count, arr and t1.out have no initial value:
   the image keeps them.
   tick 2: pump n 3: coil FALSE (Q0 00); count 3; out1 201; lamp 10,
   20; kept 63 (3f); t1 3, t2 101 (65).
   reference-skip: the reference compiler parses VAR_CONFIG and
   generates nothing for it. *)
TYPE
  HWT : STRUCT
    in1 AT %I* : BOOL;
    out1 AT %Q* : UINT := 200;
    gain : INT := 3;
  END_STRUCT;
  Holder : STRUCT v : valve; END_STRUCT;
END_TYPE
FUNCTION_BLOCK valve
  VAR_INPUT top : INT := 3; END_VAR
  VAR coil AT %Q* : BOOL; level AT %I* : INT; END_VAR
  VAR n : INT := 1; END_VAR
  n := n + level;
  coil := n > top;
END_FUNCTION_BLOCK
FUNCTION_BLOCK station
  VAR inner : valve; END_VAR
  VAR count AT %M* : USINT; END_VAR
  inner(top := 100);
  count := count + 1;
END_FUNCTION_BLOCK
FUNCTION Twice : USINT
  VAR_IN_OUT x : USINT; END_VAR
  x := x * 2;
  Twice := x;
END_FUNCTION
PROGRAM line
  VAR pump, fan : valve; st : station; h : Holder; hw, hw2 : HWT; END_VAR
  VAR lamp AT %Q* : USINT; arr AT %Q* : ARRAY [0..1] OF INT; END_VAR
  VAR RETAIN kept AT %M* : USINT; END_VAR
  VAR r : REF_TO INT; k : USINT; END_VAR
  pump();
  fan();
  st();
  h.v();
  hw.out1 := hw.out1 + 1;
  hw2 := hw;
  lamp := lamp + 1;
  k := Twice(lamp);
  r := REF(arr[1]);
  r^ := 300;
  arr[0] := hw2.gain;
  kept := kept + 1;
END_PROGRAM
PROGRAM tiny
  VAR out AT %Q* : USINT; END_VAR
  out := out + 1;
END_PROGRAM
CONFIGURATION plant
  RESOURCE cpu ON PLC
    TASK fast (INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM main WITH fast : line;
    PROGRAM t1 WITH fast : tiny;
    PROGRAM t2 WITH fast : tiny;
  END_RESOURCE
  VAR_CONFIG
    cpu.main.pump.coil AT %QX0.0 : BOOL;
    cpu.main.pump.level AT %IW0 : INT;
    cpu.main.fan.coil AT %QX0.1 : BOOL;
    cpu.main.fan.level AT %IW2 : INT;
    cpu.main.fan.n : INT := 10;
    cpu.main.fan.top : INT := 12;
    cpu.main.st.inner.coil AT %QX0.2 : BOOL;
    cpu.main.st.inner.level AT %IW0 : INT;
    cpu.main.st.count AT %MB0 : USINT;
    cpu.main.h.v.coil AT %QX0.3 : BOOL;
    cpu.main.h.v.level AT %IW2 : INT;
    cpu.main.hw.in1 AT %IX4.0 : BOOL;
    cpu.main.hw.out1 AT %QW2 : UINT;
    cpu.main.hw.gain : INT := 9;
    cpu.main.hw2.in1 AT %IX4.1 : BOOL;
    cpu.main.hw2.out1 AT %QW4 : UINT;
    cpu.main.lamp AT %QB1 : USINT := 9;
    cpu.main.arr AT %QW6 : ARRAY [0..1] OF INT;
    cpu.main.kept AT %MB1 : USINT := 60;
    cpu.t1.out AT %QB10 : USINT;
    cpu.t2.out AT %QB11 : USINT := 100;
  END_VAR
END_CONFIGURATION
