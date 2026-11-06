(* The counters and bistables of Ed.3 Tables 43-45: CTU, CTD, CTUD, R_TRIG,
   F_TRIG, SR and RS.

   matiec's counters stop where Ed.3 Table 45 does not: CTU and CTUD count
   up only while CV < PV, and CTD down only while CV > 0, where the table
   has CV < PVmax and CV > PVmin (each implementer-specific). Four CU
   pulses against a PV of 3 show it: matiec's CTU stops at 3 and its CTUD
   at its loaded 2. iecf counts over the type's whole range, as decided
   (docs/dependencies.md): CTU to 4, CTUD to 6, and CTUD below 0 at the
   last CD. Its trace is edges.iecf.trace. *)
PROGRAM main
VAR
  cu AT %IX0.0 : BOOL;
  cd AT %IX0.1 : BOOL;
  rst AT %IX0.2 : BOOL;
  ld AT %IX0.3 : BOOL;
  s1 AT %IX0.4 : BOOL;
  r1 AT %IX0.5 : BOOL;
  up_q AT %QX0.0 : BOOL;
  dn_q AT %QX0.1 : BOOL;
  ud_qu AT %QX0.2 : BOOL;
  ud_qd AT %QX0.3 : BOOL;
  rise AT %QX0.4 : BOOL;
  fall AT %QX0.5 : BOOL;
  sr_q AT %QX0.6 : BOOL;
  rs_q AT %QX0.7 : BOOL;
  up_cv AT %MW0 : INT;
  dn_cv AT %MW2 : INT;
  ud_cv AT %MW4 : INT;
END_VAR
VAR
  c_up : CTU;
  c_dn : CTD;
  c_ud : CTUD;
  e_r : R_TRIG;
  e_f : F_TRIG;
  b_sr : SR;
  b_rs : RS;
END_VAR
c_up(CU := cu, R := rst, PV := 3);
c_dn(CD := cd, LD := ld, PV := 2);
c_ud(CU := cu, CD := cd, R := rst, LD := ld, PV := 2);
e_r(CLK := cu);
e_f(CLK := cu);
b_sr(S1 := s1, R := r1);
b_rs(S := s1, R1 := r1);
up_q := c_up.Q;
up_cv := c_up.CV;
dn_q := c_dn.Q;
dn_cv := c_dn.CV;
ud_qu := c_ud.QU;
ud_qd := c_ud.QD;
ud_cv := c_ud.CV;
rise := e_r.Q;
fall := e_f.Q;
sr_q := b_sr.Q1;
rs_q := b_rs.Q1;
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
