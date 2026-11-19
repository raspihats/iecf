# Implementer extensions

IEC 61131-3 Ed.3 clause 5 c asks an implementation for "a document that
specifies all Implementer specific extensions. These are any features
accepted by the system that are prohibited or not specified." This is
that document for iecf; what it implements of the standard is its
[compliance statement](compliance.md).

By default iecf accepts none: it checks the standard as written. Each
extension is switched on by name, for one run:

```
iecf check --extension implicit-externals FILE...
```

A program that needs an extension is not portable to another compliant
tool as it stands; the section of each extension says what to change to
make it so.

## What is an extension

Each construct is placed by three questions, asked in order (decided
2026-11-18, florin):

1. **Does Ed.3 define it - in its text, a feature table or its grammar
   - and say what it does?** Then it is iecf's, always on, and does
   what the text says. Where iecf would do otherwise, that is a bug or
   an extension, never the default.
2. **Does Ed.3 define it and leave what it does, a limit or a type to
   the implementer** ("Implementer specific", "allowed, but not
   required", "the Implementer shall define")? Then it is iecf's too,
   always on, and the choice is written in
   [Implementer choices](dependencies.md) (clause 5 d).
3. **Does Ed.3 not define it, or forbid it?** Then it is an extension:
   off unless named, described here with how to make a program portable
   without it. So are the conversions Figure 11 marks "-", which its key
   says an implementation "may support" as "Implementer specific data
   type conversions": the standard lets them be, and does not define
   them.

Where Ed.3's grammar and its text disagree, the text decides (the
repeated list of an array's initial value,
[Implementer choices](dependencies.md#a-list-repeated-in-an-arrays-initial-value-ed3-64452)).
What Ed.3 says a program "shall not" do is an error, where the checker
can see it (clause 5 e, [Diagnostics](errors.md)). An extension may give
a standard name a meaning the standard does not, as `bit-string-to-bool`
does `BYTE_TO_BOOL`: clause 5 f forbids that of a compliant system, so a
run with such an extension is not one.

## Packs

Code written for one family of tools uses many of its extensions at
once. A pack switches several on by one name, given wherever an
extension's is: `--extension codesys` on the command line, an
`extension codesys` line in a harness case, `iecf_extension` in the
WebAssembly build (decided 2026-11-18, florin: "a flag that defines an
extension pack").

| pack | for | extensions |
|---|---|---|
| `codesys` | what code written for CODESYS and TwinCAT uses | `implicit-externals`, `var-config-list`, `time-number-conversions`, `bit-string-to-bool`, `reference-to`, `string-parentheses`, `temp-instances`, `in-out-constant`, `initial-expressions`, `codesys-enums`, `struct-semicolon`, `real-without-point`, `bare-array-init`, `input-constant`, `keyword-names` |

`codesys` is each extension that is CODESYS's and TwinCAT's own. Those
of other tools stay out - `global-namespace` (Siemens, PLCnext),
`endian-dates` (PLCnext, logi.CAD 3) - and are named beside it where
wanted: `--extension codesys --extension global-namespace`. A program
built with a pack is not IEC 61131-3, as with any of its extensions.
The harness case `codesyspack` builds with it.

## implicit-externals: a POU uses a global variable without declaring it VAR_EXTERNAL

**The standard.** Ed.3 6.5.2.2: global variables are "only accessible
to a program organization unit via a VAR_EXTERNAL declaration", with
the global's type, and `CONSTANT` if the global is. Without the
extension, a global used undeclared is E0010, and a global constant
named in a POU's array bound, STRING length or subrange limit E0032 (a `TYPE`'s sees
the global constants directly; until 2026-11-19 a POU's did too).

**The extension.** A name a POU does not declare, which a `VAR_GLOBAL`
of the configuration, of a resource or of a global variable list does,
is that global, as if the POU had declared it
`VAR_EXTERNAL` (`VAR_EXTERNAL CONSTANT` for a constant global). The
POU's own declarations come first: a local of the same name hides the
global. Everything else about the global is checked as usual: its
type, and that a constant is not assigned to. It is compiled as the
VAR_EXTERNAL would be, in any PROGRAM, FUNCTION_BLOCK or FUNCTION; a
FUNCTION that writes a global so has effects, and its calls keep the
standard's order of evaluation ([dependencies](dependencies.md)).

**Why.** Some tools and editors keep a project's globals in one list and
let every POU use them. A platform that wants that may switch this on,
or have its editor write the `VAR_EXTERNAL` declarations where the user
sees them; iecf supports both and decides neither.

**To make a program portable.** Add to each POU a `VAR_EXTERNAL` block
naming every global it uses, with the global's type.

## var-config-list: VAR_CONFIG outside a CONFIGURATION, paths from the PROGRAM's name

**The standard.** Ed.3 6.8.2 and Annex A (`Config_Decl`): `VAR_CONFIG`
is part of a `CONFIGURATION`, after its resources; a path starts with
the resource (if the configuration names one), then the program
instance. Without the extension, a `VAR_CONFIG` at the top of a file is
E0025.

**The extension.** As CODESYS writes it: `VAR_CONFIG ... END_VAR` at the
top of any file, in as many blocks as wanted. Each PROGRAM is its own
one instance, under its own name, and a path starts there:

```
VAR_CONFIG
  PLC_PRG.valve1.coil AT %QX0.0 : BOOL;
END_VAR
```

Everything else is checked as for the standard form: the path, the
repeated type, the address's area, the initial values, and every `*`
address of every PROGRAM completed (E0023). A project that has a
`CONFIGURATION` keeps its `VAR_CONFIG` there; a list of its own is then
still E0025.

**Why.** CODESYS projects, and those of the systems built on CODESYS,
complete their `AT %I*` addresses this way: there is no textual
`CONFIGURATION`, tasks call PROGRAMs by name. (Beckhoff TwinCAT has no
`VAR_CONFIG` at all; it links `%I*` variables in its I/O configuration,
outside the source text, which iecf does not read: PROPOSAL.md §2b.)

**To make a program portable.** Write a `CONFIGURATION` with a resource,
its tasks and a program instance per PROGRAM, and move the entries into
its `VAR_CONFIG`, each path prefixed with the resource's name.

## time-number-conversions: TIME and LTIME to and from integers and reals, in ms and ns

**The standard.** Ed.3 Figure 11 defines no conversion between TIME or
LTIME and a number ("-"; its key: "The implementation may support
additional Implementer specific data type conversions"), and Ed.2 named
none. Without the extension, `DINT_TO_TIME`, `TIME_TO_UDINT`,
`TIME_TO_REAL`, `TO_TIME(i)` and the like are E0013, given a literal
too (`DINT_TO_TIME(1500)`; until 2026-11-19 a literal slipped through). A duration is made
from a number with TIME arithmetic: `T#1ms * n`, `T#1ms * r` (Table 35).

**The extension.** `<number>_TO_TIME`, `TIME_TO_<number>`, the same for
LTIME, and the overloaded `TO_TIME`, `TO_LTIME` from a number and
`TO_<number>` from a duration, where the number is an integer, BYTE,
WORD, DWORD, LWORD, REAL or LREAL. A TIME converts as its count of
milliseconds, an LTIME as its count of nanoseconds: `DINT_TO_TIME(1500)`
is `T#1s500ms`, `TIME_TO_DINT(T#2s)` is 2000, `TIME_TO_REAL(T#2s)` is
2000.0. Each converts as that count would: an integer that does not fit
the target keeps its low bits and is flagged, a bit string takes the
low bits, a real is rounded half to even and one outside the target
flagged and its statement left undone, as `REAL_TO_DINT`'s
([Conversions](behaviour/conversions.md),
[TIME and LTIME](behaviour/time.md#time-and-numbers-only-with-an-extension)).

**Why.** CODESYS, Beckhoff TwinCAT and Siemens convert a TIME to and
from integers counting milliseconds (`TIME_TO_DWORD(T#5m)` is 300000 in
TwinCAT), and CODESYS has `TIME_TO_REAL` and `TIME_TO_LREAL` counting
the same (decided 2026-11-18, florin: milliseconds for the reals too,
one unit for every number). Programs written for them use these. Not
every vendor agrees: matiec and logi.CAD 3 count seconds, with the
fraction in a real (`TIME_TO_REAL(T#1s500ms)` is 1.5 there), Panasonic
counts 10 ms; a program carried over from matiec waits a thousand times
less here, and the Annex F blocks' `TR` and `TD` are given in
milliseconds.

**To make a program portable.** Replace `DINT_TO_TIME(n)` with
`T#1ms * n` and `REAL_TO_TIME(r)` with `T#1ms * r` (Table 35, MUL). The
standard has no way back from a duration to a number (Table 35 divides
a TIME only by a number), so keep a duration in a TIME throughout and
compare it with TIME literals.

## bit-string-to-bool: BYTE, WORD, DWORD and LWORD to BOOL: TRUE for any value but 0

**The standard.** Ed.3 contradicts itself. Figure 11 defines no
conversion from a bit string to BOOL ("-"); Table 25 lists
`BYTE_TO_BOOL`, `WORD_TO_BOOL`, `DWORD_TO_BOOL` and `LWORD_TO_BOOL` as
"Binary transfer of the rightmost bit into the target". iecf follows
Figure 11: without the extension they, and `TO_BOOL` of a bit string,
are E0013.

**The extension.** They convert, and the result is TRUE for any value
but 0: `BYTE_TO_BOOL(BYTE#2)` is TRUE, where Table 25's rightmost bit
would make it FALSE. Integers to BOOL stay refused.

**Why.** CODESYS and Beckhoff TwinCAT have these conversions and give
TRUE for any value but 0; programs written for them use them (decided,
florin, 2026-11-08: PROPOSAL.md §6.8).

**To make a program portable.** Write what is meant: `b <> 0` for any
bit set, `b.0` (Ed.3 6.5.3, bit access) for the rightmost bit.

## reference-to: CODESYS's REFERENCE TO, bound with REF=, tested with __ISVALIDREF

**The standard.** Ed.3 6.4.4.10 has one kind of reference, `REF_TO`,
taken with `REF()` and dereferenced with `^`. `REFERENCE TO`, `REF=`
and `__ISVALIDREF` are not IEC 61131-3. Without the extension the type
is E0011 and `REF=` is E0005.

**The extension.** As CODESYS and Beckhoff TwinCAT have it:

- `r : REFERENCE TO T` is a reference that stands for what it
  references wherever it is used: `r := 5`, `x := r + 1`, `r.f`, `r[i]`
  and `r(...)` for a function block go through it, with no `^`.
- `r REF= x` binds it to `x`, which `REF(x)` could take (E0026);
  `r REF= r2` binds it to what the REFERENCE TO `r2` references;
  `r REF= 0` unbinds it. It starts unbound; it takes no initial value
  (E0013).
- `__ISVALIDREF(r)` is TRUE when `r` is bound.
- A function block's `VAR_INPUT` of a REFERENCE TO is bound by a call,
  `fb(r := x)`: by its argument, as REF= would. A call that does not
  pass it leaves it bound as it was, as an IEC input keeps its value.
  A FUNCTION's or method's is bound the same, given by name or in
  order: `F(r := x)`, `F(x)` (in order since 2026-11-19).
- No vendor page found says what a call that does not pass such an
  input does, nor shows `r.f`, `r[i]` or `r(...)` through a REFERENCE
  TO; both are as above, inferred from the rest (decided, florin,
  2026-11-09).
- An unbound REFERENCE TO used is a fault, `IECF_FAULT_NULL`, as a NULL
  `REF_TO` dereferenced is ([dependencies](dependencies.md),
  References).
- Where a `REF_TO` cannot be, a REFERENCE TO cannot either: not a
  VAR_IN_OUT, not RETAIN, not at an address. Nor, as in CODESYS and
  TwinCAT, an array of REFERENCE TO, a `REF_TO` a REFERENCE TO, or a
  REFERENCE TO a reference (E0027).

In the C, a REFERENCE TO is the same `iecf_ref_t` as a `REF_TO`
([The C output](c-output.md) §4), and its uses are generated as the
`REF_TO` ones they stand for: `r` as `r^`, `r REF= x` as
`r := REF(x)`. The symbol table names its type `REFERENCE TO T`.

**Why.** Programs written for CODESYS and TwinCAT use REFERENCE TO for
what Ed.3 does with `REF_TO` (decided, florin:
PROPOSAL.md §5 step 3).

**To make a program portable.** Declare `r : REF_TO T`, write
`r := REF(x)` for `r REF= x`, `r := NULL` for `r REF= 0`,
`r <> NULL` for `__ISVALIDREF(r)`, and `r^` wherever `r` is used. A
function block's REFERENCE TO input becomes a `REF_TO` input passed
`REF(x)`.

## global-namespace: `_.` names the global namespace, as Siemens and PLCnext write it

**The standard.** Ed.3 6.9.1 puts every element declared outside a
NAMESPACE, and every standard function and function block, in the
global namespace, but gives no way to name it. Its one example,
`Ton2: PUBLIC.TON; (* uses the standard timer *)` (6.9.3), cannot be
read: PUBLIC is an access specifier, and the grammar has no such name.
So a namespace that declares its own `TON` cannot reach the standard
one. Without the extension, `_.` is E0028.

**The extension.** As Siemens TIA Portal and PLCnext Engineer write it:
`_.` before a name looks it up in the global namespace only, past every
namespace the code is in and every USING (C#'s `global::`).

- `_.TON` is the standard timer, from inside a namespace with its own
  `TON`;
- `_.Plant.Main` is the top-level namespace `Plant`'s `Main`, even
  inside a namespace that has a `Plant` of its own;
- `_.g`, a global variable, is `g`.

**Why.** Example 2 of Ed.3 6.9.2 declares a namespace's own TON, "an
implementation of TON with a new name"; inside that namespace the
standard TON is then out of reach (NAMESPACES.md, D4).

**To make a program portable.** Give the namespace's own element
another name, so the standard one is reached by its short name.

## endian-dates: TO_BIG_ENDIAN and its kin reverse LTIME and the dates too

**The standard.** Ed.3 6.6.2.5.13 lists the types the endianness
functions convert - ANY_INT and ANY_BIT of 16 bits or more, ANY_REAL,
WCHAR, TIME, arrays and structures of them - and says "Other data types
are not converted but may be contained in the structures to convert".
Without the extension, `TO_BIG_ENDIAN`, `TO_LITTLE_ENDIAN`,
`FROM_BIG_ENDIAN` and `FROM_LITTLE_ENDIAN` give an LTIME, DATE, TOD, DT,
LDATE, LTOD or LDT back as it is, alone or in a structure or array.

**The extension.** They convert those seven too: their bytes as iecf
holds them reversed (LTIME ns, DATE and DT s, TOD ms, the long ones
ns), so a reversed value is another of the type, not one with a meaning
of its own: `TO_BIG_ENDIAN(DT#1970-01-02-00:00:00)` is
`DT#2038-03-21-13:52:00` on a little-endian target
([Conversions](behaviour/conversions.md#byte-order-to_big_endian-and-its-kin)).

**Why.** PLCnext's `TO_BIG_ENDIAN` takes ANY_DURATION, and logi.CAD 3's
DATE, TOD and DT, which "IEC does not demand"; a program that sends a
timestamp over a big-endian protocol as it is held uses them so
(decided 2026-11-17, florin; an extension since 2026-11-18).

**To make a program portable.** Lay the value over a bit string of its
size in a structure with relative locations, `STRUCT OVERLAP v AT %B0 :
DT; raw AT %B0 : DWORD; END_STRUCT`, and convert the bit string: that is
IEC 61131-3, though what a DT's bytes are is each implementation's own
(here its seconds since 1970).

## string-parentheses: STRING(20) for STRING[20], as CODESYS and TwinCAT write a length

**The standard.** A STRING's or WSTRING's length is in brackets,
`STRING[20]` (Ed.3 Annex A, String_Type_Name; Tables 10 and 13). Without
the extension, `STRING(20)` is E0005.

**The extension.** `STRING(n)` and `WSTRING(n)` are `STRING[n]` and
`WSTRING[n]`, in every place a type is written.

**Why.** CODESYS and TwinCAT write the length so in their examples
(`STRING(35)`), and take brackets too; programs written for them use
parentheses.

**To make a program portable.** Write the length in brackets.

## temp-instances: a function block instance in VAR_TEMP or in a FUNCTION, set up at each call

**The standard.** "Instances of other function blocks, classes and
object oriented function blocks can be declared in all variable sections
except the VAR_TEMP section" (Ed.3 6.6.3.2 rule 17), and "For functions
and methods, the keywords VAR and VAR_TEMP are equivalent" (6.5.2.2).
Without the extension, an instance - or an array or structure holding
one - in VAR_TEMP, or in a FUNCTION's or a METHOD's VAR, is E0005.

**The extension.** It is declared there, and set up at each call of
the POU, its initial values given again, as every temporary variable
is: a timer there starts again at each call. It reaches the globals of the instance or resource
that is running ([Implementer choices](dependencies.md#several-configurations-and-the-scope-of-globals-ed3-6522-681)).

**Why.** CODESYS and TwinCAT let a FUNCTION and a method declare an
instance in VAR, and a block or program one in VAR_TEMP, set up at each
call, for a block used as a calculation (a `R_TRIG` whose state is not
kept, a block computing a value at once).

**To make a program portable.** Declare the instance in the caller's
VAR and give it to the FUNCTION or method as a `VAR_IN_OUT`; in a block
or program, declare it in VAR.

## in-out-constant: CODESYS's VAR_IN_OUT CONSTANT: given any value, a STRING of any length

**The standard.** Ed.3's VAR_IN_OUT has no qualifier (Annex A,
In_Out_Decls), and "shall not be applied" an implicit conversion
(6.6.1.6 rule 6): it is given a variable of its type exactly. Without
the extension, `VAR_IN_OUT CONSTANT` is E0005.

**The extension.** A `VAR_IN_OUT CONSTANT` of a FUNCTION, a method or a
FUNCTION_BLOCK is passed by reference and is not written by the POU
(E0015, as a CONSTANT is not), nor given on to an in-out that could
write it. A call gives it any value its type takes as an input does,
with the implicit conversions of Figure 11:

- **a variable of its type**, by its address: the POU reads the
  variable itself;
- **any other value** - a literal, a constant, an expression, an
  element, a variable at an address or of a narrower type - a copy made
  for the call, which lives while the POU runs and is never stored back;
  into a subrange it is clamped and flagged, as an input's value is;
- **a STRING or WSTRING of any length**, whatever the length declared:
  the POU reads it whole (`LEN`, comparisons, a copy into a variable,
  cut there as any store), and a string it computes from it is as long
  as the declared length lets the result be, cut and flagged beyond.

```
FUNCTION Label : INT
VAR_IN_OUT CONSTANT name : STRING[8]; scale : INT; END_VAR
Label := LEN(name) * scale;
END_FUNCTION

n := Label(name := 'a much longer name', scale := 2);   (* 36 *)
n := Label(name := s, scale := k + 1);
```

An in-out CONSTANT of a function block instance, an interface or an
`ARRAY[*]` is given a variable, as any in-out is. A PROGRAM's is
connected as any of its in-outs. The harness case `inoutconst` runs
these.

**Why.** CODESYS and TwinCAT have it: passed by reference, not written,
given a variable, a constant or a literal; "There are no restrictions
on the string length of the transferred variables/constants, and the
length does not depend on the string length of the VAR_IN_OUT CONSTANT
variable" (Beckhoff). Libraries written for them declare string
parameters so, for speed: no copy, any length.

**To make a program portable.** Declare the parameter `VAR_INPUT`: a
constant is given to an input, and a STRING input of the longest length
takes any shorter one, copied.

## initial-expressions: a variable's initial value names variables and calls functions, as CODESYS's and TwinCAT's do

**The standard.** An initial value is a constant expression (Ed.3
6.4.4.1, Figure 6; 6.6.1.2.4: it "must evaluate to a constant value at
compile time"), which iecf works out as it compiles
([Initial values](behaviour/arrays-subranges.md#initial-values-are-constant-expressions)).
Without the extension, a variable or the result of a FUNCTION of the
program's there is [E0032](errors/E0032.md).

**The extension.** A variable's initial value may name other variables
and call functions, the program's and the standard ones, and is worked
out as the variable is set up:

```
FUNCTION_BLOCK Scaler
VAR_INPUT gain : REAL := 2.0; END_VAR
VAR_OUTPUT top : REAL := gain * 100.0; END_VAR
END_FUNCTION_BLOCK

PROGRAM main
VAR_EXTERNAL base : INT; END_VAR
VAR
    a   : INT := base * 2;
    b   : INT := a + Twice(4);                 (* a FUNCTION of the program's *)
    arr : ARRAY[0..2] OF INT := [a, b, 7];
    s   : STRING[8] := CONCAT('v', INT_TO_STRING(a));
    tm  : TON := (PT := T#1s * a);
    sc  : Scaler := (gain := 0.5);             (* top 200.0: gain's own 2.0 *)
END_VAR
```

- **What may have one**: a variable of a PROGRAM, a FUNCTION_BLOCK or
  a CLASS, in VAR, VAR_INPUT, VAR_OUTPUT or VAR_TEMP; a FUNCTION's or
  method's VAR and VAR_TEMP; a VAR_GLOBAL. So may each value of an
  array's or a structure's initial value, and each value an instance's
  declaration gives its variables. A CONSTANT's may not: the compiler
  works its value out, and CODESYS and TwinCAT say the same ("Inputs or
  function calls cannot be specified"). Neither may a type's, a
  FUNCTION's or method's input's (the value a caller that leaves it
  out gives), its output's, or a PROGRAM's own VAR_GLOBAL's: E0032
  there, as without the extension.
- **In the order declared.** The variables are set up in the order
  they are declared, so an initial value reads those declared before
  it, [E0036](errors/E0036.md) else. An instance sets up its variables
  once, as the program starts; its VAR_TEMPs at each run, after, so a
  VAR_TEMP's reads the instance's variables; its in-outs are bound by
  each call, and are not there as it is set up. A FUNCTION or method is
  given its parameters, then sets up its variables, at each call. The
  globals are set up before the programs: the top-level ones, then the
  CONFIGURATION's, then each RESOURCE's, in the order written, and an
  initial value of one sees the globals seen where it is declared. A
  constant may be named wherever it is declared. A function an initial
  value calls reads its VAR_EXTERNALs as they are then: a global not
  yet set up does not hold its initial value yet.
- **As an assignment.** The variable is given its type's initial value,
  then the value as an assignment statement would give it
  (docs/c-output.md §5): converted as one is, clamped into a subrange
  and flagged, a STRING cut to fit and flagged, an overflow flagged -
  each at the POU and line of the initial value. An instance's own
  variables are set up before those its declaration gives: in `sc`
  above, Scaler's `top` reads `gain`'s own 2.0, and `gain` is then
  given 0.5.
- **A fault** - a division by zero, an index out of its bounds - stops
  the set-up where it is, as it stops a body, its POU and line in
  `ctx->fault`: the program is stopped until the next start, as on a
  fault in a task (docs/c-output.md §5).
- **A start sets each variable up anew** but what it keeps (Ed.3
  6.5.6): at a warm start, `m : INT := r + 1` with `r` RETAIN is `r`'s
  kept value plus one. A RETAIN variable's own initial value is worked
  out at a cold start only.

The harness cases `initexpr` and `initfault` run these.

**Why.** CODESYS and TwinCAT have it: a variable's initial value
"consists of any valid expression of the programming language ST",
defined "with the help of constants, other variables or functions"
(`y : INT := x + fun(4);`, "notice the order!"). In a function block,
CODESYS sets up "firstly, all constants in accordance with the order of
their declarations", then "all other variables in accordance with the
order of their declarations", and "variables from global variable lists
are always initialized before the local variables of a POU" (CODESYS
help, variable initialization). It refuses a global variable list's
initial value that reads a global set up after it (its help on the
`global_init_slot` attribute); iecf sets the globals up in the order
written and refuses the same (E0036). Code written for them sets one
variable up from another so.

**To make a program portable.** Give the variable a constant initial
value, and set it from the others in the body at the first cycle: a
`first : BOOL := TRUE;` tested, and cleared there.

## codesys-enums: CODESYS's enumerations: a value named COLOR.yellow, the base type after the values

**The standard.** Ed.3 names a value of an enumeration, or of a type
with named values, `COLOR#yellow` (6.4.4.2, 6.4.4.3), and writes a base
type before the values: `DWORD (white := 16#FFFFFF00, ...)` (6.4.4.3,
Annex A Enum_Type_Decl). Without the extension `COLOR.yellow` is
[E0010](errors/E0010.md) and `(...) DWORD` [E0005](errors/E0005.md),
each saying how Ed.3 writes it.

**The extension.**

```
{attribute 'qualified_only'}
{attribute 'strict'}
TYPE COLOR :
(
    white := 16#FFFFFF00,
    green := 16#FF00FF00,
    black := 16#88000000
) DWORD := black
;
END_TYPE

c := COLOR.green;                  (* COLOR#green *)
IF m = Lib.Mode.run THEN ...       (* a TYPE in the namespace Lib *)
```

- **`COLOR.yellow` is `COLOR#yellow`**, where COLOR is a TYPE - an
  enumeration or a type with named values, in a namespace too
  (`Lib.Mode.run`) - that has the value. A variable, an instance or a
  structure of the name keeps its field: `s.white` is the field of the
  variable `s`.
- **`(white := 16#FFFFFF00, ...) DWORD := black` is `DWORD (white :=
  16#FFFFFF00, ...) := black`**: a type with named values, its base
  type one of the integer and bit string types CODESYS lists (INT,
  UINT, SINT, USINT, DINT, UDINT, LINT, ULINT, BYTE, WORD, DWORD,
  LWORD), its initial value after it.
- The attributes before the TYPE are pragmas, which iecf reads past
  (Ed.3 Table 4). Under `strict` CODESYS refuses arithmetic on an
  enumeration; iecf's plain enumerations are so already (Ed.3 6.4.4.2),
  and a type with named values is its base type, as Ed.3 6.4.4.3 has
  it. Under `qualified_only` CODESYS refuses a bare value name; iecf
  takes one that is unique, as Ed.3 does.

The harness case `codesysenum` runs these.

**Why.** CODESYS and TwinCAT write their enumerations so: "TYPE COLOR :
( white := 16#FFFFFF00, ... ) DWORD := black" and "cbFlower :=
COLOR_BASIC.blue;" (CODESYS help, enumerations). CODESYS calls a base
type an "extension of the IEC 61131-3 standard", as Ed.2 had none, and
TwinCAT gives each enumeration it creates `qualified_only`, so code
written for it names every value `E_Color.eRed`.

**To make a program portable.** Write `COLOR#yellow`, and the base type
before the values: `DWORD (white := 16#FFFFFF00, ...) := black`.

## struct-semicolon: END_STRUCT with no `;` after it, as CODESYS and TwinCAT write a structure

**The standard.** Every declaration in a TYPE ends with `;`, a
structure's too: `TYPE S : STRUCT a : INT; END_STRUCT; END_TYPE` (Ed.3
Annex A, `Data_Type_Decl : 'TYPE' ( Type_Decl ';' )+ 'END_TYPE'`).
Without the extension, a missing `;` after `END_STRUCT` is
[E0005](errors/E0005.md).

**The extension.** After a structure's `END_STRUCT` the `;` may be left
out where END_TYPE or the next type's `name :` follows:

```
TYPE
  Pair : STRUCT a, b : INT; END_STRUCT
  Trio : STRUCT x, y, z : REAL; END_STRUCT
END_TYPE
```

Only there: after any other type declaration, and after a structure
given an initial value, the `;` stays required.

**Why.** CODESYS documents a structure so, `END_STRUCT` then
`END_TYPE` with no `;` (CODESYS help, "Structure"), as did CODESYS 2.3;
TwinCAT's syntax is the same (Beckhoff InfoSys, "STRUCT"). Every TYPE of
a CODESYS export is written so - all 14 of the OSCAT BASIC library.
PLCnext writes `END_STRUCT;`; Siemens SCL requires the `;` ("the
END_STRUCT keyword must be terminated by a semicolon", S7-SCL manual).

**To make a program portable.** Write `END_STRUCT;`. CODESYS and
TwinCAT take that too.

## real-without-point: a real literal with an exponent and no point, 1E37

**The standard.** A real literal has a point: `Real_Literal : (
Real_Type_Name '#' )? Signed_Int '.' Unsigned_Int ( 'E' Signed_Int )?`
(Ed.3 Annex A; Table 5 writes `1.0E+6`). Without the extension, `1E37`
is [E0005](errors/E0005.md), its message giving the literal with a
point.

**The extension.** Digits and an exponent with no point, `1E37`,
`2e-3`, `4E+2`, are a real literal of the same value as `1.0E37`,
`2.0e-3`, `4.0E+2`, typed as any real literal is: REAL or LREAL by
where it stands, `REAL#1E3` too.

**Why.** CODESYS and TwinCAT write a real's exponent so in their own
documentation (`-1E-44`, CODESYS help and Beckhoff InfoSys, "REAL/LREAL
constants"); Siemens SCL: "A real number must either contain a decimal
point or an exponent (or both)" (`3E10`). Code written for them uses it:
four blocks of OSCAT BASIC (`OUT_MIN : REAL := -1E37`).

**To make a program portable.** Write a point before the exponent:
`1.0E37`.

## bare-array-init: an array's initial values without brackets, CODESYS 2.3's `:= 1, 2, 3`

**The standard.** An array's initial value is a list in brackets,
`ARRAY[1..3] OF INT := [1, 2, 3]`, each value repeated by a count before
parentheses, `[1, 2(7)]` (Ed.3 Annex A, `Array_Init : '[' Array_Elem_Init
( ',' Array_Elem_Init )* ']'`; 6.4.4.5.2). Without the extension, a
list without brackets is [E0005](errors/E0005.md).

**The extension.** In a variable's, a structure field's or an array
type's declaration, the list after `:=` may stand without its brackets,
up to the declaration's `;`; it is then the bracketed list:

```
VAR
  days : ARRAY[1..12] OF INT := 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334;
  pad  : ARRAY[1..8] OF BYTE := 2(16#FF), 6(0);     (* [2(16#FF), 6(0)] *)
END_VAR
```

A list is told by a `,` after its first value, or a count and `(`
before it; a single value without either stays a single value, which
an array is not given ([E0013](errors/E0013.md), as without the extension). Repeats,
nested arrays and structures in the list are as in brackets; how many
values a list gives is checked as in brackets: fewer or more than the
array has elements is [W0008](errors/W0008.md), the rest taking their
type's initial value or the excess ignored (Ed.3 6.4.4.5.2).

**Why.** CODESYS 2.3 documents its arrays so ("arr1 : ARRAY [1..5] OF
INT := 1,2,3,4,5;", CoDeSys 2.3 manual, "Initializing Arrays"), and its
exports, the OSCAT BASIC library among them, are written so. Siemens
SCL takes the brackets as optional too ("The initial values can
optionally be enclosed in a bracket", S7-SCL manual). CODESYS 3's own
examples use brackets; PLCnext requires them.

**To make a program portable.** Put the list in brackets: `:= [0, 31,
59]`.

## input-constant: CODESYS's VAR_INPUT CONSTANT, a plain VAR_INPUT outside its CFC editor

**The standard.** CONSTANT qualifies VAR, VAR_EXTERNAL and VAR_GLOBAL
sections only (Ed.3 Annex A: `Input_Decls : 'VAR_INPUT' ( 'RETAIN' |
'NON_RETAIN' )? ...` has none). Without the extension, `VAR_INPUT
CONSTANT` is [E0005](errors/E0005.md).

**The extension.** `VAR_INPUT CONSTANT ... END_VAR` is a plain
`VAR_INPUT` section: its inputs are given by the caller, take their
initial value when not given, and may be written in the body, as any
input. With RETAIN or NON_RETAIN beside it, it is still E0005: a section
has one qualifier.

**Why.** In CODESYS and TwinCAT the CONSTANT of an input section means
something to their CFC editor only: there such inputs are not drawn as
pins but set as the block's parameters ("Edit Parameters"); "this
functionality and the declaration of variables with the keyword
VAR_INPUT CONSTANT only apply to the CFC editor" (Beckhoff InfoSys;
CODESYS help, "Changing constant inputs"). Their text editors make no
difference, and neither does iecf. Libraries written for them use it:
40 blocks of OSCAT BASIC. It is not CODESYS's VAR_IN_OUT CONSTANT,
which is read-only ([in-out-constant](#in-out-constant-codesyss-var_in_out-constant-given-any-value-a-string-of-any-length)).

**To make a program portable.** Drop the CONSTANT: `VAR_INPUT`.

## keyword-names: STEP, ON, R_EDGE and F_EDGE as names, which CODESYS does not reserve

**The standard.** Keywords "shall not be used for any other purpose,
for example, variable names or extensions" (Ed.3 6.1.3). STEP is SFC's
(Annex A, Step), ON a RESOURCE's (`RESOURCE r ON PLC`), R_EDGE and
F_EDGE an edge input's (`x : BOOL R_EDGE`). Without the extension, each
used as a name is [E0005](errors/E0005.md), once per name and file, at
its first place.

**The extension.** These four are names wherever their keyword cannot
stand, and keywords where it can:

- STEP is the keyword before a step's name (`STEP s1 :`), a name
  elsewhere: `step := 1`, `FOR i := 1 TO n BY step DO`, `MAX(IN1 :=
  step, ...)`, `fb.step`.
- ON is the keyword after `RESOURCE name`, a name elsewhere.
- R_EDGE and F_EDGE are keywords after a declaration's BOOL (`x : BOOL
  R_EDGE`), names elsewhere.

The grammar has each keyword in that one place only, so a program that
uses them as keywords - an SFC, a configuration, edge inputs - reads as
without the extension, in the same POU as the names too. A variable
named `step` and a step named `s1` live side by side; a step cannot be
named `step` (`STEP step :`, the keyword before a keyword).

Only these four: they are what CODESYS's code uses that Ed.3 reserves
and CODESYS does not. The other Ed.3 keywords stay keywords.

**Why.** CODESYS's reserved words do not include STEP, ON, R_EDGE or
F_EDGE (its keyword list, CODESYS help "Keywords"; the CODESYS 2.3
list, as ABB's help copies it), nor do TwinCAT's: CODESYS has no SFC in
text, and declares a configuration's resources in its project tree, not
in text. Code written for them names variables so: OSCAT BASIC has `step`
(a counter's step width), `on` (a hysteresis's output) and `r_edge` in
14 blocks.

**To make a program portable.** Rename them: `step_size`, `is_on`,
`rising`.

# Proposed, not built

These are extensions iecf could offer and does not yet: the standard's
behaviour stays the default, and each would be switched on by name as
those above are. None of them is accepted by `--extension` today.

### recursion: a FUNCTION, block or method that calls itself, with a depth limit

**The standard.** "The recursive call of POUs and methods is
Implementer specific" (Ed.3 6.6.1). iecf refuses it (E0016): a FUNCTION,
a function block, a PROGRAM or a method that can reach itself through
calls, directly or through others, a call through an instance, THIS,
an in-out or an interface counting as a call of every override or
implementation that could run there (decided 2026-11-12, florin). Its
stack would have no bound, and an overflow of C's stack is no fault the
program could report: it is undefined behaviour, which the output
never has (docs/c-output.md §1).

**The extension, as proposed.** `--extension recursion` would accept
recursive calls, each counted: every call of a recursive POU or method
adds one to a depth in the context, and a call past a limit the
platform sets is a fault (a new code, `IECF_FAULT_DEPTH`), which stops
the task as an index out of range does, rather than overflowing the
stack. The platform sizes its stack for the limit. A program with no
recursive call would be built exactly as without the extension.

**Vendors.** CODESYS and TwinCAT refuse a FUNCTION that calls itself
(CODESYS error C0224, "Call Recursion") and allow a method to, with a
warning that the stack's use cannot be worked out (C0298), silenced by
`{attribute 'estimated-stack-usage' := '...'}`. PLCnext Engineer
refuses every recursive call of a POU (SEM1030, "Recursive dependency
found in program organization unit") and of a method (SEM1259). Siemens
documents no rule; its CPUs stop when the block nesting depth is
exceeded. The reference compiler (matiec) can express none: a POU is
known only after its declaration.

**To make a program portable.** Write the recursion as a loop, with an
array as its stack where it needs one.

### union: CODESYS's and TwinCAT's UNION, its members sharing their bytes

**The standard.** Ed.3 has no UNION. Its way to let components share
memory is a structure with relative locations and OVERLAP (6.4.4.7):
`STRUCT OVERLAP r AT %B0 : REAL; d AT %B0 : DWORD; END_STRUCT`, which
iecf compiles (docs/c-output.md §4). `STRUCT OVERLAP` without relative
locations says nothing of which components share which bytes, and is
refused (E0034; decided 2026-11-13, florin: for now).

**The extension, as proposed.** `--extension union` would accept the
type as CODESYS and TwinCAT write it:

```
TYPE U :
UNION
    r : REAL;
    d : DWORD;
    w : WORD;
END_UNION
END_TYPE
```

read as a `STRUCT OVERLAP` whose every member is at `%B0`: its size the
largest member's, its bytes laid out as any structure with relative
locations is, the same in C memory and at an address. A variable of
it is initialised through one member, `v : U := (r := 1.5);`, as
CODESYS writes it; without one, the first member's type gives the
bytes it covers, as for an OVERLAP. A member's initial value in the
type would be allowed on one member at most (CODESYS's pages allow one
and say nothing of several). Open, for when it is built: whether the
extension also reads a `STRUCT OVERLAP` without relative locations as
such a union.

**Vendors.** CODESYS: "The members of the instance of a UNION data type
all reserve the same memory space. All members have the same address
offset and share the memory"; "The memory requirement is determined by
the largest member". TwinCAT: every member starts at the same address,
"and therefore co-written". PLCnext Engineer plans relative addressing
of structure elements through UNION types. Siemens has no UNION; SCL
overlays a variable with `AT` in a declaration instead. The reference
compiler (matiec) has neither.

**To make a program portable.** Write the union as `STRUCT OVERLAP`,
each member `AT %B0`; that is IEC 61131-3 and compiles without the
extension.

### action-call: an action called as `fb.act()`

**The standard.** An ACTION is run by the steps it is associated with,
under its qualifier (Ed.3 6.7.4); the grammar's Invocation names a
function block instance or a method, no action. iecf refuses `fb.act()`
with E0016, naming the action.

**The vendors.** CODESYS and TwinCAT let a POU have actions without any
SFC, as named pieces of its body sharing its variables, and call them
`fb.act();` or, inside the POU, `act();`.

**Proposed.** `--extension action-call`: an action of a FUNCTION_BLOCK
or PROGRAM called as a statement, from inside by its name and from
outside through the instance, its body run once with the instance's
variables; one that a step also runs would be refused, as its action
control (N, S, R, the timed qualifiers) and a direct call would both
claim it. A METHOD without inputs or a result does the same in the
standard's own terms.
