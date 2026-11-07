(* Conversions to and from STRING where matiec's text and iecf's agree
   (docs/dependencies.md): integers in decimal, BOOL as TRUE and FALSE,
   a REAL whose shortest text is its %g, and reading them back - a
   number from the inputs there and back again. The rest of the text,
   which matiec writes otherwise (a REAL's 10 digits, 16#ff for a bit
   string, every unit of a TIME), is runtime/tests/strings.c's. *)
PROGRAM main
VAR
  a AT %IW0 : INT;
  b AT %ID2 : DINT;
  r_back AT %MW0 : INT;
  r_len AT %MW2 : INT;
  r_dint AT %MD4 : DINT;
  r_real AT %MD8 : REAL;
  r_cmp AT %MW12 : WORD;
END_VAR
VAR
  text : STRING;
END_VAR
text := INT_TO_STRING(a);
r_len := LEN(text);
r_back := STRING_TO_INT(text) / 2;
r_dint := STRING_TO_DINT(CONCAT(DINT_TO_STRING(b), '0'));
r_real := STRING_TO_REAL(REAL_TO_STRING(INT_TO_REAL(a) / 4.0)) * 2.0;
r_cmp := 0;
IF BOOL_TO_STRING(a > 0) = 'TRUE' THEN r_cmp := r_cmp OR 1; END_IF;
IF BOOL_TO_STRING(a > 0) = 'FALSE' THEN r_cmp := r_cmp OR 2; END_IF;
IF REAL_TO_STRING(INT_TO_REAL(a) / 4.0) = '2.5' THEN r_cmp := r_cmp OR 4; END_IF;
IF STRING_TO_BOOL('TRUE') THEN r_cmp := r_cmp OR 8; END_IF;
IF DINT_TO_STRING(b) = '-123456' THEN r_cmp := r_cmp OR 16; END_IF;
END_PROGRAM

CONFIGURATION config
  RESOURCE res ON PLC
    TASK t(INTERVAL := T#10ms, PRIORITY := 0);
    PROGRAM p WITH t : main;
  END_RESOURCE
END_CONFIGURATION
