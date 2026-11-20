(* A method using its block's VAR_IN_OUT (method-in-outs), as TcOpen's
   do, CODESYS warning C0371: what the block's last call bound.
   Worked by hand:
   %QW0, %QW2: d1 := 5, d2 := 6; ax(drive := d1), ax.Stop() sets d1 to 0;
   ax(drive := d2), ax.Stop() sets d2 to 0: 0 and 0.
   %QW4: p.speed through the structure in-out: Stop writes 7.
   %QW6: set to 1 before bx.Stop(), bx never called: NULL fault on the
   line of bx.Stop() (34), the task stops there; q3 stays 1.
   reference-skip: not IEC 61131-3. *)
TYPE Info : STRUCT speed : INT; END_STRUCT; END_TYPE

FUNCTION_BLOCK Axis
  VAR_IN_OUT drive : INT; data : Info; END_VAR
  METHOD PUBLIC Stop
    drive := 0;
    data.speed := 7;
  END_METHOD
  drive := drive;
END_FUNCTION_BLOCK

PROGRAM main
  VAR ax : Axis; bx : Axis; d1 : INT := 5; d2 : INT := 6; p : Info; END_VAR
  VAR q0 AT %QW0 : INT; q1 AT %QW2 : INT; q2 AT %QW4 : INT; q3 AT %QW6 : INT; END_VAR
  ax(drive := d1, data := p);
  ax.Stop();
  ax(drive := d2, data := p);
  ax.Stop();
  q0 := d1;
  q1 := d2;
  q2 := p.speed;
  q3 := 1;


  bx.Stop();
  q3 := 2;
END_PROGRAM

CONFIGURATION cf
  RESOURCE r ON x
    TASK tk(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH tk : main;
  END_RESOURCE
END_CONFIGURATION
