# Implementer specific dependencies

IEC 61131-3 Ed.3 clause 5 d asks an implementation for "a document that
specifies all Implementer specific dependencies": the choices the
standard leaves open, and its limits. This is that document for iecf,
growing as the compiler does.

Each choice is made after checking what the big vendors do (CODESYS and
the systems built on it, Beckhoff TwinCAT, Siemens), and says so: a
program that compiles there should, as far as the standard lets it,
mean the same here.

## Untyped literals (Ed.3 6.6.1.6, rule 8)

"The Implementer shall define the rules for non-typed literals."

- An integer literal without a type (`5`, `16#FF`) takes the type the
  place it is used needs, if its value fits: any integer or bit-string
  type, and also REAL or LREAL. `SQRT(2)` is `SQRT(2.0)`; `r := 2` is
  `r := 2.0`. A value that does not fit is E0020.
- A real literal without a type (`2.5`) goes only where REAL or LREAL
  goes.
- Between two untyped literals, an operation keeps them untyped until a
  typed operand or the target decides.

Vendors: CODESYS documents `SQRT` as taking "any numeric base type" and
gives `q := SQRT(16)` as its example. It goes further and takes integer
*variables* too; iecf does not: an INT variable where ANY_REAL is
required is E0014 (convert it, `INT_TO_REAL`).

## LEN and FIND give INT (Ed.3 Table 34)

The standard gives their result as ANY_INT. iecf gives INT.

Vendors: INT in the CODESYS Standard library, in Beckhoff's
Tc2_Standard, and in Siemens S7-SCL (FC21, FC11). (CODESYS's separate
StrFunc library has a FIND of its own that gives UDINT.)

## DAY_OF_WEEK gives USINT (Ed.3 Table 36)

The standard gives the result as ANY_INT, 0 for Sunday to 6 for
Saturday, and leaves the type to the implementer (NOTE 2). iecf gives
USINT, the type of the standard's own example (`myDoW: USINT`).

Vendors: none of those checked has the standard's DAY_OF_WEEK; each has
a function of its own (Beckhoff `F_GetDayOfWeek` giving WORD, CODESYS
Util `DayOfWeek` giving an enumeration), so none sets a precedent.

## TRUNC without a type gives DINT (Ed.3 Table 22, 2a)

The "old" overloaded `TRUNC`, ANY_REAL to ANY_INT, is deprecated by
Ed.3, which keeps `TRUNC_INT` and `REAL_TRUNC_INT`, forms that name the
result. iecf reads it, warns (W0003), and gives DINT. Decided 2026-11-06
(florin).

Vendors: all three read it and give DINT. TwinCAT 3: "converts from the
data type REAL to the data type DINT"; TwinCAT 2 gave INT, and
`TRUNC_INT` stands in for that TRUNC when a TwinCAT 2 project is
imported. CODESYS V3: the same, `TRUNC_INT` standing in for V2.3's
TRUNC. Siemens S7-1200/1500: DINT unless the instruction's type is
changed. matiec gives whatever integer the target is, the standard's
ANY_INT read literally; no vendor does.

## The counters' PVmax and PVmin are their type's range (Ed.3 Table 45)

CTU and CTUD count up while `CV < PVmax`, CTD and CTUD down while
`CV > PVmin`, and the standard leaves both to the implementer. In iecf
they are the maximum and minimum of CV's type: a CTU (INT) stops at
32767, a CTD (INT) at -32768, a CTD_UDINT at 0. A counter never wraps.
Decided 2026-11-06 (florin).

Vendors: Siemens S7-1200/1500 counts "up to the range limit" of the
type, and on a signed type "down to the negative integer limit"
(S7-1200 System Manual), and stops there. CODESYS and TwinCAT give CV
and PV the type WORD; TwinCAT documents CTD stopping at 0, WORD's
minimum, and neither documents the top. Rockwell's CTUD wraps over.
matiec stops CTU at PV and CTD at 0, which the table does not say
(PROPOSAL.md §1b).

## An infinite or NaN real is flagged, not a fault (Ed.3 Table 10 e, f)

"Results of arithmetic instructions with denormalized values, infinity,
or not-a-number values are Implementer specific." In iecf they carry on
as IEEE 754 has them, and the first is flagged where the platform sees it
(docs/c-output.md §5); the task does not stop. Division by zero stays a
fault. Decided 2026-11-06 (florin).

