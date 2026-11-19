(* Found in OSCAT NETWORK (IP_CONTROL2): JMP and labels in ST
   (jump-labels). Worked by hand:
   %QW0: i counted to 5 by a jump back: 5.
   %QW2: JMP out of the FOR at k = 3, k left as the body has it: 3.
   %QW4: n = 1 + 2 + 3: 6; the jumped-over q2 := 99 never runs.
   %QW6: arr[k] with k 2 holds; k 7 on the jump back is outside 0..3:
   INDEX fault on the line of arr[k] := 1 (32), the task stops: q3 1.
   reference-skip: not IEC 61131-3. *)
PROGRAM main
  VAR i : INT; k : INT; n : INT; arr : ARRAY[0..3] OF INT; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT; END_VAR
  i := 0;
  n := 0;
again:
  i := i + 1;
  IF i < 5 THEN
    JMP again;
  END_IF;
  q0 := i;
  JMP skip;
  q2 := 99;
skip:
  FOR k := 1 TO 10 DO
    n := n + k;
    IF k = 3 THEN JMP out; END_IF;
  END_FOR;
out:
  q1 := k;
  q2 := n;
  k := 2;
back:
  arr[k] := 1;
  k := k + 5;
  q3 := q3 + 1;
  IF k < 20 THEN JMP back; END_IF;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
