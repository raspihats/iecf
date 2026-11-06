(* The process image: each size of address, read and written; a located
   variable's initial value; %M as well as %Q. *)
PROGRAM main
VAR
  inp AT %IB0 : BYTE;
  bit AT %IX1.2 : BOOL;
  cnt AT %QW0 : INT;
  acc AT %QD2 : DINT := 100;
  flag AT %QX6.3 : BOOL;
  big AT %ML0 : LINT;
  w AT %MW8 : WORD := 16#ABCD;
END_VAR
cnt := cnt + BYTE_TO_INT(inp);
acc := acc - 7;
flag := bit;
big := big + LINT#4294967296;
w := ROL(w, 4);
END_PROGRAM

CONFIGURATION plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