Vendors: Siemens S7-1200 writes the infinity or NaN and sets ENO to
FALSE; the CPU goes on (S7-1200 System Manual, Tables 8-46, 8-67).
Rockwell Logix sets its overflow flag and a minor fault and goes on.
CODESYS, as an OEM's help documents it (ABB), carries infinities and
NaNs on without a signal. TwinCAT stops the PLC on an invalid operation
by default, a per-task "Floating Point Exception" setting, and warns
that turned off, a NaN can reach the outputs. iecf takes the middle: on,
but seen.

## An integer overflow wraps and is flagged (Ed.3 7.3.2, Table 23)

Ed.3 calls a result out of its type's range an error and leaves its
handling to the implementer; a conversion to a type that cannot hold
the value is implementer specific (Table 23). In iecf both carry on:
the result wraps, two's complement (`32767 + 1` is `-32768`), a
conversion keeps the value's low bits (`INT_TO_SINT(4223)` is 127), and
the first is flagged in `ctx->flag` with its POU and line, as an
infinite or NaN real is (docs/c-output.md §5). The task does not stop.
Division by zero stays a fault: no vendor agrees on a value to carry on
with. An index out of range will be a fault: carrying on would write
outside the array. Decided 2026-11-06 (florin), replacing the fault of
2026-11-05 (PROPOSAL.md §6.3).

A real converted to an integer it does not fit (an infinity and a NaN
among them) has no low bits to keep: it is flagged and its statement is
left undone - an assignment keeps its target's value (docs/c-output.md
§5). Decided 2026-11-06 (florin), and kept once Siemens's tables were
read: iecf's own choice, which invents no value.

Siemens S7-1200 (System Manual A5E02486680-AG, 03/2014, Table 7-75,
CONV): "Result exceeds valid range for OUT data type" gives ENO 0 and
"OUT is set to the IN value"; the 11/2011 edition (A5E02486680-05) says
"OUT is set to the least-significant bytes of IN" - iecf's rule for an
integer conversion. "IN is +/- INF or +/- NaN" gives ENO 0 and "+/- INF
or +/- NaN", which an integer OUT cannot hold. ROUND and TRUNC (Table
7-85) and CEIL and FLOOR (Table 7-88) list only the INF and NaN row. A
program execution error leaves the CPU in RUN (Table 4-14).

Vendors: none stops on an integer overflow. Siemens S7-1200 keeps the
least significant part, sets ENO to FALSE and stays in RUN, and the same
for a conversion out of range (S7-1200 System Manual, Table 8-46); a
division by zero gives 0. Rockwell Logix wraps, sets its overflow flag
S:V and a minor fault, and goes on; an overflow in an array subscript is
a major fault. CODESYS and TwinCAT wrap without a signal, and truncate
a conversion (TwinCAT: `INT_TO_SINT(4223)` is 127). TwinCAT stops the
task on an integer division by zero; on CODESYS it depends on the
target, and the optional CheckDiv functions replace a zero divisor with
1. iecf takes Siemens's and Rockwell's way: on, but seen.

## A subrange clamps a value outside it, and flags it (Ed.3 6.4.4.4)

A value stored into a subrange variable outside its bounds becomes the
bound it passed (`p := 150` into `INT (0..100)` stores 100), and the
first is flagged `IECF_FAULT_CONVERSION` in `ctx->flag` with its POU and
line (docs/c-output.md §5). It holds for every store: an assignment, an
array's element or a structure's field of the type, a function block's
input and an output taken into one. A FOR over a subrange starts at its
start clamped and stops at the bound it would pass; an end on the other
side is left, so the loop does not run, as written. A constant initial
value outside the subrange is an error. Decided 2026-11-06 (florin).

Vendors: CODESYS and TwinCAT do not check a subrange unless the project
adds the implicit-check functions CheckRangeSigned and
CheckRangeUnsigned: "If functions are not available, then the subrange
is not checked for the respective variables at runtime" (CODESYS help,
"POU CheckRangeSigned"; TwinCAT 3 InfoSys says the same). Their template
returns the bound passed, with no error (`i := 10*y` with y = 1000
gives 4095 for `INT (-4095..4095)`), and warns that a FOR counter of the
type running to the bound never ends; iecf's FOR does. Siemens and
Rockwell have no subrange types. matiec stores unchecked, except into a
function block's input, which it clamps. iecf clamps always, and is
seen.

