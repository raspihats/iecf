(* A FUNCTION's and a method's inputs, outputs, in-outs and results of a
   STRING, array or structure type (Ed.3 6.6.1.2, 6.6.2): LABEL's STRING
   result and STRING[3] input, given 'abcd' (cut to 'abc', flagged
   STRING_LENGTH where the input is declared) or left to its initial
   value 'n'; RAMP's array result, assigned and given to SUM3's array
   input; SHIFT's structure input and result, its structure and STRING
   outputs, and its STRING in-out, which it cuts at the fifth '+'; a
   method's STRING result through an INTERFACE, overridden (Ed.3
   6.6.5.5); WRAP's WSTRING input and result.
   Worked by hand, the inputs v %IB0, pick %IX1.0:
   tick 0, v 7: qlab 'abc7'; xs 7, 8, 9: qsum 24 + 6 = 30; SHIFT of p
   (1, 2) by its d 10: q (11, -8), was (1, 2), tag 'ok', trail '+' (qtl
   1); nm the ANIMAL: 'anim'; ws '<' 16#263A '>': qw 16#263A, qwl 3; qdef
   TRUE ('n1'). The flag from LABEL's input, line 33, stays.
   Tick 1, v 42: qlab 'abc42'; qsum 129 + 6 = 135; q (21, -18), was 11;
   qtl 2.
   Tick 2, v 255, pick TRUE: qlab 'abc255'; qsum 768 + 6 = 774; q (31,
   -28), was 21; qtl 3; nm the DOG: 'dog' (DOG's NAME, not ANIMAL's).
   Tick 3, v 0: qlab 'abc0', its sixth byte still '5' (a STRING in the
   image is its characters and a 0, the rest left); qsum 9; q (41, -38),
   was 31; qtl 4.
   Tick 4: q (51, -48), was 41; trail '+++++' cut to '++++', qtl 4.
   reference-skip: a method, an INTERFACE and WSTRING are Ed.3's; the
   reference compiler (Ed.2) has none. *)
TYPE PT :
  STRUCT
    x : INT;
    y : INT;
  END_STRUCT;
END_TYPE

FUNCTION LABEL : STRING[6]
  VAR_INPUT name : STRING[3] := 'n'; v : INT; END_VAR
  LABEL := CONCAT(name, INT_TO_STRING(v));
END_FUNCTION

FUNCTION SUM3 : INT
  VAR_INPUT xs : ARRAY[1..3] OF INT; END_VAR
  SUM3 := xs[1] + xs[2] + xs[3];
END_FUNCTION

FUNCTION RAMP : ARRAY[1..3] OF INT
  VAR_INPUT start : INT; END_VAR
  RAMP[1] := start;
  RAMP[2] := start + 1;
  RAMP[3] := start + 2;
END_FUNCTION

FUNCTION SHIFT : PT
  VAR_INPUT p : PT; d : INT := 10; END_VAR
  VAR_OUTPUT was : PT; tag : STRING[2]; END_VAR
  VAR_IN_OUT trail : STRING[4]; END_VAR
  was := p;
  SHIFT.x := p.x + d;
  SHIFT.y := p.y - d;
  tag := 'ok';
  trail := CONCAT(trail, '+');
END_FUNCTION

INTERFACE NAMED
  METHOD NAME : STRING[5]
  END_METHOD
END_INTERFACE

CLASS ANIMAL IMPLEMENTS NAMED
  METHOD PUBLIC NAME : STRING[5]
    NAME := 'anim';
  END_METHOD
END_CLASS

CLASS DOG EXTENDS ANIMAL
  METHOD PUBLIC OVERRIDE NAME : STRING[5]
    NAME := 'dog';
  END_METHOD
END_CLASS

FUNCTION WRAP : WSTRING[4]
  VAR_INPUT w : WSTRING[2]; END_VAR
  WRAP := CONCAT("<", w, ">");
END_FUNCTION

PROGRAM main
  VAR
    v AT %IB0 : USINT;
    pick AT %IX1.0 : BOOL;
    qlab AT %QB0 : STRING[6];
    qsum AT %QW8 : INT;
    qx AT %QW10 : INT;
    qy AT %QW12 : INT;
    qwx AT %QW14 : INT;
    qtag AT %QB16 : STRING[2];
    qtl AT %QB19 : USINT;
    qname AT %QB20 : STRING[5];
    qw AT %QW26 : WCHAR;
    qwl AT %QB28 : USINT;
    qdef AT %QX29.0 : BOOL;
  END_VAR
  VAR
    p : PT := (x := 1, y := 2);
    q, w : PT;
    xs : ARRAY[1..3] OF INT;
    trail : STRING[4];
    t : STRING[2];
    a : ANIMAL;
    d : DOG;
    nm : NAMED;
    ws : WSTRING[4];
  END_VAR
  qlab := LABEL(name := 'abcd', v := USINT_TO_INT(v));
  xs := RAMP(start := USINT_TO_INT(v));
  qsum := SUM3(xs := xs) + SUM3(xs := RAMP(start := 1));
  q := SHIFT(p := p, was => w, tag => t, trail := trail);
  p := q;
  qx := q.x;
  qy := q.y;
  qwx := w.x;
  qtag := t;
  qtl := INT_TO_USINT(LEN(trail));
  IF pick THEN
    nm := d;
  ELSE
    nm := a;
  END_IF;
  qname := nm.NAME();
  ws := WRAP(w := "${263A}");
  qw := ws[2];
  qwl := INT_TO_USINT(LEN(ws));
  qdef := LABEL(v := 1) = 'n1';
END_PROGRAM

CONFIGURATION Plant
  RESOURCE r ON PLC
    TASK fast(INTERVAL := T#10ms, PRIORITY := 1);
    PROGRAM p WITH fast : main;
  END_RESOURCE
END_CONFIGURATION
