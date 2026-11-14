(* An ACTION whose body is an SFC (Ed.3 6.7.4.1; Annex A, Action : ...
   FB_Body). The chart in it evolves in the scans its action runs, the
   final scan too, and is held in the others: its steps stay, its BOOL
   action keeps its value, a step's T stands still; it is never reset
   (docs/dependencies.md). Its steps and actions are the POU's by name:
   its step i2 runs mark, an action of the outer chart.
   Outer: o0 runs sub for 20 ms, o1 waits 20 ms, and again; w0 runs
   watch every scan, after sub. In sub: i0 counts k to 2, i1 lights
   lamp and waits until i1.T is 30 ms, i2 stays.
   %QB0 the steps (o0 1, o1 2, i0 4, i1 8, i2 16), %QB1 k, %QB2 lamp,
   %QB3 how many of 10, 20, 30 ms i1.T has reached, %QB4 m.
   Worked by hand, a tick 10 ms:
   tick 0: o0, i0; cnt: k 1.                         05 01 00 00 00
   tick 1: k 2, so i0 to i1.                         09 02 00 00 00
   tick 2: cnt's final scan, k 3; lamp; i1.T 10 ms;
           o0.T 20 ms, so o0 to o1.                  09 03 01 01 00
   tick 3: sub's final scan: i1.T 20 ms.             0A 03 01 02 00
   tick 4: sub held: lamp and i1.T as they were;
           o1.T 20 ms, so o1 to o0.                  0A 03 01 02 00
   tick 5: sub again, where it was: i1.T 30 ms (the
           10 ms it was held not counted): i1 to i2. 11 03 01 03 00
   tick 6: lamp falls; i2 runs mark: m 1; o0 to o1.  11 03 00 03 01
   tick 7: sub's final scan; m 2.                    12 03 00 03 02
   reference-skip: the reference compiler takes no SFC in an action. *)
PROGRAM main
  VAR k, m : INT; lamp : BOOL; END_VAR
  VAR
    q0 AT %QB0 : USINT;
    q1 AT %QB1 : USINT;
    q2 AT %QB2 : USINT;
    q3 AT %QB3 : USINT;
    q4 AT %QB4 : USINT;
  END_VAR
  INITIAL_STEP o0 : sub(N); END_STEP
  TRANSITION FROM o0 TO o1 := o0.T >= T#20ms; END_TRANSITION
  STEP o1 : END_STEP
  TRANSITION FROM o1 TO o0 := o1.T >= T#20ms; END_TRANSITION
  INITIAL_STEP w0 : watch(N); END_STEP
  TRANSITION FROM w0 TO w0 := FALSE; END_TRANSITION
  ACTION sub :
    INITIAL_STEP i0 : cnt(N); END_STEP
    TRANSITION FROM i0 TO i1 := k >= 2; END_TRANSITION
    STEP i1 : lamp(N); END_STEP
    TRANSITION FROM i1 TO i2 := i1.T >= T#30ms; END_TRANSITION
    STEP i2 : mark(N); END_STEP
    TRANSITION FROM i2 TO i2 := FALSE; END_TRANSITION
    ACTION cnt : k := k + 1; END_ACTION
  END_ACTION
  ACTION mark : m := m + 1; END_ACTION
  ACTION watch :
    q0 := BOOL_TO_USINT(o0.X) + 2 * BOOL_TO_USINT(o1.X) + 4 * BOOL_TO_USINT(i0.X)
          + 8 * BOOL_TO_USINT(i1.X) + 16 * BOOL_TO_USINT(i2.X);
    q1 := INT_TO_USINT(k);
    q2 := BOOL_TO_USINT(lamp);
    q3 := BOOL_TO_USINT(i1.T >= T#10ms) + BOOL_TO_USINT(i1.T >= T#20ms) + BOOL_TO_USINT(i1.T >= T#30ms);
    q4 := INT_TO_USINT(m);
  END_ACTION
END_PROGRAM
CONFIGURATION c RESOURCE r ON PLC TASK t(INTERVAL := T#10ms, PRIORITY := 1); PROGRAM p WITH t : main; END_RESOURCE END_CONFIGURATION
