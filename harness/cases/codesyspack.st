(* The codesys pack (docs/extensions.md, "Packs"): one name for the
   extensions code written for CODESYS and TwinCAT uses, seven of them
   here. Worked by hand:
   gCount, a top-level global, used without VAR_EXTERNAL
   (implicit-externals) and bound to a REFERENCE TO (reference-to): r :=
   r + 1 makes it 6, then 7 (%QB3). t T#1500ms; ms := TIME_TO_DINT(t),
   1500 (time-number-conversions), worked out as it is set up
   (initial-expressions): ms / 100, 15 (%QB0). Len3 takes a VAR_IN_OUT
   CONSTANT STRING(10) (in-out-constant, string-parentheses), given 16
   characters, all of them read: n := Len3('abcdefghijklmnop') + ms,
   1516; n - 1500, 16 (%QB1). m : E_Mode := E_Mode.eRun, its base type
   after its values (codesys-enums): m = E_Mode.eRun, 1 (%QB2).
   BYTE_TO_BOOL(16#10) TRUE (bit-string-to-bool, %QX4.0).
   reference-skip: CODESYS's extensions are not IEC 61131-3. *)
VAR_GLOBAL gCount : INT := 5; END_VAR

TYPE E_Mode : (eIdle, eRun) DWORD; END_TYPE

FUNCTION Len3 : DINT
  VAR_IN_OUT CONSTANT s : STRING(10); END_VAR
  Len3 := INT_TO_DINT(LEN(s));
END_FUNCTION

PROGRAM main
  VAR
    t : TIME := T#1500ms;
    ms : DINT := TIME_TO_DINT(t);
    n : DINT := Len3('abcdefghijklmnop') + ms;
    m : E_Mode := E_Mode.eRun;
    r : REFERENCE TO INT;
    b : BOOL := BYTE_TO_BOOL(16#10);
  END_VAR
  VAR
    q0 AT %QB0 : SINT;
    q1 AT %QB1 : SINT;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    b0 AT %QX4.0 : BOOL;
  END_VAR
  r REF= gCount;
  r := r + 1;
  q0 := DINT_TO_SINT(ms / 100);
  q1 := DINT_TO_SINT(n - 1500);
  q2 := SEL(m = E_Mode.eRun, SINT#0, SINT#1);
  q3 := INT_TO_SINT(gCount);
  b0 := b;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
