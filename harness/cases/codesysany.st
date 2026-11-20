(* ANY as an input (any-input), as CODESYS passes one and TcOpen's
   StringBuilder reads it: its type class, a pointer to the value, its
   size. Worked by hand:
   %QW0 Kind(i), an INT 7: read through the pointer, 7 + 1000 = 1007;
   %QW2 Kind(r), a REAL: 2; %QW4 Kind(5), a literal, an INT: 1005;
   %QW6 b.Add(i) adds through its pointer: i 7 -> 8, b's own total 8;
   %QW8 Kind(s), a STRING(20): neither, its size, 21 bytes.
   %QW10 a block's input: Sz(Data := r), a REAL's 4 bytes.
   cycle 1: i 8: 1008, 2, 1005, total 8 + 9 = 17 (i 9), 21, 4.
   reference-skip: not IEC 61131-3. *)
FUNCTION Kind : INT
VAR_INPUT Data : ANY; END_VAR
VAR p : POINTER TO INT; END_VAR
IF Data.TypeClass = __SYSTEM.TYPE_CLASS.TYPE_INT THEN
  p := Data.pValue;
  Kind := p^ + 1000;
ELSIF Data.TypeClass = __SYSTEM.TYPE_CLASS.TYPE_REAL THEN
  Kind := 2;
ELSE
  Kind := DINT_TO_INT(Data.diSize);
END_IF
END_FUNCTION

FUNCTION_BLOCK Acc
VAR_OUTPUT total : INT; END_VAR
METHOD PUBLIC Add : INT
VAR_INPUT Data : ANY; END_VAR
VAR p : POINTER TO INT; c : __SYSTEM.TYPE_CLASS; END_VAR
c := Data.TypeClass;
IF c = __SYSTEM.TYPE_CLASS.TYPE_INT THEN
  p := Data.pValue;
  p^ := p^ + 1;
  total := total + p^;
END_IF
Add := total;
END_METHOD
END_FUNCTION_BLOCK

FUNCTION_BLOCK Sz
VAR_INPUT Data : ANY; END_VAR
VAR_OUTPUT n : INT; END_VAR
n := DINT_TO_INT(Data.diSize);
END_FUNCTION_BLOCK

PROGRAM main
  VAR i : INT := 7; r : REAL; s : STRING(20); b : Acc; t : INT; z4 : Sz; END_VAR
  VAR x AT %QW0 : INT; y AT %QW2 : INT; z AT %QW4 : INT; w AT %QW6 : INT; v AT %QW8 : INT; u AT %QW10 : INT; END_VAR
  x := Kind(i);
  y := Kind(Data := r);
  z := Kind(5);
  t := b.Add(i);
  w := b.total;
  v := Kind(s);
  z4(Data := r);
  u := z4.n;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