## SPLIT_DATE, SPLIT_TOD, SPLIT_DT outputs (Ed.3 Table 36)

The outputs are ANY_INT and "the Implementer specifies the provided data
types" (NOTE 2). iecf takes any integer variable for each.

## The time types: ranges and resolution (Ed.3 6.4.2, Table 10)

The standard leaves their range and resolution to the implementer, and
allows negative durations (6.3.4, Table 8: `T#-14ms`). Decided
2026-11-05 (florin), after the vendors:

| Type | iecf | CODESYS / TwinCAT | Siemens S7-1200/1500 |
| --- | --- | --- | --- |
| TIME | signed 32-bit, ms: ±24d20h31m23s647ms | unsigned 32-bit, ms: 0 to 49 days | signed 32-bit, ms |
| LTIME | signed 64-bit, ns: ±292 years | 64-bit, ns | signed 64-bit, ns |
| DATE, DT | unsigned 32-bit, seconds since 1970-01-01 | the same | DATE: days since 1990 |
| TOD | unsigned 32-bit, ms since midnight | the same | the same |
| LDATE, LTOD, LDT | signed 64-bit, ns (since 1970, since midnight) | 64-bit, ns | LTOD, LDT: ns |

TIME is signed because the standard has negative durations; the unit is
every vendor's. A result outside the range faults (PROPOSAL.md §6.3).

## STRING and WSTRING (Ed.3 6.4.2, 6.3.3)

The maximum length is the implementer's. iecf follows CODESYS and
TwinCAT, decided 2026-11-05 (florin):

- `STRING` is `STRING(80)`; `STRING(n)` holds up to n characters in n+1
  bytes, the last a 0 after the characters. One byte per character, read
  as Latin-1 (ISO 8859-1), as CODESYS reads it by default. `'$00'` ends
  the string where it stands.
- `WSTRING` is `WSTRING(80)`; `WSTRING(n)` is n+1 16-bit units, UTF-16,
  0-terminated.
- The longest `n` is 65535.

Siemens differs: 254 characters by default, after a two-byte header of
maximum and current length.

`$N`, the newline, is a line feed, as matiec has it; a character a
STRING cannot hold, one above U+00FF, is refused (E0020), as is an
initial value longer than its STRING[n] (CODESYS error C0198; its
STRING page says such a literal is cut instead).

### When a string does not fit (Ed.3 6.6.2.5.11)

The standard calls three things errors and leaves their handling to the
implementer: a negative length or position, a character position that
does not exist, a result longer than the maximum. Decided 2026-11-06
(florin), after the vendors: **neither stops the task; both are
flagged** in `ctx->flag`, as an overflow is (PROPOSAL.md §6.3).

- **Too long**: cut to fit, `IECF_FAULT_STRING_LENGTH`. Every vendor
  cuts: CODESYS's CONCAT "will be silently truncated. No error is
  produced" (its Standard library, CONCAT); Siemens copies "the part of
  the IN string which can fit in the OUT string" (S7-1200 System Manual
  A5E02486680, 8.2.2) and sets ENO to 0 for a function's result. A
  function's result in iecf is never cut on its way (its buffer holds
  the most its inputs make, at most 65535), only where it is stored.
