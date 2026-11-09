(* A FUNCTION's outputs (Ed.3 6.6.1.4, 6.6.2.2): taken with =>, into a
   variable and a variable at an address; set from the call, an output's initial value at each call, as
   a FUNCTION has no memory; RETURN gives the outputs as they are.
   divmod(5, 0) returns before it sets q and r: q 0, r its initial -1.
   count's output starts at 0 at each call, so it always gives 1. An
   output into an array element is in fnoutx: the reference compiler's
   C for it does not compile. *)
FUNCTION divmod : INT
VAR_INPUT a, b : INT; END_VAR
VAR_OUTPUT q : INT; r : INT := -1; END_VAR
IF b = 0 THEN RETURN; END_IF;
q := a / b;
r := a MOD b;
divmod := 1;
END_FUNCTION

FUNCTION count : INT
VAR_OUTPUT n : INT; END_VAR
n := n + 1;
count := n;
END_FUNCTION

PROGRAM main
VAR
  vin AT %IW0 : INT;
  bin AT %IW2 : INT;
  qq AT %QW0 : INT;
  qr AT %QW2 : INT;
  qk AT %QW4 : INT;
  qn AT %QW6 : INT;
  qa AT %QW8 : INT;
END_VAR
VAR
  k, n, m : INT;
END_VAR
qk := divmod(a := vin, b := bin, q => qq, r => qr);
k := count(n => n);
qk := qk + k * 10;
qn := n;
k := divmod(a := 9, b := 4, r => m);
qa := m + k * 10;
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
