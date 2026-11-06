(* STRING (Ed.3 6.4.2, Table 34): variables with and without a length,
   initial values, assignment, comparison, and LEN, LEFT, RIGHT, MID,
   CONCAT, INSERT, DELETE, REPLACE, FIND, with lengths and positions
   from the inputs, inside the strings so both compilers define them
   (matiec reads no STRING[n] variable, so every one here is STRING). A
   string in a structure and in an array. The results are seen through
   LEN, FIND and comparisons. From tick 2 INSERT's P is past the end of
   its string: iecf appends and flags it (docs/dependencies.md), which
   matiec does not; the bytes are the same. *)
TYPE item : STRUCT
  tag : STRING := 'ab';
  n : INT;
END_STRUCT;
END_TYPE
PROGRAM main
VAR
  l AT %IW0 : INT;
  p AT %IW2 : INT;
  r_len AT %MW0 : INT;
  r_find AT %MW2 : INT;
  r_cmp AT %MW4 : WORD;
  r_mix AT %MW6 : INT;
END_VAR
VAR
  greeting : STRING := 'hello';
  message : STRING;
  short : STRING;
  names : ARRAY[1..2] OF STRING := ['x', 'y$'z'];
  it : item;
  part : STRING;
END_VAR
message := CONCAT(greeting, ' world', '!');
r_len := LEN(message);
r_find := FIND(message, 'wor');
short := LEFT(message, 4);
part := MID(message, l, p);
r_mix := LEN(part) * 100 + FIND(message, part);
r_cmp := 0;
IF message = 'hello world!' THEN r_cmp := r_cmp OR 1; END_IF;
IF 'apple' < greeting THEN r_cmp := r_cmp OR 2; END_IF;
IF short = 'hell' THEN r_cmp := r_cmp OR 4; END_IF;
IF names[2] = 'y$'z' THEN r_cmp := r_cmp OR 8; END_IF;
it.tag := names[1];
IF it.tag <> 'x' THEN r_cmp := r_cmp OR 16; END_IF;
IF RIGHT(message, l) = 'ld!' THEN r_cmp := r_cmp OR 32; END_IF;
IF INSERT(greeting, '--', p) = 'he--llo' THEN r_cmp := r_cmp OR 64; END_IF;
IF DELETE(message, l, p) = 'hello!' THEN r_cmp := r_cmp OR 128; END_IF;
IF REPLACE(greeting, 'LL', l, p) = 'heLLo' THEN r_cmp := r_cmp OR 256; END_IF;
IF GT(message, greeting) THEN r_cmp := r_cmp OR 512; END_IF;
IF part >= 'w' THEN r_cmp := r_cmp OR 1024; END_IF;
END_PROGRAM

CONFIGURATION config
  RESOURCE res ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
