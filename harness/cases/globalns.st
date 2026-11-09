(* The global-namespace extension (docs/extensions.md): `_.` names the
   global namespace (Siemens, PLCnext; NAMESPACES.md D4). Inside Plant,
   ABS is Plant's own, x + 1000, and _.ABS the standard one; Lib.twice
   is Plant.Lib's, 3 x, and _.Lib.twice the top-level Lib's, 2 x.
   Worked by hand, tick 0, vin -4: q0 = -4 + 1000 = 996; q1 = 4;
   q2 = -12; q3 = -8. Tick 1, vin 7: q0 1007; q1 7; q2 21; q3 14.
   reference-skip: an extension, and the reference compiler has no
   namespaces. *)
NAMESPACE Lib
    FUNCTION twice : INT
    VAR_INPUT x : INT; END_VAR
        twice := x * 2;
    END_FUNCTION
END_NAMESPACE

NAMESPACE Plant
    NAMESPACE Lib
        FUNCTION twice : INT
        VAR_INPUT x : INT; END_VAR
            twice := x * 3;
        END_FUNCTION
    END_NAMESPACE

    FUNCTION ABS : INT
    VAR_INPUT x : INT; END_VAR
        ABS := x + 1000;
    END_FUNCTION

    PROGRAM Main
    VAR
        vin AT %IW0 : INT;
        q0 AT %QW0 : INT;
        q1 AT %QW2 : INT;
        q2 AT %QW4 : INT;
        q3 AT %QW6 : INT;
    END_VAR
        q0 := ABS(vin);
        q1 := _.ABS(vin);
        q2 := Lib.twice(vin);
        q3 := _.Lib.twice(vin);
    END_PROGRAM
END_NAMESPACE

CONFIGURATION Works
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : Plant.Main;
  END_RESOURCE
END_CONFIGURATION
