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
| `codesys` | what code written for CODESYS and TwinCAT uses | `implicit-externals`, `var-config-list`, `time-number-conversions`, `bit-string-to-bool`, `reference-to`, `string-parentheses`, `temp-instances`, `in-out-constant`, `initial-expressions`, `codesys-enums`, `struct-semicolon`, `real-without-point`, `bare-array-init`, `input-constant`, `keyword-names`, `time-function`, `bit-string-arithmetic`, `integer-shifts`, `integer-bit-access`, `integer-expt`, `implicit-narrowing`, `date-number-conversions`, `shadowing` |

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

## time-function: TIME() gives the clock in milliseconds, CODESYS's and TwinCAT's time since start

**The standard.** Ed.3 has no function that reads a clock: `TIME` is a
type, and time is measured by the timers TON, TOF and TP (6.6.3.5.5,
Table 46), against the time the platform gives the task. Without the
extension, `TIME()` is [E0010](errors/E0010.md), saying so.

**The extension.** `TIME()`, with no argument, is a TIME: the running
task's clock - the `now_ns` the platform passes the task function, the
one the timers measure against ([The C output](c-output.md) §3) - in
whole milliseconds, rounded down, its low 32 bits as a TIME. It is the
same value anywhere in one run of the task: in the PROGRAM, in the
FUNCTIONs and blocks it calls.

```
start := TIME();                   (* once *)
elapsed := TIME() - start;         (* each cycle: the time since *)
```

- **Its origin is the platform's.** CODESYS counts from the
  controller's start ("the time (in milliseconds) that has elapsed
  since system boot", CODESYS help, "TIME"); TwinCAT says its value
  "does not represent an absolute reference point" and is meant for
  differences. iecf's is whatever origin the platform's clock has: a
  platform that starts it at 0 with the program gives CODESYS's value.
  Measure with differences, as code written for them does.
- **It goes round.** As in CODESYS, the count goes round after 2^32 ms
  (49.7 days). A TIME is signed in iecf ([TIME and LTIME](behaviour/time.md)),
  so the reading turns negative after 2^31 ms (24.8 days); the
  difference of two readings less than 24.8 days apart is still right,
  and a subtraction across the turn is flagged as an overflow, as any
  TIME's ([Implementer choices](dependencies.md#the-time-types-ranges-and-resolution-ed3-642-table-10)).
  Reading the clock itself is never flagged.
- In an initial value (initial-expressions), set up before any task
  has run, it reads the context's time then: 0.

**Why.** CODESYS and TwinCAT have it, as an operator of their own
(CODESYS help, "Operators: TIME"; Beckhoff InfoSys, "TIME"), and
CODESYS 2.3 had it; code written for them measures time with it. The
OSCAT BASIC library reads it in two functions, `T_PLC_MS` and
`T_PLC_US`, which 63 of its blocks build on. PLCnext has no TIME()
(its forum points to GetMicrotick); Siemens reads its clock with
`TIME_TCK` (SFC 64), which counts the same way, in milliseconds, round
at 2^31.

**To make a program portable.** Measure with a timer: a TON's ET is the
time since its IN rose (Table 46). Or keep a TIME of your own, adding
the task's interval each cycle.

## bit-string-arithmetic: arithmetic on BYTE, WORD, DWORD and LWORD, each counted as the unsigned integer of its width

