(* One of each Ed.2 core construct the Prokop examples do not use. *)
TYPE
    Mode : (Idle, Running, Fault) := Idle;
    Level : INT (0..100) := 50;
    Code : UINT (Ok := 0, Warn := 1, Bad := 2);
    Point : STRUCT
        x : REAL;
        y : REAL := 1.5;
    END_STRUCT;
    Row : ARRAY [1..3] OF INT := [1, 2, 3];
    Name : STRING[20] := 'none';
END_TYPE

FUNCTION Clamp : INT
VAR_INPUT
    v, lo, hi : INT;
END_VAR
IF v < lo THEN
    Clamp := lo;
ELSIF v > hi THEN
    Clamp := hi;
ELSE
    Clamp := v;
END_IF;
END_FUNCTION

FUNCTION_BLOCK Counter
VAR_INPUT
    pulse : BOOL R_EDGE;
    reset : BOOL;
END_VAR
VAR_OUTPUT
    count : DINT;
END_VAR
VAR_IN_OUT
    total : LINT;
END_VAR
VAR_TEMP
    inc : DINT := 1;
END_VAR
IF reset THEN
    count := 0;
    RETURN;
END_IF;
IF pulse THEN
    count := count + inc;
    total := total + 1;
END_IF;
END_FUNCTION_BLOCK

PROGRAM core
VAR
    mode : Mode;
    grid : ARRAY [0..2, 0..2] OF REAL := [9(0.0)];
    p : Point := (x := 1.0, y := 2.0);
    c : Counter;
    big : LINT;
    i, n : INT;
    w : WORD := 16#00FF;
    b : BYTE := 2#1010_0101;
    s : STRING := 'it$'s $$5$N';
    ws : WSTRING := "wide$0041";
    t : TIME := T#1h2m3s4ms;
    d : DATE := D#2026-11-05;
    tod_v : TOD := TOD#12:00:00.5;
    dt_v : DT := DT#2026-11-05-12:00:00;
    r : REAL := -1.5E-3;
    ok : BOOL;
    flag : BOOL := BOOL#TRUE;
    neg : INT := INT#-5;
END_VAR
VAR CONSTANT
    MAX_N : INT := 10;
END_VAR

CASE mode OF
    Idle:
        mode := Running;
    Running, Fault:
        ;
END_CASE;

CASE i OF
    1, 2..5: n := 1;
    -1: n := -1;
ELSE
    n := 0;
END_CASE;

FOR i := 0 TO 2 BY 1 DO
    grid[i, i] := 1.0;
    IF i = MAX_N THEN
        EXIT;
    END_IF;
END_FOR;

WHILE n < MAX_N DO
    n := n + 1;
    IF n MOD 2 = 0 THEN
        CONTINUE;
    END_IF;
END_WHILE;

REPEAT
    n := n - 1;
UNTIL n <= 0
END_REPEAT;

c(pulse := ok, reset := FALSE, count => big, total := big);
n := Clamp(v := n, lo := 0, hi := MAX_N);
n := Clamp(n, 0, MAX_N);
ok := NOT ok AND (n > 0 OR n < -2) XOR w.3;
r := -2.0 ** 2.0 + p.x * 3.0 / 2.0;
mode := Mode#Fault;
END_PROGRAM
