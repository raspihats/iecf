(* Single-level bit addresses and hierarchical addresses (Ed.3 Table
   16, 6.5.5.2). %IX9 is bit 9 of the input area, byte 1 bit 1; %I3 byte
   0 bit 3. A hierarchical address is where the platform's iecf_address
   puts it (docs/c-output.md §4); the harness's folds the levels into a
   byte from 256, each a hex digit, a bit address's last its bit:
   %IW2.5.7.1 is byte 256 + 0x2571 % 1024 = 625, %IX1.1.4 byte 273 bit
   4, %QX1.2.3 byte 274 bit 3, %QW3.3 byte 307, %QW7.7 byte 375, %MW1.1
   byte 273, %MW1.2.3 byte 547. Each written one is read back into the
   flat outputs. %IX1.1 (b9), %IX0.3 (b3), %IX273.4 TRUE, %IW625 41.
   Worked by hand:

   tick 0: out := iw + 1, 42 (Q0 2a 00); flags: b9, b3 and %IX1.1.4,
   2#111 (Q2 07); p, a structure at %MW1.1, p.y 2 (Q4 02 00); the
   block's w at %M*, put at %MW1.2.3 by VAR_CONFIG, 1 (Q10 01 00); %QW3.3 := iw, read back 41
   (Q6 29 00); s, at %Q* completed by %QW7.7 in VAR_CONFIG, 1 (Q8 01
   00); hb at %QX1.2.3 := b9, read back 1 (Q12 01).
   tick 1: p.y 4, w 2, s 2.
   reference-skip: the reference compiler takes one level. *)
TYPE Pt : STRUCT x : INT; y : INT; END_STRUCT; END_TYPE
FUNCTION_BLOCK blk
  VAR w AT %M* : UINT; END_VAR
  VAR_OUTPUT o : UINT; END_VAR
  w := w + 1;
  o := w;
END_FUNCTION_BLOCK
PROGRAM main
  VAR b9 AT %IX9 : BOOL; b3 AT %I3 : BOOL; END_VAR
  VAR iw AT %IW2.5.7.1 : INT; hb AT %QX1.2.3 : BOOL; p AT %MW1.1 : Pt; END_VAR
  VAR f : blk; END_VAR
  VAR out AT %QW0 : INT; flags AT %QB2 : BYTE; o2 AT %QW4 : INT; o3 AT %QW6 : INT; o4 AT %QW8 : INT;
      o5 AT %QW10 : UINT; o6 AT %QB12 : USINT; END_VAR
  VAR s AT %Q* : INT; END_VAR
  out := iw + 1;
  hb := b9;
  flags := BOOL_TO_BYTE(b9) OR SHL(BOOL_TO_BYTE(b3), 1) OR SHL(BOOL_TO_BYTE(%IX1.1.4), 2);
  p.y := p.y + 2;
  f();
  o2 := p.y;
  %QW3.3 := INT_TO_WORD(iw);
  o3 := WORD_TO_INT(%QW3.3);
  s := s + 1;
  o4 := s;
  o5 := f.o;
  o6 := BOOL_TO_USINT(hb);
END_PROGRAM
CONFIGURATION c
  RESOURCE r ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM pp WITH t : main;
  END_RESOURCE
  VAR_CONFIG r.pp.s AT %QW7.7 : INT; r.pp.f.w AT %MW1.2.3 : UINT; END_VAR
END_CONFIGURATION
