(* CODESYS's enumerations (the codesys-enums extension,
   docs/extensions.md): the base type after the values, `(...) DWORD :=
   black`, and a value named `COLOR.green`, as CODESYS and TwinCAT write
   them (their `qualified_only` makes every use so); the attributes
   before a TYPE are pragmas, which iecf reads past. Worked by hand:
   COLOR is DWORD, black its initial value: c is 16#88000000, its top
   byte 16#88 (%QB0); c := COLOR.green, 16#FF00FF00, top byte 16#FF
   (%QB1); c = COLOR.green TRUE (%QX4.0). Lib.Mode (stop, run, fault)
   starts as Lib.Mode.stop: m = Lib.Mode.stop, 1 at tick 0, then 0
   (%QB2); m := Lib.Mode.run; CASE m OF Lib.Mode.run: %QB3 := 7.
   reference-skip: CODESYS's writing is not IEC 61131-3. *)
{attribute 'qualified_only'}
{attribute 'strict'}
TYPE COLOR :
(
    white := 16#FFFFFF00,
    green := 16#FF00FF00,
    black := 16#88000000
) DWORD := black
;
END_TYPE

NAMESPACE Lib
  TYPE Mode : (stop, run, fault); END_TYPE
END_NAMESPACE

PROGRAM main
  VAR c : COLOR; m : Lib.Mode := Lib.Mode.stop; END_VAR
  VAR
    q0 AT %QB0 : BYTE;
    q1 AT %QB1 : BYTE;
    q2 AT %QB2 : SINT;
    q3 AT %QB3 : SINT;
    b0 AT %QX4.0 : BOOL;
  END_VAR
  c := COLOR#black;
  q0 := DWORD_TO_BYTE(SHR(c, 24));
  c := COLOR.green;
  q1 := DWORD_TO_BYTE(SHR(c, 24));
  b0 := c = COLOR.green;
  q2 := SEL(m = Lib.Mode.stop, SINT#0, SINT#1);
  m := Lib.Mode.run;
  CASE m OF
    Lib.Mode.stop: q3 := 1;
    Lib.Mode.run: q3 := 7;
  END_CASE;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
