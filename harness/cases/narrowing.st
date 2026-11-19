(* implicit-narrowing: an integer or bit string where another is wanted,
   converted as its *_TO_* would, W0010 at each. Worked by hand:
   i := d, DINT 100000 (16#186A0) into an INT: its low 16 bits, 16#86A0,
   -31072, flagged (%QW0).
   w := m, INT -2 into a WORD: its bits, 16#FFFE, unflagged (%QW2).
   u := m, INT -2 into a UINT: low bits, 65534, flagged (%QW4).
   g(i), the INT -31072 passed to a SINT input: low 8 bits 16#A0, -96,
   flagged (%QB6).
   q4 := b, BYTE 200 into an INT: 200, which it holds (%QW8).
   The first flag is i := d's, line 28.
   reference-skip: implicit narrowing is CODESYS's, not IEC 61131-3. *)
FUNCTION g : SINT
  VAR_INPUT x : SINT; END_VAR
  g := x;
END_FUNCTION

PROGRAM main
  VAR
    i : INT; d : DINT := 100000; w : WORD; u : UINT; m : INT := -2; b : BYTE := 200;
  END_VAR
  VAR
    q0 AT %QW0 : INT;
    q1 AT %QW2 : WORD;
    q2 AT %QW4 : UINT;
    q3 AT %QB6 : SINT;
    q4 AT %QW8 : INT;
  END_VAR
  i := d;
  q0 := i;
  w := m;
  q1 := w;
  u := m;
  q2 := u;
  q3 := g(i);
  q4 := b;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
