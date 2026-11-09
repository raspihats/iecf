(* Namespaces (Ed.3 6.9; NAMESPACES.md), over two files: this one and
   namespaces.d/lib.st, which both add to the namespace Lib (Table 65
   NOTE).
   - Plant's own TON hides the standard one inside Plant (6.9.2
     Example 2): its ET is an INT that counts the calls with IN TRUE.
   - Plant's own area hides the one Plant's USING Lib.Shapes brings in
     (D2): w + h.
   - Lib.twice written inside Plant is Plant.Lib.twice, 3 x: the first
     part of a dotted name is found from inside out (D3).
   - outside, at global level, reaches Lib.Shapes.area through this
     file's USING (D6), area itself calls the INTERNAL offset of its own
     namespace, and Lib.tenfold calls Detail.scale, in the INTERNAL
     namespace Lib.Detail, from inside Lib (Figure 29).
   - Corner, a type of Lib.Shapes, by its short name through USING.
   Worked by hand, tick 0, vin 2: qa = area 2 + 3 = 5; qb = 3 * 2 = 6;
   qc = t.ET 1; qd = outside(2) = (2 * 2 + 100) + 2 * 2 + 2 * 10 = 128;
   qe = 7, c being Corner#Round.
   Tick 1, vin 5: qa 8; qb 15; qc 2; qd = (10 + 100) + 10 + 50 = 170;
   qe 7.
   reference-skip: the reference compiler (Ed.2) has no namespaces. *)
USING Lib.Shapes;

NAMESPACE Lib
    FUNCTION twice : INT
    VAR_INPUT x : INT; END_VAR
        twice := x * 2;
    END_FUNCTION
END_NAMESPACE

FUNCTION outside : INT
VAR_INPUT x : INT; END_VAR
    outside := area(x, 2) + Lib.twice(x) + Lib.tenfold(x);
END_FUNCTION

NAMESPACE Plant
    USING Lib.Shapes;

    FUNCTION_BLOCK TON
    VAR_INPUT IN : BOOL; END_VAR
    VAR_OUTPUT Q : BOOL; ET : INT; END_VAR
        IF IN THEN
            ET := ET + 1;
        END_IF;
        Q := ET >= 2;
    END_FUNCTION_BLOCK

    FUNCTION area : INT
    VAR_INPUT w, h : INT; END_VAR
        area := w + h;
    END_FUNCTION

    NAMESPACE Lib
        FUNCTION twice : INT
        VAR_INPUT x : INT; END_VAR
            twice := x * 3;
        END_FUNCTION
    END_NAMESPACE

    PROGRAM Main
    VAR
        vin AT %IW0 : INT;
        qa AT %QW0 : INT;
        qb AT %QW2 : INT;
        qc AT %QW4 : INT;
        qd AT %QW6 : INT;
        qe AT %QW8 : INT;
    END_VAR
    VAR
        t : TON;
        c : Corner := Corner#Round;
    END_VAR
        qa := area(vin, 3);
        qb := Lib.twice(vin);
        t(IN := TRUE);
        qc := t.ET;
        qd := outside(vin);
        IF c = Corner#Round THEN
            qe := 7;
        END_IF;
    END_PROGRAM
END_NAMESPACE

CONFIGURATION Works
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : Plant.Main;
  END_RESOURCE
END_CONFIGURATION