- **A length or position out of its string**: what Siemens gives, which
  is the one vendor that documents every case (the same manual, 8.2.4,
  its ENO tables), and `IECF_FAULT_STRING_POSITION`:

  | | Result |
  |---|---|
  | LEFT, RIGHT: L < 0 | empty |
  | LEFT, RIGHT: L past the end | all of IN |
  | MID: L < 0, P < 1, P past the end | empty |
  | MID: L past the end | from P to the end |
  | DELETE: L < 0, P < 1 | empty |
  | DELETE: P past the end | IN unchanged |
  | DELETE: L past the end | deleted to the end |
  | INSERT: P < 0 | empty |
  | INSERT: P past the end | IN2 appended |
  | REPLACE: L < 0, P < 1 | empty |
  | REPLACE: P past the end | IN2 appended |
  | REPLACE: L past the end | replaced to the end |

  A length of 0 takes nothing and is no error; INSERT at P = 0 puts IN2
  first, as CODESYS and Siemens do. FIND gives 0 when IN2 is not in IN1,
  and for an empty IN2. CODESYS documents only INSERT ("If POS is > 255
  OR < 0, the result equals STR1") and keeps a DELETE and REPLACE at
  P = 0 it calls not correct; TwinCAT's Tc2_Standard documents none.

### Conversions to and from STRING

The text is the implementer's. Decided 2026-11-06 (florin), after the
vendors; where none documents a case, it says so.

| To STRING | Text | Why |
|---|---|---|
| BOOL | `TRUE`, `FALSE` | TwinCAT: "the result is 'TRUE' or 'FALSE'" (tc3_plc_intro, Boolean conversion); CODESYS `BOOL_TO_STRING(TRUE)` is `'TRUE'` |
| integers | decimal, `-` only: `'-123456'` | Siemens S_CONV "do[es] not use a leading "+" sign"; CODESYS and TwinCAT silent |
| BYTE ... LWORD | decimal: `'255'` | no vendor documents it; CODESYS users report decimal (forum, unverified). matiec writes `16#ff` |
| REAL, LREAL | C's `%g`, 7 and 15 significant digits, correctly rounded, trailing zeros dropped: `'2.5'`, `'1'`, `'1.677722e+07'`, `'1e-05'`, `'inf'`, `'nan'` | CODESYS and TwinCAT: a REAL's "number of decimal places of the mantissa is limited to 6", "rounded to the 6th digit". matiec writes 10 digits (`'0.1000000015'`) |
| TIME, LTIME | the literal, lower-case units, those that are not 0: `'T#1d2h3m4s500ms'`, `'T#0ms'`, `'T#-1s500ms'`, `'LTIME#1s5us'` | TwinCAT `TIME_TO_STRING(T#12ms)` is `'T#12ms'`; CODESYS shows `'T#0MS'` |
| DATE, TOD, DT (with those types) | `'D#2026-10-03'`, `'TOD#12:34:56'`, `.123` when the milliseconds are not 0, `'DT#2026-10-03-12:34:56'` | TwinCAT: `'D#2002-08-18'`, `'TOD#14:01:05.123'`, `DT_TO_STRING(DT#1998-02-13-14:20)` is `'DT#1998-02-13-14:20:00'` |

From STRING, the text is read as CODESYS and TwinCAT read it: spaces
before it skipped ("Spaces before" are allowed, CODESYS STRING_TO), then
a literal of the type - a type prefix (`INT#5`), a sign, a based integer
(`16#FF`, `2#1111_1111`), underscores between digits, a REAL's fraction
and exponent, a duration with its `T#` or `LTIME#` (any case; a TIME
keeps whole milliseconds), `TRUE`/`FALSE` in any case or `1`/`0` - and
what follows ignored: `'123abc'` is 123 (CODESYS: "These are
truncated"; TwinCAT: `STRING_TO_WORD('34abc')` is 34). **No literal, or a
value its type cannot hold, gives 0 and is flagged**
`IECF_FAULT_CONVERSION`, as Siemens gives 0 with ENO = FALSE ("OUT is
set to 0 and ENO is set to FALSE", S_CONV); the task goes on. A REAL is
read exactly and rounded once, half to even, to its own 24 bits, not
twice through an LREAL.

Neither direction uses a C library: the digits come from big-integer
arithmetic in `iecf.h`, the same on every target (checked against
glibc's `%g` and `strtod`/`strtof` on millions of values, and on
wasm32).

## Directly represented variables: numbering and byte order (Ed.3 6.5.5)

The standard leaves the mapping of an address to memory to the
implementer. Decided 2026-11-05 (florin), after the vendors:

- **The number counts bytes.** `%IX3.5` is bit 5 of byte 3, `%IW3` bytes
  3 and 4, `%MD48` bytes 48 to 51; `%MW0` and `%MW1` overlap. As Beckhoff
  ("addressing is always bytewise") and Siemens; CODESYS counts in units
  of the size (`%MD48` is bytes 192 to 195).
- **Little-endian**: a word's first byte is its low byte, as CODESYS and
  TwinCAT on their usual targets, and EtherCAT; Siemens and PROFINET are
  big-endian.
- One-level addresses only for now; `%IX1.2.3` is refused.
