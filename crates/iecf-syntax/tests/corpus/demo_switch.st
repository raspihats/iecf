(* demo_switch.st - Prokop's first spike (ARCHITECTURE.md §13): a real ST
 * program, compiled by a real IEC compiler, run through the real toolchain
 * pipeline into a module a real scan engine loads and executes.
 *
 * A seal-in latch plus a pass-through on located I/O (exercises
 * the toolchain's process-image binding), plus a VAR RETAIN counter and
 * totalizer (exercises the toolchain's bridge to spec/abi.md §5's retain
 * region, see toolchain/README.md).
 *)
PROGRAM main
VAR
    di0 AT %IX0.0 : BOOL;   (* set: latches dout0 on *)
    di1 AT %IX0.1 : BOOL;   (* set: latches dout0 off *)
    sp AT %IW0 : INT;       (* setpoint in *)
    dout0 AT %QX0.0 : BOOL;
    pv AT %QW0 : INT;       (* setpoint echoed out *)
END_VAR
VAR RETAIN
    scan_counter : DINT;    (* must survive an online slot swap *)
    totalizer : LREAL;      (* accumulates pv every scan di0 is set *)
END_VAR

IF di0 THEN
    dout0 := TRUE;
    totalizer := totalizer + INT_TO_LREAL(pv);
END_IF;
IF di1 THEN
    dout0 := FALSE;
END_IF;
pv := sp;
scan_counter := scan_counter + 1;

END_PROGRAM

CONFIGURATION Config0
    RESOURCE Res0 ON PLC
        TASK task0(INTERVAL := T#10ms, PRIORITY := 0);
        PROGRAM instance0 WITH task0 : main;
    END_RESOURCE
END_CONFIGURATION
