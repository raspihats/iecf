(* __NEW and __DELETE (the dynamic-memory extension), from the task's
   pool of 4096 bytes. Worked by hand: each tick a block one INT longer
   is taken, the list copied into it, its last element (count + 1) * 10,
   the old list given back: %QB0 the count, %QB1 the sum / 10 - tick 0:
   1 and 1, tick 1: 2 and 3, tick 2: 3 and 6, tick 3: 4 and 10. A pair
   from __NEW has its initial value, x 5 (%QB2), and __DELETE sets pr to
   0 (%QB3 1); 100000 INTs fit no pool of 4096 bytes: 0 (%QB4 1). At
   tick 3 a copy of a block given back is read: IECF_FAULT_NULL on line
   47, %QB5 left at 7.
   reference-skip: __NEW and __DELETE are CODESYS's, not IEC 61131-3. *)
TYPE pair : STRUCT x : INT := 5; y : INT; END_STRUCT; END_TYPE
PROGRAM main
  VAR
    list, grown, big, stale, copy : POINTER TO INT;
    pr : POINTER TO pair;
    count, i, sum : INT;
  END_VAR
  VAR q0 AT %QB0 : SINT; q1 AT %QB1 : SINT; q2 AT %QB2 : SINT; q3 AT %QB3 : SINT; q4 AT %QB4 : SINT;
      q5 AT %QB5 : SINT; END_VAR
  grown := __NEW(INT, count + 1);
  FOR i := 0 TO count - 1 DO
    grown[i] := list[i];
  END_FOR;
  grown[count] := (count + 1) * 10;
  IF list <> 0 THEN
    __DELETE(list);
  END_IF;
  list := grown;
  count := count + 1;
  sum := 0;
  FOR i := 0 TO count - 1 DO
    sum := sum + list[i];
  END_FOR;
  q0 := INT_TO_SINT(count);
  q1 := INT_TO_SINT(sum / 10);
  pr := __NEW(pair);
  q2 := INT_TO_SINT(pr^.x);
  __DELETE(pr);
  q3 := BOOL_TO_SINT(pr = 0);
  big := __NEW(INT, 100000);
  q4 := BOOL_TO_SINT(big = 0);
  IF count = 4 THEN
    stale := __NEW(INT, 2);
    copy := stale;
    __DELETE(stale);
    q5 := 7;
    q5 := INT_TO_SINT(copy[0]);
  END_IF;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
