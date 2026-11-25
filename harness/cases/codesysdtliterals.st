(* A DT literal's seconds left out, and a fraction of its second
   (dt-literals): the seconds 0, the fraction dropped, W0016 - as
   TcOpen's TcoMessenger writes DT#1970-01-01-00:00 and its
   fbManipulatorAutomat DT#2013-05-09-13:29:20.500. Worked by hand,
   each as its seconds since 1970 (date-number-conversions):
   DT#1970-01-01-00:01: 60, 16#3C (%QD0).
   DT#2013-05-09-13:29:20.500: 2013-05-09 is day 15834, 1368057600 s;
   13:29:20 is 48560 s; 1368106160, 16#518BA4B0 (%QD4).
   An initial value DT#1970-01-02-00:00:00.999: one day, 86400, 16#15180
   (%QD8).
   reference-skip: not IEC 61131-3. *)
PROGRAM main
  VAR
    a : DT; b : DT; c : DT := DT#1970-01-02-00:00:00.999;
  END_VAR
  VAR q0 AT %QD0 : UDINT; q1 AT %QD4 : UDINT; q2 AT %QD8 : UDINT; END_VAR
  a := DT#1970-01-01-00:01;
  b := DT#2013-05-09-13:29:20.500;
  q0 := DT_TO_UDINT(a);
  q1 := DT_TO_UDINT(b);
  q2 := DT_TO_UDINT(c);
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
