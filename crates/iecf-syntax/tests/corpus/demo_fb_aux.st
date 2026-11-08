(* demo_fb_aux.st - exercises the aux region (spec/abi.md §5a,
 * Prokop's toolchain): a real TON timer's public interface members
 * (IN/PT/Q/ET) and a plain top-level PROGRAM-local variable, both
 * non-located and non-retained, so neither the process image nor the
 * retain region would expose them without the aux bridge.
 *
 * VAR blocks are deliberately split (FB instance declarations separate
 * from the AT-located one): mixing them in one VAR block is refused
 * (PROPOSAL.md §1e).
 *)
PROGRAM main
VAR
    fill_tmr : TON;
END_VAR
VAR
    start_pb AT %IX0.0 : BOOL;
END_VAR
VAR
    local_counter : INT;
END_VAR
fill_tmr(IN := start_pb, PT := T#20s);
local_counter := local_counter + 1;
END_PROGRAM

CONFIGURATION Config0
    RESOURCE Res0 ON PLC
        TASK task0(INTERVAL := T#10ms, PRIORITY := 0);
        PROGRAM instance0 WITH task0 : main;
    END_RESOURCE
END_CONFIGURATION