**The standard.** A bit string is a string of bits, not a number: Ed.3's
arithmetic takes ANY_NUM (Table 29; MOD ANY_INT), an array index ANY_INT
(6.4.4.5.1: "a value corresponding to one of the sub-types of generic
type ANY_INT"), an implicit conversion goes from a bit string only to a
wider bit string (Figure 11), and Figure 11 converts between a bit
string and a real only DWORD with REAL and LWORD with LREAL, by their
bits (Table 25). Without the extension, each of the forms below is
[E0014](errors/E0014.md) or [E0013](errors/E0013.md), the message
naming the extension.

**The extension.** A bit string - BYTE, WORD, DWORD, LWORD, not BOOL -
counts as the unsigned integer of its width: BYTE as USINT, WORD as
UINT, DWORD as UDINT, LWORD as ULINT.

- **`+ - * / MOD`.** Two bit strings, or a bit string and an integer
  literal, give the wider bit string, computed as its unsigned integer:
  a result beyond it wraps and is flagged, a division by 0 faults, as
  UDINT's would. A bit string and a number give what the unsigned
  integer and the number give by Ed.3's implicit conversions: `INT +
  BYTE` an INT, `REAL * BYTE` a REAL, `DWORD + INT` a LINT.
- **Where a number is wanted** - an assignment, an argument, an initial
  value - a bit string is taken where its unsigned integer would be:
  `i := b` for an INT and a BYTE (USINT widens to INT), `r := w` for a
  REAL and a WORD. Not `i := d` for an INT and a DWORD: UDINT does not
  widen to INT.
- **An array index** may be a bit string.
- **To and from a real**, where Figure 11 has no conversion - BYTE and
  WORD with REAL and LREAL, DWORD with LREAL, LWORD with REAL - by value:
  `BYTE_TO_REAL(BYTE#200)` is 200.0, `REAL_TO_BYTE(51.5)` is 52 (half
  to even; beyond the bit string, flagged and the statement left undone,
  as `REAL_TO_USINT`).

What stays as without it: AND, OR, XOR, NOT, the shifts and partial
access are the bit string's; a comparison takes two values of one type;
unary minus is refused (a bit string has no sign: `0 - b`); a number
does not convert implicitly to a bit string.

**DWORD_TO_REAL is not CODESYS's.** Ed.3 defines `DWORD_TO_REAL`,
`REAL_TO_DWORD`, `LWORD_TO_LREAL` and `LREAL_TO_LWORD` as binary
transfers (Table 25), and iecf keeps them so, with or without this
extension (rule 1 above). CODESYS and TwinCAT convert them by value. So
`DWORD_TO_REAL(DWORD#100)` is 100.0 there and 1.4E-43 here, and code
written for them that uses these four gives other numbers, silently.
Write `UDINT_TO_REAL(DWORD_TO_UDINT(d))`, which means the same in
every tool. (The OSCAT BASIC library calls them in 33 blocks.)

**Why.** CODESYS and TwinCAT accept a bit string in ADD, SUB, MUL, DIV
and MOD ("Allowed data types: ... BYTE, ... DWORD, INT, ... LWORD, ...
WORD", Beckhoff InfoSys, ADD; CODESYS help, ADD and MOD), and convert
between a bit string and a real by value. Code written for them counts
in bit strings: OSCAT BASIC in 85 of its blocks (checksums, scaling,
`DWORD` timestamps). PLCnext refuses it (its ADD takes integers, reals
and durations); Siemens accepts it only with its IEC check off. How a
bit string mixed with a number is typed, CODESYS does not document;
counting it as its unsigned integer is iecf's reading, which keeps every
value a BYTE, WORD or DWORD holds.

**To make a program portable.** Convert first: `DWORD_TO_UDINT(d) + 1`,
`BYTE_TO_INT(b)`, `UDINT_TO_REAL(DWORD_TO_UDINT(d))`, and index with an
integer.

## integer-shifts: SHL, SHR, ROL and ROR of an integer, SHR keeping its sign; a bit string as N

**The standard.** The shifts take a bit string and an integer count
(Ed.3 Table 30: IN ANY_BIT, N ANY_INT). Without the extension, an
integer as IN, or a bit string as N, is [E0014](errors/E0014.md), the
message naming the extension.

**The extension.** IN may be any integer, N any integer or bit string;
the result is IN's type.

- **SHL, ROL, ROR** work on the integer's two's complement, of its
  width: `SHL(INT#16384, 1)` is -32768, `ROL(INT#-32768, 1)` is 1. A
  shift is no arithmetic: a bit into or out of the sign is not flagged.
- **SHR keeps a signed integer's sign** (decided 2026-11-19, florin):
  the bits it frees take the sign bit's value, so `SHR(INT#-8, 1)` is
  -4 and `SHR(INT#-7, 1)` -4 (a division by 2^N rounded down), and by
  the width or more a negative value gives -1, any other 0.
- **An unsigned integer** shifts as the bit string of its width:
  zeros in, 0 by the width or more.
- **N a bit string** counts as its unsigned integer. A negative N
  faults, as for a bit string ([The C output](c-output.md) §5).

**Why.** CODESYS and TwinCAT allow it ("CODESYS permits shift
operations on signed data types", the CODESYS static analysis rule
SA0052, which warns of it; TwinCAT's SA0052 the same), and their own
examples pass N as a BYTE (`SHR(nDINT, BYTE#4)`). Siemens S7-1200/1500
shift integers too and document SHR's sign: "If the specified value has
a sign, the free bit positions are filled with the signal state of the
sign bit". CODESYS does not say which SHR it does; iecf takes Siemens's,
the one that keeps a signed value's meaning. PLCnext refuses an
integer. OSCAT BASIC shifts integers in 28 blocks.

**To make a program portable.** Shift the bit string of the integer's
width: `WORD_TO_INT(SHR(INT_TO_WORD(i), 2))` (zeros in), or divide for
SHR's sign-keeping meaning: `i / 4` rounds toward 0, SHR down.

## integer-bit-access: a bit, byte or word of an integer, i.3, of its two's complement

**The standard.** A part - `.3`, `.%X3`, `.%B1`, `.%W0`, `.%D1` - is
taken of a bit string (Ed.3 6.6.1.3, Table 17: "Partial access of
ANY_BIT variables"). Without the extension, a part of an integer is
[E0014](errors/E0014.md), naming the extension.

**The extension.** An integer has the parts of the bit string of its
width - SINT and USINT a BYTE's, INT and UINT a WORD's, DINT and UDINT
a DWORD's, LINT and ULINT an LWORD's - of its two's complement, read
and written as a bit string's are: `d.31` of a negative DINT is TRUE,
`k.%X15 := TRUE` makes an INT negative, `d.%B0` is the low byte as a
BYTE. A part outside the width is E0014, as for a bit string.

**Why.** CODESYS and TwinCAT allow it on "BYTE | WORD | DWORD | LWORD |
SINT | USINT | INT | UINT | DINT ..." (CODESYS help, "Bit access in
variables"; Beckhoff InfoSys the same). PLCnext takes ANY_BIT only;
Siemens documents its slice access for bit strings. OSCAT BASIC reads
bits of integers in 7 blocks (`EVEN(x) := NOT x.0`).

**To make a program portable.** Convert to the bit string of the width
first: `DINT_TO_DWORD(d).0` is not allowed (a part of a call's result),
so `w := DINT_TO_DWORD(d); b := w.0;`.

## integer-expt: EXPT of an integer base, widened to REAL or LREAL

**The standard.** EXPT raises a real (Ed.3 Table 23: IN1 ANY_REAL, IN2
ANY_NUM); an untyped integer literal as the base is typed as a real
(`EXPT(2, n)` is accepted without the extension). A base of an integer
type is [E0014](errors/E0014.md), naming the extension.

**The extension.** A base of an integer type widens to the real that
holds every value of it, as Figure 11's implicit conversions do: REAL
for SINT, INT, USINT and UINT, LREAL for DINT, LINT, UDINT and ULINT.
The result is that real: `EXPT(INT#2, 10)` is REAL 1024.0,
`EXPT(DINT#-3, 3)` LREAL -27.0.

**Why.** CODESYS, TwinCAT and Siemens (`**`) take an integer base and
give a real ("SINT, USINT, INT, ... BYTE, WORD, DWORD, and LWORD",
CODESYS help, EXPT); PLCnext takes ANY_REAL only. OSCAT BASIC raises an
INT in one block.

**To make a program portable.** Convert the base: `EXPT(INT_TO_REAL(i),
n)`.

## implicit-narrowing: an integer or bit string where another is wanted, converted as its `*_TO_*` would, with a warning

**The standard.** A value converts implicitly only where every value of
its type is one of the target's: Figure 11's "i", INT to DINT, USINT to
INT, BYTE to WORD (Ed.3 6.6.1.6, 6.6.2.5.1). Anything else - DINT to
INT, INT to UINT, INT to WORD, DWORD to INT - takes a conversion
function, and without the extension is [E0013](errors/E0013.md), its
message naming the extension.

**The extension.** Between integers and bit strings (not BOOL), a value
goes where another type is wanted - an assignment, a function's or
block's input, a result, an initial value - converted as the explicit
`*_TO_*` would ([Conversions](behaviour/conversions.md)):

- between integers, a value the target holds is kept; one it does not
  keeps its low bits and is flagged (`IECF_FAULT_CONVERSION`):
  `i := d` with a DINT 100000 gives -31072;
- to or from a bit string, the bits move, the low ones kept, unflagged:
  `w := i` with an INT -2 gives 16#FFFE.

Each such place is warned, [W0010](errors/W0010.md), so it can be found
and written out. What converts implicitly in Ed.3 is not warned, and
neither are the operands of an operator: `INT + DINT` was always a
DINT. Reals, BOOL, durations, dates and strings keep Figure 11's rules:
`i := r` with a REAL is still E0013.

**Why.** CODESYS converts "from a "larger" type to a "smaller" type ...
implicitly" (CODESYS help, "Type conversion"), warning C0197 "possible
loss of information", and C0195/C0196 for a change of sign; TwinCAT the
same. Code written for them relies on it: OSCAT BASIC in 5 blocks
(a DINT result into an INT). PLCnext refuses it ("Implicit conversion
is only possible, if the source data type is smaller than the
target"); Siemens allows it only with its IEC check off.

**To make a program portable.** Write the conversion the warning names:
`i := DINT_TO_INT(d)`.

## date-number-conversions: DATE, TOD, DT to and from numbers: seconds since 1970, milliseconds since midnight

**The standard.** Ed.3 Figure 11 defines no conversion between a date,
a time of day or a date and time and a number ("-"; its key: an
implementation "may support additional Implementer specific data type
conversions"). Without the extension, `DATE_TO_UDINT`, `DT_TO_DWORD`,
`TOD_TO_DINT`, `UDINT_TO_DATE`, `TO_TOD(n)` and the like are
[E0013](errors/E0013.md), naming the extension. A date is built from
numbers by CONCAT_DATE, CONCAT_TOD and CONCAT_DT, and taken apart by the
SPLIT_ functions (Table 36).

**The extension.** DATE, TOD, DT, LDATE, LTOD and LDT convert to and
from the integers, the bit strings (not BOOL) and the reals, as the
count iecf holds each in ([Implementer choices](dependencies.md#the-time-types-ranges-and-resolution-ed3-642-table-10)):

| type | the count |
|---|---|
| DATE, DT | seconds since 1970-01-01 00:00 |
| TOD | milliseconds since midnight |
| LDATE, LDT | nanoseconds since 1970-01-01 00:00 |
| LTOD | nanoseconds since midnight |

The count converts as an integer of its type would (UDINT for DATE,
TOD and DT, LINT for the long ones): one the target does not hold keeps
its low bits and is flagged, to or from a bit string the bits move, a
real is rounded half to even. Into a date: a DATE's or LDATE's count
rounds down to its midnight, unflagged, as DT_TO_DATE does; a TOD's or
LTOD's count outside one day is taken modulo the day and flagged.
`DATE_TO_UDINT(D#1970-01-02)` is 86400, `TOD_TO_DINT(TOD#12:00:00)`
43200000, `UDINT_TO_DATE(90000)` D#1970-01-02
([dates](behaviour/dates.md#dates-and-numbers-only-with-an-extension)).

**Why.** CODESYS and TwinCAT convert them so, and in these units: "The
resolution for DATE is 1 day. The resolution for DT is 1 second. Both
begin at January 1, 1970", with `DATE_TO_DINT(D#1970-1-2)` 86400 and
`TOD_TO_DINT(TOD#12:0:0)` 43200000 in their examples (CODESYS help,
"DATE_TO" and "TOD_TO"; Beckhoff InfoSys the same). Code written for
them uses it: 34 blocks of OSCAT BASIC (`DT_TO_DWORD(utc)`). Siemens
counts otherwise: a DATE is days since 1990-01-01 and DT_TO_UDINT is
"not possible"; a program from S7 needs its numbers changed. PLCnext has
no 32-bit dates.

**To make a program portable.** Use CONCAT_DATE, CONCAT_TOD, CONCAT_DT
and the SPLIT_ functions (Ed.3 Table 36), and date arithmetic with TIME:
`DT#1970-01-01-00:00:00 + T#1s * n` counts seconds from 1970.

## shadowing: a POU's variable named as a type, a POU or a standard block or function

**The standard.** Names ignore case (Ed.3 6.1.2), and the names of the
standard's data types, functions and function blocks are keywords that
"shall not be used for any other purpose, for example, variable names"
(6.1.3; Ed.2 Table C.2). iecf refuses a variable named as a TYPE, a
FUNCTION_BLOCK, a CLASS, an INTERFACE or a PROGRAM, so that a name
means one thing (decided 2026-11-14, florin): without the extension,
[E0012](errors/E0012.md); one named as a standard function block or
function, [E0018](errors/E0018.md). Both name the extension.

**The extension.** A variable a POU declares - in any of its VAR
sections, a FUNCTION's, a block's, a method's - may be named as:

- a TYPE, a FUNCTION_BLOCK, a CLASS, an INTERFACE or a PROGRAM of the
  project (`speed : Speed;`, `length : INT;` beside a block `Length`);
- a standard function block (`tp : BOOL;`, TP being Ed.3's);
- a standard function (`max : INT;`).

In that POU the name is the variable wherever a value is meant, and
the type wherever a type is: `speed : Speed;` declares a variable
`speed` of the type `Speed`, and `t : TP;` beside `tp : BOOL` is an
instance of the block TP. So a POU that names a variable `max` cannot
call MAX: the call names the variable. Not covered, and still refused:
a name of an elementary type (`ltime`, `dt`: keywords in CODESYS too), THIS and
SUPER; a global variable (VAR_GLOBAL) named so; and a FUNCTION the
project declares under a standard function's name (ATAN2,
DAY_OF_WEEK), which E0018 keeps out with or without it.

**Why.** CODESYS: "a POU and a variable can be named the same" - a
local variable is found first, then globals, then POUs and types, then
the libraries, whose functions and blocks the standard's are - and the
compiler reports nothing (CODESYS help, "Shadowing rules"; its static
analysis has a rule, SA0013, to warn of it). TwinCAT the same. Code
written for them relies on it: OSCAT BASIC names a variable `tp` (a
pulse output) beside instances of TP, `length`, `speed`, `delay`,
`hyst`, `tmax` beside blocks of those names, and `sdt` of its own type
SDT, in 21 blocks.

**To make a program portable.** Rename the variable: `tp_out`,
`speed_set`, `length_n`.

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

### pointers: CODESYS's POINTER TO, ADR and SIZEOF, checked

**The standard.** Ed.3 has references (`REF_TO`, `REF()`, `^`, Table
12), which iecf builds, and no pointers: no arithmetic on a reference,
no address as a number. Without an extension `POINTER TO` is E0005 and
`ADR`, `SIZEOF` E0010.

**The vendors.** CODESYS and TwinCAT have C-like pointers: `POINTER TO
T`, `ADR(x)`, `p^`, arithmetic in bytes, `SIZEOF`, and an address in a
DWORD or LWORD. PLCnext has none; Siemens has VARIANT and REF_TO. Code
written for CODESYS uses them: 74 objects of the OSCAT BASIC library.

**Proposed.** `--extension pointers`, each pointer a fat pointer - its
object, the object's size, an offset - checked at each use, so a
pointer out of its object or NULL is a fault, never C's undefined
behaviour; arithmetic and comparison in bytes within one object;
another elementary type read and written through memcpy; `ADR` of a
temporary allowed as an argument, refused where it would outlive the
call; pointers to and from numbers refused. The proposal, its
alternatives and the decisions it needs:
[Pointers: the proposal](design/pointers.md).

### codesys-real-conversions: DWORD_TO_REAL and its kin by value

**The standard.** `DWORD_TO_REAL`, `REAL_TO_DWORD`, `LWORD_TO_LREAL`
and `LREAL_TO_LWORD` are binary transfers (Ed.3 Table 25, Figure 11):
the IEEE 754 bits move. iecf converts them so, with any extension
([Conversions](behaviour/conversions.md#bit-strings-a-binary-transfer)).

**The vendors.** CODESYS and TwinCAT convert them by value:
`DWORD_TO_REAL(DWORD#100)` is 100.0 there, 1.4E-43 here. Code written
for them calls them expecting the value and compiles here unchanged,
giving other numbers: the OSCAT BASIC library in 33 objects
(`HOUR_TO_TIME := DWORD_TO_TIME(REAL_TO_DWORD(IN * 3600000))`). OSCAT's
own CHK_REAL says why it avoids REAL_TO_DWORD: it "treats dword as a
number on many systems".

**Proposed.** `--extension codesys-real-conversions`: the four by
value, a DWORD as the UDINT it counts, an LWORD as the ULINT, as
`bit-string-arithmetic` already converts the bit strings Figure 11
leaves out. It gives standard names another meaning, as
`bit-string-to-bool` does (clause 5 f). Without it, a warning at each
of the four when the `codesys` pack is on would say what differs. For
florin to decide: the extension, the warning, both or neither.
