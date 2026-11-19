(* Three CODESYS ways with integers, each an extension. Worked by hand:
   integer-shifts: i := -8. SHR(i, 1) keeps the sign, -4 (%QW0);
   SHR(i, 20), past INT's 16 bits, -1 (%QW2); SHL(i, 2), -32 (%QW4);
   ROL(INT#-32768, 1), the top bit round to the bottom, 1 (%QW6);
   SHR(u, n) with UINT u 40000 and BYTE n 3, 5000 (%QW8).
   integer-bit-access: d := -2 (16#FFFFFFFE): d.0 FALSE, d.1 TRUE
   (%QX10.0, %QX10.1); d.0 := TRUE makes it -1 (%QD12); k := 5, k.%X15 :=
   TRUE sets INT's sign bit, -32763 (%QW16).
   integer-expt: EXPT(2, n) with INT n 10, 1024.0 as REAL, 1024 (%QW18);
   EXPT(q, 3) with DINT q -3, an LREAL, -27.0, -27 (%QW20).
   reference-skip: none of the three is IEC 61131-3. *)
PROGRAM main
  VAR
    i : INT := -8; u : UINT := 40000; n : BYTE := 3; d : DINT := -2; k : INT := 5;
    e : INT := 10; base : INT := 2; q : DINT := -3;
  END_VAR
  VAR
    q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT; q4 AT %QW8 : UINT;
    b0 AT %QX10.0 : BOOL; b1 AT %QX10.1 : BOOL; q5 AT %QD12 : DINT; q6 AT %QW16 : INT;
    q7 AT %QW18 : INT; q8 AT %QW20 : INT;
  END_VAR
  q0 := SHR(i, 1);
  q1 := SHR(i, 20);
  q2 := SHL(i, 2);
  q3 := ROL(INT#-32768, 1);
  q4 := SHR(u, n);
  b0 := d.0;
  b1 := d.1;
  d.0 := TRUE;
  q5 := d;
  k.%X15 := TRUE;
  q6 := k;
  q7 := REAL_TO_INT(EXPT(base, e));
  q8 := LREAL_TO_INT(EXPT(q, 3));
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
