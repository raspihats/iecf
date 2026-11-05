(* demo_ld.st - a LADDER diagram through the full pipeline, exactly as the
 * graphical path produces it (spec/graphical-languages.md §2, §8 G4): the
 * POU below is byte-for-byte what the Workbench emits for the ladder in
 * workbench/src/lib/graphical/fbd-ladder.test.ts ("demo ladder parity"
 * guards the match), and the CONFIGURATION mirrors what the server
 * generates from prokop.json.
 *
 * LD is not a language matiec parses either - it is FBD plus three element
 * types (PLCopen says so literally: LD = commonObjects + fbdObjects +
 * ldObjects), compiled by the SAME graph->ST generator. What the rungs say:
 *
 *   |--| |--+--|/|--( )--    seal-in latch: start OR motor, AND NOT halt
 *   |  start|  halt  motor
 *   |--| |--+
 *   |  motor
 *
 *   |--|P|----------( )--    a rising-edge contact: one scan per press,
 *   |   jog         pulsed   via the R_TRIG the generator synthesized
 *
 * Note the rail contributes nothing to the expression (`AND(TRUE, x)` is
 * just `x`) and the edge trigger is called BEFORE the rungs: IEC's |P|
 * fires on an edge of the CONTACT's variable, never of the rung.
 *)
PROGRAM Latch
VAR
    start AT %IX0.0 : BOOL;
    halt AT %IX0.1 : BOOL;
    jog AT %IX0.2 : BOOL;
    motor AT %QX0.0 : BOOL;
    pulsed AT %QX0.1 : BOOL;
END_VAR
VAR
    RTRIG_C4 : R_TRIG;
END_VAR

RTRIG_C4(CLK := jog);
motor := AND(OR(start, motor), NOT(halt));
pulsed := RTRIG_C4.Q;

END_PROGRAM

CONFIGURATION Config0
    RESOURCE Res0 ON PLC
        TASK task0(INTERVAL := T#10ms, PRIORITY := 0);
        PROGRAM instance0 WITH task0 : Latch;
    END_RESOURCE
END_CONFIGURATION
