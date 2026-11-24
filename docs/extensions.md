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
| `codesys` | what code written for CODESYS and TwinCAT uses | `implicit-externals`, `var-config-list`, `time-number-conversions`, `bit-string-to-bool`, `reference-to`, `string-parentheses`, `temp-instances`, `in-out-constant`, `initial-expressions`, `codesys-enums`, `struct-semicolon`, `real-without-point`, `bare-array-init`, `input-constant`, `keyword-names`, `time-function`, `bit-string-arithmetic`, `integer-shifts`, `integer-bit-access`, `integer-expt`, `implicit-narrowing`, `date-number-conversions`, `shadowing`, `codesys-real-conversions`, `pointers`, `date-time-arithmetic`, `mixed-arguments`, `chained-assignment`, `jump-labels`, `optional-semicolons`, `codesys-oo`, `internal-reads`, `properties`, `method-in-outs`, `fb-init`, `struct-extends`, `var-stat`, `short-circuit`, `type-access`, `var-inst`, `any-input`, `type-constants`, `union`, `action-call`, `any-to`, `var-info`, `empty-arguments`, `bit`, `super-calls`, `struct-blocks`, `query-interface`, `dynamic-memory`, `program-instances`, `query-pointer`, `expression-statements`, `header-semicolon` |

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
- `r REF= THIS^` (with codesys-oo, which reads `THIS^` as `THIS`) binds
  it to the own instance, which lives as long as the variables of it
  `REF()` takes; Ed.3's `REF(THIS)` is taken the same
  ([E0026](errors/E0026.md); decided 2026-11-21, florin: "research then
  fix!"). A method whose result is a REFERENCE TO is called for its
  effect, `sb.Clear();`, its result left, or called on in turn,
  `sb.Append('a').Append('b')` as TcOpen's StringBuilder is: the
  result held, checked as a REFERENCE TO is where it is used (an
  unbound one faults, `IECF_FAULT_NULL`), and its method called -
  bound dynamically, as a call through a REFERENCE TO is. The harness
  case `codesysrefthis` chains three calls, one reaching a derived
  block's override. No vendor page found shows `REF= THIS^` itself;
  TcOpen, built with TwinCAT, writes it 15 times.
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
An instance of a class or of a block with methods, a base or a derived
one (OO.md D11), is declared there too, set up at each call
with its type's methods, as TwinCAT has it: it "re-initializes all
variables and function blocks, which you have declared in a method,
with each call of the method"
([Object Method](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2530307467.html))
(added 2026-11-24, florin: "go with recommendations"; TcOpen declares
one in a method 17 times, `_signalInfo : TcoSignalInfo`). An array or
a structure holding one stays [E0031](errors/E0031.md) there. The
harness case `codesysobjcopy` calls a derived one's method through
`THIS^` in a method and in a FUNCTION.

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

- **A comparison** of a bit string and a number compares the unsigned
  integer and the number (`UDINT > DWORD`, `BYTE < INT`; added
  2026-11-19, found in OSCAT NETWORK, which compares them in 20
  places).

What stays as without it: AND, OR, XOR, NOT, the shifts and partial
access are the bit string's; unary minus is refused (a bit string has no sign: `0 - b`); a number
does not convert implicitly to a bit string.

**DWORD_TO_REAL is a separate matter.** Ed.3 defines `DWORD_TO_REAL`,
`REAL_TO_DWORD`, `LWORD_TO_LREAL` and `LREAL_TO_LWORD` as binary
transfers (Table 25), and this extension leaves them so (rule 1 above).
CODESYS and TwinCAT convert them by value: `DWORD_TO_REAL(DWORD#100)`
is 100.0 there and 1.4E-43 by Ed.3. Their meaning is the
[codesys-real-conversions](#codesys-real-conversions-dword_to_real-and-its-kin-by-value-as-codesys-converts-them)
extension's, which the `codesys` pack switches on too.

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

**The extension.** A time of day and a duration convert, `TOD_TO_TIME`
and `TIME_TO_TOD` (LTOD with LTIME too), in milliseconds (nanoseconds):
into a TOD the duration's whole days are dropped, round the day,
unflagged - CODESYS's `TIME_TO_TOD(T#1d2h3m4s5ms)` is `TOD#2:3:4.005`
(added 2026-11-19, for OSCAT BUILDING's TIMER_1). And DATE, TOD, DT,
LDATE, LTOD and LDT convert to and
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
instance of the block TP. A call `NAME(...)` names the function where
the variable of that name cannot be called - a POU with a variable
`max` still calls `MAX(a, b)`, as CODESYS resolves it (OSCAT NETWORK's
DLOG_CRON_TAB has STRING variables `DAY_OF_WEEK` and calls
`DAY_OF_WEEK(d)` beside them); a variable that is an instance is
called itself. Not covered, and still refused:
a name of an elementary type (`ltime`, `dt`: keywords in CODESYS too), THIS and
SUPER; a global variable (VAR_GLOBAL) named so; and a FUNCTION the
project declares under a standard function's name (ATAN2,
DAY_OF_WEEK), which E0018 keeps out with or without it.

A method's variable may also be named as a variable of its type - the
block's, a base's - which Ed.3 6.6.5.4.3 rule 10 refuses
([E0012](errors/E0012.md)). In the method the name is the method's
own; `THIS^.x` is the type's (decided 2026-11-23, florin: "A, a new
extension, 1 commit"). CODESYS: "Local variables of a method" are
looked up before "Local variables in the function block, program, or
function, and in any base function blocks", and "The `THIS` pointer
be used to uniquely access variables in a function block, even if a
local variable with the same name exists in a method of the function
block" ([Shadowing Rules](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_shadowing_rules.html)).
Two of a method's own of one name stay E0012. TcOpen names so 5 (`Kill`'s
`_Object2bKilled`, TcoEthercatMasterDevice's `_state` beside `_State`,
`SyncUnits`), and 24 more errors followed from them, `_state.x` read as
the block's. The harness case `codesyshdrsemi` runs one.

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

## codesys-real-conversions: DWORD_TO_REAL and its kin by value, as CODESYS converts them

**The standard.** `DWORD_TO_REAL`, `REAL_TO_DWORD`, `LWORD_TO_LREAL`
and `LREAL_TO_LWORD` are binary transfers (Ed.3 Table 25, Figure 11):
the IEEE 754 bits move, so `DWORD_TO_REAL(DWORD#16#3F800000)` is 1.0
([Conversions](behaviour/conversions.md#bit-strings-a-binary-transfer)).
Without the extension iecf converts them so.

**The extension.** The four convert by value, the DWORD as the UDINT it
counts and the LWORD as the ULINT, as `UDINT_TO_REAL` and
`REAL_TO_UDINT` would: `DWORD_TO_REAL(DWORD#100)` is 100.0,
`REAL_TO_DWORD(2.5)` 2 (half to even), one beyond the bit string
flagged and its statement left undone. The overloaded `TO_REAL` of a
DWORD and the like the same. Each use is warned,
[W0011](errors/W0011.md): the same text means another number in a tool
that follows the standard, so the warning marks where a program carried
over depends on it. It gives standard names another meaning, as
`bit-string-to-bool` does (clause 5 f): a run with it is not a
compliant one.

**Why.** CODESYS and TwinCAT convert them by value, and code written
for them relies on it - with Ed.3's meaning it compiles unchanged and
gives other numbers, silently. The OSCAT BASIC library calls them in 33
objects (`HOUR_TO_TIME := DWORD_TO_TIME(REAL_TO_DWORD(IN * 3600000))`);
its own CHK_REAL says it avoids REAL_TO_DWORD for the bits because it
"treats dword as a number on many systems". Decided 2026-11-19, florin:
the extension in the pack, and the warning.

**To make a program portable.** Write the conversion through the
unsigned integer, which means the same in every tool:
`UDINT_TO_REAL(DWORD_TO_UDINT(d))`, `UDINT_TO_DWORD(REAL_TO_UDINT(r))`.

## pointers: CODESYS's POINTER TO, ADR and SIZEOF, each pointer checked where it is used

**The standard.** Ed.3 has references - `REF_TO`, `REF()`, `^`, NULL
(6.4.4.10, Table 12) - which iecf builds, and no pointers: no
arithmetic on a reference, no address as a number, no SIZEOF. Without
the extension `POINTER TO` is [E0005](errors/E0005.md) and `ADR`,
`SIZEOF` [E0010](errors/E0010.md), each naming the extension.

**The extension.** As CODESYS and TwinCAT have them, each pointer
checked where it is used, so that a mistake is a fault, never C's
undefined behaviour ([Pointers](behaviour/pointers.md)):

- `POINTER TO T` of data - an elementary value, a STRING, an array or a
  structure of them, with no reference, interface, instance or pointer
  in it ([E0027](errors/E0027.md) otherwise); a pointer to any data
  converts to a pointer to any other, as in CODESYS.
- `ADR(x)` of a variable, an element or a field: a pointer into the
  variable `x` is part of, at `x`. `ADR` of a CONSTANT is refused (a
  pointer could write it).
- `p^`, `p^[i]`, `p^.f`: each read and write checked to lie in that
  variable (`IECF_FAULT_INDEX`, `IECF_FAULT_NULL`), the bytes in the
  platform's order, another type through a pointer allowed.
- `p[i]`, the pointer indexed: `(p + i * SIZEOF(T))^`, as CODESYS and
  TwinCAT have it, one index, checked as `p^` is (decided 2026-11-22,
  florin). The harness case `pointerindex` reads, writes, moves and
  overruns one.
- `p + n`, `p - n` in bytes; `p - q`, a DINT, and `<`, `<=`, `>`, `>=`
  of two pointers into one variable (into two, `IECF_FAULT_POINTER`);
  `=` and `<>`, with 0 or NULL too.
- `SIZEOF(x)` of a variable or a type, in bytes as the platform lays it
  out, of the smallest unsigned type that holds it; refused of a
  pointer ([E0016](errors/E0016.md)), whose size is no portable number.
- A pointer that may point to a temporary - a FUNCTION's, a method's, a
  VAR_TEMP, an in-out's - is refused (E0027) where it would outlive the
  call: in a static variable, a block's input, a global, an in-out, an
  output, a FUNCTION's result. Passed down as an argument, it is fine.
- No number is an address: a pointer to or from DWORD, LWORD,
  `__XWORD` or PVOID is refused ([E0013](errors/E0013.md)); so are
  function pointers, and `CheckPointer` means nothing special (iecf
  checks every use itself). `__NEW` and `__DELETE` are their own
  extension,
  [dynamic-memory](#dynamic-memory-codesyss-__new-and-__delete-memory-taken-while-the-program-runs-from-its-tasks-pool-each-pointer-to-it-checked).

A STRING is read and written whole through a pointer, arrays and
structures are copied whole through one, `ADR` of a variable at an
address keeps its image area and byte (found again at each use), and
`ADR(x)` may be an initial value. Not compiled ([E0009](errors/E0009.md),
each where it is written): a WSTRING through a pointer (its 16-bit units
need an alignment no pointer promises), a structure with relative
locations through one (laid out little-endian, not in the platform's
order), `ADR` of a bit.

**Why.** CODESYS and TwinCAT have C-like pointers (CODESYS help,
"POINTER TO", "ADR", "SIZEOF"; Beckhoff InfoSys the same), and code
written for them uses them: 74 objects of the OSCAT BASIC library walk
strings, pass arrays of any length, clear buffers through them. PLCnext
has none, Siemens VARIANT and REF_TO. Decided 2026-11-19, florin:
build them, checked ("there may be some programmers wanting those"),
with the recommendations of [Pointers: the proposal](design/pointers.md).

**To make a program portable.** For an array of any length, Ed.3's
`ARRAY[*]` input (6.5.3), which iecf builds; for a variable reached
indirectly, `REF_TO` and `REF()`; for the bytes of a value, the
conversions and partial access (`d.%B0`).

## date-time-arithmetic: DATE + TIME and DATE - TIME, the TIME a whole number of days

**The standard.** Table 35 adds and subtracts a TIME to and from a TOD,
a DT and their long forms (ADD_TOD_TIME, ADD_DT_TIME, ...), not a DATE:
`d + T#1d` is [E0014](errors/E0014.md), naming the extension.

**The extension.** `DATE + TIME` and `DATE - TIME` give a DATE, moved by
the TIME, which must be **whole days** (decided 2026-11-19, florin):
- a literal of no whole day, `d + T#12h`, is [E0020](errors/E0020.md);
- a variable's, as the program runs, is flagged `IECF_FAULT_CONVERSION`
  and its statement left undone, as a real too big for its integer is;
- past the DATE range the date wraps in whole days, 1970-01-01 to
  2106-02-07, and is flagged `IECF_FAULT_OVERFLOW`, so it stays a
  midnight.

Only a DATE first and a TIME second, as Table 35's forms are; not with
LTIME, not `TIME + DATE`
([dates](behaviour/dates.md#date--time-only-with-an-extension)).

**Why.** No vendor documents it: CODESYS's and TwinCAT's ADD and SUB
list TOD and DT with TIME, not DATE, nor do PLCnext's and Siemens's.
Code written for CODESYS 2.3 has it - OSCAT BUILDING's TIMER_2,
`DAY_OF_MONTH(dat + t#1d)` - and a day added to a date is a common
need; iecf's reading keeps it exact.

**To make a program portable.** Through DT: `DT_TO_DATE(DATE_TO_DT(d) +
T#1d)` - or CONCAT_DATE of the parts.

## mixed-arguments: a call's arguments in order first, then by name, as newer CODESYS takes them

**The standard.** A call gives its arguments all by name or all in
order (Ed.3 6.6.1.4): `F(x, 6, D := '.')` is [E0016](errors/E0016.md),
naming the extension.

**The extension.** A call of a FUNCTION, a standard function, a function
block instance or a method may give its first arguments in order and
the rest by name (decided 2026-11-19, florin):
- those in order go to the first parameters as declared, EN and ENO
  apart, an output among them taken as `=>` takes it;
- those by name follow every one in order: `F(b := 2, 1)` is E0016;
- a parameter given in order and again by name is passed twice, E0016:
  `F(1, a := 2)`;
- no more in order than the callee has parameters, E0016;
- any input left out is as in a call by name: its default, or
  [E0016](errors/E0016.md) for an in-out.

A program's connections stay by name. The harness case `codesysargs`
runs a FUNCTION, an output, a block and LIMIT called so
([typed functions](behaviour/typed-functions.md#in-order-then-by-name-only-with-an-extension)).

**Why.** The vendors disagree. TwinCAT forbids it: "In TwinCAT 3, you
cannot mix explicit and implicit parameter assignments in function
calls" ([Beckhoff](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2530284939.html)).
CODESYS's function page says the same
([CODESYS](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_obj_function.html)),
but its current error C0043 page gives `Test(iPar2 := 2, 5)` as the
error and `Test(5, iPar2 := 2)` as the correction
([C0043](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_error_c0043.html)):
in order first, then by name. iecf takes that newer reading. Code
written for CODESYS has it: OSCAT NETWORK's `REAL_TO_STRF(LATITUDE, 6,
D := '.')`. I found nothing on it in PLCnext's or Siemens's
documentation.

**To make a program portable.** Name every argument, or give every one
in order.

## chained-assignment: a := b := 0, assigned right to left, as CODESYS, TwinCAT and Siemens chain them

**The standard.** An assignment is a statement, not a value (Ed.3
7.3.2, Table 72): `a := b := 0;` is [E0005](errors/E0005.md), naming the
extension.

**The extension.** `a := b := c := v;` is read right to left as
`c := v; b := c; a := b;` (decided 2026-11-19, florin):
- each inner target is assigned, then read back, so each assignment
  has its own target's type: with `x : INT`, `r := x := 7 / 2` gives `x`
  3 and `r` 3.0; `y := s := 300` with `s : SINT` is
  [E0020](errors/E0020.md), as `s := 300` is;
- each step is checked as the assignment it is: a REAL into an INT is
  refused as it is on its own, as CODESYS refuses `int_var := real_var1
  := int_var`;
- an inner target with a call in it, `a := arr[f()] := 1`, is E0005:
  reading it back would call again;
- each step is a statement of its own: one that faults stops the task
  with the ones before it done; one left undone (a REAL too big for its
  integer) stores nothing, and the next reads back the target as it
  was ([statements](behaviour/statements.md#chained-assignment-only-with-an-extension)).

The harness case `codesyschain` runs these.

**Why.** CODESYS ("CODESYS permits the use of assignments as
expressions", `int_var1 := int_var2 := int_var3 + 9;`,
[CODESYS](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_exst_operator_expression.html))
and TwinCAT ([Beckhoff](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2528264843.html))
have it as ExST; Siemens has "multiple assignment" from V14
(`#statFillLevel := #statTemperature := #tempTemperature := 0.0;`,
Programming Guideline S7-1200/1500 V1.6, 2.9.5). I found nothing on it
in PLCnext's documentation. Code written for CODESYS has it: OSCAT
BUILDING's BOILER. CODESYS's S= and R= in a chain are not processed
right to left; iecf has neither.

**To make a program portable.** One assignment per target: `b := 0; a
:= b;`.

## jump-labels: JMP and labels in ST, as CODESYS and TwinCAT write them, never into a loop or branch

**The standard.** JMP and labels are Instruction List's (Ed.3 7.2), not
Structured Text's (7.3), which has IF, the loops, EXIT and RETURN
instead: `JMP again;` and `again:` in ST are [E0005](errors/E0005.md),
naming the extension.

**The extension.** `name:` marks the statement after it, and `JMP name;`
goes there (decided 2026-11-19, florin):
- a JMP goes to a label of its own statement list or of one around it,
  in the same body (a POU's, an action's): backward or forward, out of a
  loop, an IF or a CASE, never into one it is not in
  ([E0021](errors/E0021.md)), as Siemens's SCL rules it;
- a label is declared once in its body ([E0012](errors/E0012.md)), a
  JMP to none is [E0010](errors/E0010.md); labels have their own names,
  apart from variables', case not counting;
- in a CASE's statements, `name:` is a case label, as it is without the
  extension; a label there goes inside an IF or a loop of that arm;
- a JMP out of a FOR leaves its control variable as the body has it, as
  EXIT does.

The generated C is a `goto` and a C label. Whatever iecf had proven of a
variable's range before a label (to leave an index unchecked) is
forgotten there, as a jump may bring other values. The harness case
`codesysjump` runs these, an index out of range after a jump back
among them.

**Why.** CODESYS ("The `JMP` statement is used for executing an
unconditional jump to a program line that is marked by a jump label",
[CODESYS](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_st_instruction_jmp.html))
and TwinCAT ([Beckhoff](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2528307851.html))
have it in ST; Siemens's SCL spells it GOTO, the label once in its
block: "A jump from the 'outside' into a program loop is not
permitted, but a jump from a loop to the 'outside' is possible"
([Siemens](https://docs.tia.siemens.cloud/r/en-us/v21/scl-s7-1200-s7-1500-s7-1200-g2/program-control-operations-s7-1200-s7-1500-s7-1200-g2/goto-jump-s7-1200-s7-1500-s7-1200-g2)).
PLCnext's ST has none
([PLCnext](https://engineer.plcnext.help/latest/ST_LanguageElements.htm)).
Code written for CODESYS has it: OSCAT NETWORK's IP_CONTROL2.

**To make a program portable.** A loop with EXIT, or IF around what a
jump skips.

## optional-semicolons: no ';' after END_IF, END_CASE and the other ends, as CODESYS and TwinCAT take it

**The standard.** Every statement ends with `;`, the structured ones
too: `END_IF;`, `END_CASE;`, `END_FOR;`, `END_WHILE;`, `END_REPEAT;`
(Ed.3 Annex A, Stmt_List). Without it, [E0005](errors/E0005.md), naming
the extension.

**The extension.** The `;` after END_IF, END_CASE, END_FOR, END_WHILE
and END_REPEAT may be left out (decided 2026-11-20, florin). Each of
those keywords ends its statement, so the next one reads the same with
the `;` or without it. Any other statement keeps its `;`: `x := 1`
before the next line is still E0005, as is END_STRUCT's (that one is
[struct-semicolon](#struct-semicolon-end_struct-with-no--after-it-as-codesys-and-twincat-write-a-structure)).
The harness case `codesyssemi` runs each of the five.

**Why.** Code written for TwinCAT leaves it out more often than not:
TcOpen, a TwinCAT 3 framework, has 10639 of these ends without `;`
against 829 with it, in 231 of its 1055 objects; OSCAT NETWORK and
BUILDING have a few. CODESYS and TwinCAT compile it; neither documents
it - Beckhoff's own examples write the `;`, and an answer on the
CODESYS forum calls it "needless" after END_IF
([forum](https://forge.codesys.com/forge/talk/Engineering/thread/65be32a57d/)).

**To make a program portable.** Write the `;`.

## codesys-oo: CODESYS's object orientation: THIS^ and SUPER^, overriding without OVERRIDE, methods PUBLIC unless written

**The standard.** `THIS` and `SUPER` are the instance and its base
(`THIS.x`, `SUPER.m()`, `SUPER()` for the base's body; Ed.3 6.6.5.7,
6.6.7.2.9); a method that replaces its base's is written OVERRIDE
(6.6.5.5.3); a method with no access specifier is PROTECTED (6.6.5.9).
iecf holds to all three ([objects](behaviour/objects.md), OO.md D1 and
D6): `THIS^` is [E0014](errors/E0014.md), a method of its base's name
without OVERRIDE [E0031](errors/E0031.md), calling one with no specifier
from outside [E0030](errors/E0030.md) - each naming the extension.

**The extension.** CODESYS's spellings are read as Ed.3's, before
anything is checked (decided 2026-11-20, florin):
- `THIS^` and `SUPER^` are `THIS` and `SUPER`: `THIS^.x`, `THIS^.m()`,
  `SUPER^.m()`, and `SUPER^()`, the base's body; in a variable's
  initial value too, `d : S := (Parent := THIS^)`, an interface given
  the instance (added 2026-11-22, florin: "go"; TcOpen gives its
  structures their parent so, 6 times). The instance's type is set
  before any of its values, so through that interface a derived
  block's methods run (docs/c-output.md); the harness case
  `codesysthisinit` reads it back. `SUPER^` there is `SUPER`, which on
  its own stays E0031;
- a method with no access specifier is PUBLIC; one written keeps it;
- a method of a FUNCTION_BLOCK or CLASS with the name of a method of a
  base, at any depth, not PRIVATE there, overrides it, OVERRIDE written
  or not - with all an override's rules: the same signature, not a FINAL
  one;
- a method of the block or of a base called bare in the block's body or
  methods, `Step(...)`, is `THIS.Step(...)` (OO.md D3), unless a variable
  there has its name - dynamically bound as THIS's calls are (added
  2026-11-20, florin: "go"; TcOpen calls so about 1100 times);
- an interface compared with 0, `itf <> 0` or `itf = 0`, is compared
  with NULL: CODESYS's interfaces are pointers, and Beckhoff tests one
  so, "IF iSample <> 0 THEN"
  ([__ISVALIDREF](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529165707.html))
  (added 2026-11-20, florin: "build ... interface <> 0"; TcOpen tests
  its parent so, `_Parent <> 0 AND_THEN _Parent.Context <> 0`).
- the literal 0 given to an interface is its NULL: `itf := 0`, `VAR itf
  : I := 0; END_VAR`, an input given 0, a FUNCTION's or method's result
  set to 0 - where Ed.3 writes NULL. CODESYS: "Until the first
  assignment, the variable contains the value `0`"
  ([Implementation of an Interface](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_implementing_interface.html))
  (decided 2026-11-23, florin: "go with a"; TcOpen initialises 18
  interfaces so, `_drive : ITcoDriveSimple := 0`, and sets one,
  `THIS^._s.Observer := 0`). Any other number, a typed `INT#0`, or a
  variable holding 0 stays [E0013](errors/E0013.md). The harness case
  `codesysitfzero` gives 0 to one in each way.

- a method or property of an INTERFACE may be written PUBLIC, which it
  is, and FINAL, which has no effect: an implementing type's method
  still overrides its base's (added 2026-11-22, florin: "go"; TcOpen
  writes `METHOD PUBLIC Debug` 16 times, `PROPERTY PUBLIC` twice,
  `METHOD PUBLIC FINAL StepIn` 4 times, 23 errors). CODESYS's interface
  pages give a prototype no specifier
  ([interface method](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_obj_interface_method.html))
  and none says what FINAL means there; TwinCAT compiles both, and no
  effect is iecf's reading. PRIVATE, PROTECTED, INTERNAL, ABSTRACT and
  OVERRIDE there stay E0031. The harness case `codesysitfspecs` calls a
  FINAL one's override.
- an instance of a class or of a block with methods, a base or a
  derived one, is copied with `:=` from one of its own type, and is an
  input, given one of its own type: the bytes copied, as CODESYS's and
  TwinCAT's (added 2026-11-24, florin: "go with recommendations";
  TcOpen copies 10 so, `_content := inoContent`, and takes
  `inSignalInfo : TcoSignalInfo` as an input once). A copy takes all
  the source has, the interfaces and references it holds with it - a
  `TcoObject`'s parent among them. A derived one into a base-typed
  variable or input, which would cut its derived part and keep its
  methods, an output, and an array or a structure holding one stay
  [E0031](errors/E0031.md) (OO.md D11). Neither vendor's
  pages say what the copy does; TwinCAT compiles it. The harness case
  `codesysobjcopy` copies one directly, through a `REFERENCE TO` and
  as an input.
- an ABSTRACT FUNCTION_BLOCK or CLASS needs no ABSTRACT method: ABSTRACT
  says only that it has no instances, where Ed.3 has "An abstract class
  shall contain at least one abstract method" (6.6.5.8.2). TwinCAT:
  "Abstract function blocks can contain abstract and non-abstract
  methods and properties", and "If a function block contains an
  abstract method or property, it must itself be abstract"
  ([ABSTRACT concept](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/6413748235.html))
  - the rule one way only (added 2026-11-24, florin: "go with
  recommendations"; TcOpen has 11, `TcoObject`, `TcoStateBase`, the
  robotics tasks). An instance of one other than a `VAR_IN_OUT` stays
  [E0031](errors/E0031.md). The harness case `codesysabstract` calls a
  derived one's override through a reference to its base.
- a derived block that adds a `VAR_IN_OUT` stands where its base is
  wanted - given to a base-typed `VAR_IN_OUT`, referenced by a
  base-typed `REF_TO` or `REFERENCE TO` - where Ed.3 allows it only "if
  the derived function block type has no additional in-out variables"
  (6.6.8.3, 6.6.8.4). TwinCAT: "An instance of the derived function
  block can be used in any context in which TwinCAT expects a function
  block of the type of the basic function block"
  ([Inheritance principle](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/3537661579.html)).
  A call through the base binds only the base's in-outs; an added one
  keeps what the last call bound (an in-out is held as a pointer in
  the instance, as TwinCAT's), and one never bound is a fault `NULL`
  before the body's statement that uses it, as a NULL reference's
  (OO.md D8) (added 2026-11-24, florin: "go with recommendations";
  TcOpen gives its `TcoCylinderMoveTask`, which adds
  `MoveToPositionSignal`, as a `REFERENCE TO TcoTask` 4 times, and calls
  its methods only). Neither vendor's pages say what such a call
  leaves. The harness case `codesysaddedinout` calls one through its
  base, bound and never bound.

An INTERFACE's methods are otherwise left as they are, and so are VAR
sections: a block's VAR stays internal.
Once rewritten, a program is checked and compiled as Ed.3 has it,
dynamic binding included: a base's method calling `THIS^.m()` reaches
the override. The harness cases `codesysoo` and `codesysitfzero` run
these.

**Why.** CODESYS writes object orientation so and has no OVERRIDE
("PUBLIC or not specified: Access is not restricted"; a derived block
can "define a method with the same name, the same inputs and the same
output",
[method](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_obj_method.html),
[extending](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_extending_function_block.html));
TwinCAT the same. TcOpen, a TwinCAT 3 framework, has 1796 methods, 865
without a specifier, OVERRIDE written nowhere, `THIS^` 2811 times and
`SUPER^` 8429. Siemens SIMATIC AX writes OVERRIDE; PLCnext has no
inheritance (OO.md §2).

**To make a program portable.** `THIS.` and `SUPER.`, OVERRIDE on each
override, an access specifier on each method, and `THIS.m()` for a
method of the own instance.

## internal-reads: a block's internal variable read from outside, as CODESYS and TwinCAT allow, warned

**The standard.** From outside an instance, a function block's inputs
and outputs are reached, `t.Q`, and nothing else: its `VAR` is its own
(Ed.3 6.5.2.2). `n := a.count` is [E0010](errors/E0010.md), naming the
extension.

**The extension.** A FUNCTION_BLOCK's plain `VAR` variable, its own or
a base's, may be **read** through an instance, as an output is, and
each read is warned, [W0012](errors/W0012.md) (decided 2026-11-20,
florin). A method of an internal instance may be called through it,
`a.inner.m()`, the instance read to call it, as TcOpen calls
`THIS^.DriveRef._movementState.Restore()` and TwinCAT compiles it
(florin, 2026-11-20). An internal instance itself may be called through
it too, `a.inner(in := x, out => y)`, its body run with the inputs the
call gives, warned so (decided 2026-11-23, florin: "go"): TcOpen calls
`_inspectorTask._failTimer(IN := TRUE, PT := ...)`, a TON in its task's
VAR, and its tests `_to_A._sut_A()`, and TwinCAT compiles them, though
Beckhoff's and CODESYS's help speak of reading only (it was E0010 until
then). Never written: an assignment to it or to an input of it, `=>`
into it, an in-out argument, `REF()`, `ADR()`, `REF=` of it, stay
E0010. Not a `VAR_TEMP`, a
`VAR_EXTERNAL`, a CLASS's variable (its `VAR PUBLIC` says what is
seen), nor a section with an access specifier (that rules, E0030). The
harness case `codesysreads` reads an integer, an array's element and an
inner instance's output and variable, calls an inner instance's
method, and calls an inner instance itself, directly and two levels in.

**Why.** CODESYS and TwinCAT allow it, reading only: "You can access
local variables for reading from outside the programming objects via
the instance path. Access for writing from outside the programming
object is not possible"
([Beckhoff](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2528755083.html)).
CODESYS's own static analysis flags each such read, rule SA0102: it
"contradicts the principle of data encapsulation (hiding data) and does
not comply with the IEC 61131-3 standard"
([SA0102](https://content.helpme-codesys.com/en/CODESYS%20Static%20Analysis/_san_rule_sa0102.html)),
hence the warning. STruCpp's tests read them.

**To make a program portable.** Declare what is read outside as a
`VAR_OUTPUT`.

## properties: CODESYS's PROPERTY, a GET and a SET called where it is read and assigned

**The standard.** Ed.3 has no properties: a FUNCTION_BLOCK's or CLASS's
data is reached through its inputs, outputs and PUBLIC variables, its
operations through methods. `PROPERTY` is [E0005](errors/E0005.md),
naming the extension.

**The extension.** A FUNCTION_BLOCK, a CLASS or an INTERFACE declares a
property, among its methods, as CODESYS's text form writes it (decided
2026-11-20, florin):

```
PROPERTY PUBLIC Speed : INT
  GET
    Speed := _speed;          (* the property's name is the result *)
  END_GET
  SET
    _speed := MIN(Speed, 100);   (* and here, the value assigned *)
  END_SET
END_PROPERTY
```

- GET and SET are each optional; each may declare `VAR` sections before
  its statements. The specifiers (`PUBLIC`, `ABSTRACT`, ...) are a
  method's, and apply to both.
- They are the methods `PROPERTY_GET_Speed : INT` and
  `PROPERTY_SET_Speed` (its input `Speed`), with everything a method
  has: access, overriding in a derived block, an interface's property
  implemented by a block's, dynamic binding. Messages name them so.
- `x := m.Speed` calls the GET, `m.Speed := v` the SET, through an
  instance, a reference or an interface; inside its block, the bare
  name does the same, unless a variable of that name hides it.
- A part of one is read from its GET's result, `m.P.x`, `m.P[1]`.
- One whose GET gives a `REFERENCE TO`, `REF_TO` or `POINTER TO` is
  written through it, as CODESYS code does (TcOpen's `Status.State.x :=
  ...` of `PROPERTY Status : REFERENCE TO ...`): the GET is called, and
  what it references written.
- One with no SET whose GET gives a `REFERENCE TO` is assigned whole
  the same way, `m.P := v`, and inside its block `P := v`: the GET is
  called and `v` written where it references, as `Ref := value` "writes
  the value `value` to the location where the reference points to"
  ([REFERENCE TO](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_reference.html))
  (decided 2026-11-23, florin: "go"; TcOpen assigns 21 so,
  `_tcoInspectorDialogue.HideOverride := TRUE` of `PROPERTY
  HideOverride : REFERENCE TO BOOL`). A SET, where there is one, is
  called instead; a `REF_TO` or `POINTER TO` is written through its
  `^`, `m.P^ := v`.
- Otherwise a property is no variable: assigning one with no SET or
  reading one with no GET, `=>` into one, an in-out argument, `REF()`,
  or writing a part of one (`m.P.x := 1`) is
  [E0015](errors/E0015.md).
- An INTERFACE's property accessor may hold an empty `VAR ... END_VAR`,
  which declares nothing: TwinCAT's exports write one in an interface's
  GET (added 2026-11-24, florin: "go with recommendations"; TcOpen's
  `ITcoEthercatMasterDevice` 3 times). A prototype has "no temporary
  variables" (Ed.3 6.6.6.3), and TwinCAT: "You cannot declare variables
  within an interface"
  ([Object Interface](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/4256428299.html)):
  one that declares a variable, and an interface method's VAR, stay
  [E0031](errors/E0031.md). The harness case `codesysexprstmt` has one.

The harness case `codesysprop` runs a clamping SET, a read-only
property, one read through an interface, one used inside its block, a
STRING one, one giving a REFERENCE TO a structure, written through, and
one giving a REFERENCE TO INT with no SET, assigned whole.

**Why.** CODESYS and TwinCAT have them
([CODESYS](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_obj_property.html)),
and code written for them uses them: TcOpen, a TwinCAT 3 framework, has
511 in 148 of its 1055 objects. CODESYS keeps each in its own object;
this text form is how STruCpp's tests and TwinCAT's exports write them.
Ed.4's own spelling, `PROPERTY_GET` and `PROPERTY_SET`, is not this one;
iecf reads it when Ed.4 is done.

**To make a program portable.** A METHOD for each accessor, or a
VAR_OUTPUT.

## method-in-outs: a method reaches its block's VAR_IN_OUT, as CODESYS lets it with a warning, warned

**The standard.** A method reaches its block's variables, but not its
VAR_TEMP nor its VAR_IN_OUT (Ed.3 6.6.7.2.3; OO.md): an in-out is bound
by a call of the block, and a method is called on its own.
[E0010](errors/E0010.md), naming the extension.

**The extension.** A method of a FUNCTION_BLOCK reaches its block's
VAR_IN_OUT, its own type's or a base's, and each use is warned,
[W0013](errors/W0013.md) (decided 2026-11-20, florin):
- it is the variable the block's **last call** bound;
- a method called before any call of its block has nothing bound: a
  statement using such an in-out checks it first, and stops with a NULL
  fault if it is not bound (until 2026-11-20 the method checked each
  in-out it uses before its first statement); one a variable's initial
  value uses is still checked before the first statement;
- `__ISVALIDREF(io)`, or `__ISVALIDREF(THIS^.io)`, is TRUE once a call
  of the block has bound `io`, as TcOpen tests one before using it
  (`IF __ISVALIDREF(io) THEN io.x := 1; END_IF` never faults); it is
  no use of `io`, and not warned. Neither CODESYS's nor Beckhoff's pages
  allow it: "The operator __ISVALIDREF can only be used for operands of
  type REFERENCE TO"
  ([Beckhoff](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529165707.html)),
  and an in-out is "always valid" there
  ([REFERENCE](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529458827.html));
  TwinCAT compiles it, and TcOpen does so 547 times (decided 2026-11-20,
  florin: "go"). The block's own body may test one too, its own or a
  base's, as TcOpen's tasks do (TRUE there: the call running the body
  bound it). A FUNCTION's or method's own in-out has no such test;
- a call of the block binding such an in-out - one some method of the
  block uses - to a variable that may be gone after the call, a
  VAR_TEMP, a FUNCTION's or method's variable, or an in-out of the
  caller's own, is [E0027](errors/E0027.md): the method would use it
  after the call.

The harness case `codesysinout` binds two variables in turn, writes a
structure through one, and faults a method of a block never called;
`codesysinoutvalid` tests one with `__ISVALIDREF` before and after the
block's first call, an ELSIF reading it left untested while it is not
bound.

**Why.** CODESYS and TwinCAT allow it with their compiler warning
C0371, "Access to VAR_IN_OUT '&lt;variable&gt;' declared in '&lt;block&gt;' from
external context '&lt;method&gt;'"
([C0371](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_error_c0371.html)).
TcOpen's methods use their blocks' in-outs 1812 times in 47 files.

**To make a program portable.** Give the method the variable as its own
VAR_IN_OUT.

## fb-init: CODESYS's FB_init, given arguments where an instance is declared, called as it is set up

**The standard.** An instance is set up by its type's initial values and
its own, `fb : T := (in := 1)` (Ed.3 6.6.3.3); there is no constructor.
`fb : T(args)` is [E0005](errors/E0005.md), naming the extension; a
method named FB_init is a method like any other.

**The extension.** A FUNCTION_BLOCK's method `FB_init : BOOL`, whose
first inputs are `bInitRetains : BOOL` and `bInCopyCode : BOOL`, is
called as each of its instances is set up (decided 2026-11-20, florin):
- after the instance's own initial values, and after its instances' set-up,
  at the end of the set-up of the PROGRAM or FUNCTION_BLOCK declaring it;
- `bInitRetains` TRUE at a cold start, FALSE at a warm one; `bInCopyCode`
  FALSE (iecf has no online change);
- given the further inputs from its declaration, `fb : T(THIS^,
  eMode.Keep)` in order or `fb : T(parent := THIS^)` by name, worked out
  where it is declared: `THIS` is the instance declaring it;
- the FB_init of each type of its chain that declares one, its base's
  first, each with the arguments it takes, as CODESYS calls them, and
  each as its own, never through the method table;
- a derived type's FB_init has its base's inputs first, of their types
  in their order, and may add some after them - "you can add further
  parameters" (CODESYS); its own names for them are its own, as TwinCAT
  takes TcOpen's `Parent` for its base's `ParentObject` (decided
  2026-11-23, florin: "go"). An argument given by place reaches each
  type's by place, one given by name only the types that have that
  name. Fewer inputs, another type or another order is
  [E0031](errors/E0031.md), saying so;
- a type with no FB_init takes no arguments ([E0016](errors/E0016.md)).

An array of instances whose type has an FB_init, `items : ARRAY[0..2]
OF T(THIS^)` as TcOpen's, has each element's FB_init chain called, in
index order (row by row, the last index the fastest; an array of arrays
so too), the same arguments given each, `THIS` the instance declaring
the array; with no arguments, each element's with the two flags only
(decided 2026-11-23, florin: "go"). The arguments are checked once, for
the array. TwinCAT's arguments for each element, `ARRAY[1..2] OF
T[(k := 4), (k := 7)]`, are not taken yet ([E0009](errors/E0009.md),
saying so): TcOpen does not use them. Before this, such an array was
E0016 with arguments, and taken without a word without them, no
element's FB_init called.

An in-out of such a type, `io : T` in a VAR_IN_OUT of a block or a
method, is bound at each call to the caller's instance, set up where
that is declared: nothing is set up or called for it, and arguments
given it are [E0016](errors/E0016.md) (it was E0009 before 2026-11-23).

Only an instance or array in a PROGRAM's or FUNCTION_BLOCK's `VAR`, one
name to a declaration: one in a VAR_TEMP, a FUNCTION, a method or a
VAR_GLOBAL is not set up so yet ([E0009](errors/E0009.md)); one in a
structure is with [struct-blocks](#struct-blocks-a-structures-function-block-members-set-up-by-their-fb_init-this-the-structure-there-abstract-on-a-structure),
and without it E0009 too (before 2026-11-22 such a member's arguments
were taken without a word and its FB_init never called).
FB_reinit and FB_exit, CODESYS's for an online change, are not called.
The harness case `codesysfbinit` runs a chain of two, `THIS^` given as
an interface, one taking no arguments, and a third level that renames
the inputs it shares and adds one; `fbinitarrays` arrays of one and two
dimensions given `THIS^`, arrays and an array of arrays given nothing,
one in a structure, and in-outs of a block and of a method bound to
elements.

**Why.** CODESYS and TwinCAT set instances up so
([CODESYS](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_method_fb_init_fb_reinit.html),
[Beckhoff](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/5094414603.html)),
and code written for them relies on it: TcOpen gives 887 instances
arguments, 463 of them `THIS^`, its blocks' parent, and has 69 FB_init
methods.

**To make a program portable.** An input set by the instance's initial
value, `fb : T := (parent := ...)`, and a call of an `Init` method from
the first cycle.

## struct-extends: TYPE X EXTENDS Base : STRUCT, the base's fields first, as CODESYS extends a structure

**The standard.** A structure is its own fields (Ed.3 6.4.4.6); one
reuses another by a field of its type. `TYPE X EXTENDS Base : STRUCT` is
[E0005](errors/E0005.md), naming the extension.

**The extension.** A structure may extend another, `TYPE X EXTENDS Base
: STRUCT ... END_STRUCT`: its fields are its base's, at any depth, the
farthest base's first, then its own, each with its initial value
(decided 2026-11-20, florin: "go"). It is then a structure as any other:
a field of a base's name again is [E0012](errors/E0012.md), a base that
is no structure (or itself) [E0011](errors/E0011.md). An X is not a Base
for assignment or as an input: copy the fields. With struct-blocks an X binds a `REFERENCE TO Base`. The harness case
`codesysstructext` runs a chain of three.

An X is given to a `VAR_IN_OUT` of its base's type, at any depth: a
block's, a FUNCTION's, a method's, as a derived instance is given to
an in-out of its base's (Ed.3 6.6.8). The in-out is X's part of that
type, the fields it inherited; what is written through it is X's
(decided 2026-11-23, florin: "B"). A base given to an X's in-out, or a
structure of the same fields not declared to extend it, stays
[E0013](errors/E0013.md). In C an X's structure starts with its
base's, `T_<BASE> base;`, its inherited fields reached through it,
`x.base.V_A`, so a pointer to it is one to its base (C99 6.7.2.1), as
a derived block's is; a structure with fields at locations, or OVERLAP,
stays laid out field by field. The harness case `codesysstructinout`
gives a structure three deep to a block's in-out, kept for a method
to write through, and to a FUNCTION's of its middle type, an element
of an array of them too.

CODESYS's and Beckhoff's pages say what an extended structure has, not
how it is given: "In addition to its own members, the extended
structure also has the same structure members as the base structure"
([Structure](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_structure.html));
"Like function blocks, structures can be extended. The structure then
obtains the variables of the basic structure in addition to its own
variables" ([Extending a structure](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/3468091787.html)).
TwinCAT compiles TcOpen's TcoTixonFeeding, which gives its steps'
Config, Control and Status so.

**Why.** CODESYS and TwinCAT extend structures so, and their generated
I/O types use it: TcOpen has 282 such types, and the I/O types built
on them gave about a thousand of its errors; its in-outs given an
extending structure, 30.

**To make a program portable.** A field of the base's type, `base :
Base;`, reached as `x.base.a`, and given to an in-out as `x.base`.

## var-stat: CODESYS's VAR_STAT, static variables, one of each for the POU however many instances it has

**The standard.** Ed.3 has no static variables. A variable that one
POU's every instance, or every call of a FUNCTION or method, shares is
a global variable, declared `VAR_GLOBAL` and used through
`VAR_EXTERNAL` (6.5.2.2). A `VAR_STAT` section is
[E0005](errors/E0005.md), naming the extension. `VAR_STAT` is no
keyword of the standard: where no section can begin it stays a name.

**The extension.** `VAR_STAT ... END_VAR` in a FUNCTION, a function
block, a class, a PROGRAM or a method declares variables that exist
once for that POU, however many instances it has, and keep their
values from one call to the next (decided 2026-11-20, florin: "go").
Each is written, before the check, as a global variable named by its
POU's whole path, `T.x` (a method's `T.M.x`), set up as any global is
when the program starts, and declared `VAR_EXTERNAL` in its POU, whose
uses of `x` are then that global's. A block extending T reaches T's as
it reaches any of T's variables; a method's static variables are its
own. CONSTANT and RETAIN on the section stay on the global. Messages
and the symbol table name the variable `T.x`. A static variable named
as another of its POU's variables is [E0012](errors/E0012.md). It is
reached only from inside its POU: `inst.x` and `T.x` from outside are
not. The harness case `codesysvarstat` runs one in a block shared by
two instances, one in a method, one in a FUNCTION, and a derived
block reading its base's.

**The vendors.** CODESYS: "static variables retain their values when
the application leaves the block", an "extension of the IEC 61131-3
standard" ([VAR_STAT](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_vartypes_var_stat.html)).
Beckhoff TwinCAT: "Static variables only exist once. This also applies
to static variables of a function block or a function block method,
even if the function block is instantiated multiple times"
([Static Variables - VAR_STAT](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2528787339.html)).
When they are first set is told two ways: on download in CODESYS's
own pages, at the block's first call in others; iecf sets them when
the program starts, as a global.

**Why.** TcOpen's TcoMessenger, TcoSignalInfo and its CRC blocks
declare them; TcoMessenger, through which every TcOpen message goes,
was not read at all without it.

**To make a program portable.** Declare the variable `VAR_GLOBAL`,
named for its POU, and `VAR_EXTERNAL` in the POU.

## short-circuit: AND_THEN and OR_ELSE, the right operand evaluated only when the left does not decide

**The standard.** Ed.3's AND and OR evaluate both operands; the order
of evaluation is left to right (7.3.2), and nothing is skipped. To test
one thing before another an IF tests the first. `AND_THEN` and
`OR_ELSE` are names to the standard: written between two operands they
are [E0005](errors/E0005.md), naming the extension.

**The extension.** `a AND_THEN b` is TRUE when both are, `b` evaluated
only when `a` is TRUE; `a OR_ELSE b` is TRUE when either is, `b`
evaluated only when `a` is FALSE (decided 2026-11-20, florin: "build
AND_THEN/OR_ELSE"). They take BOOLs, and rank as AND and OR do. What `b`
does is not done when it is skipped: a call is not made, a division not
faulted, a NULL not dereferenced - `itf <> NULL AND_THEN itf.Ready()`,
`d <> 0 AND_THEN x / d > 1`. Anywhere an expression is: an assignment,
an argument, an IF's or ELSIF's condition, a WHILE's. The harness case
`codesysshortcircuit` counts a method's calls through each.

In the C, `&&` and `||`; when `b` calls a method or a FUNCTION with
effects, which iecf writes on lines before the statement, those lines
go inside `if (a)` and the value in a temporary.

**The vendors.** CODESYS: "The right-hand operands are executed only if
the left-hand operand is TRUE", an "extension of the IEC 61131-3
standard", for BOOL and BIT operands in ST
([AND_THEN](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_and_then.html),
[OR_ELSE](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_or_else.html)).
Beckhoff TwinCAT the same
([AND_THEN](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2528923787.html)).

**Why.** TcOpen tests a reference or an interface before using it,
`_Parent <> 0 AND_THEN _Parent.Context <> 0`, and calls a step's
action only once the one before is done: 32 times in 15 files,
TcoMessenger and TcoObject among them.

**To make a program portable.** Nest IFs: `IF a THEN IF b THEN ...`,
or set a BOOL in them.

## type-access: TYPE INTERNAL and TYPE PUBLIC, an INTERNAL type seen only in its namespace, as CODESYS's in its library

**The standard.** A namespace's FUNCTION, block, class or interface may
be INTERNAL, seen only inside the namespace (Ed.3 6.9.2), but a type
has no place for an access specifier: every type is public. `TYPE
INTERNAL X` and `TYPE PUBLIC X` are [E0005](errors/E0005.md), naming the
extension, and X is public.

**The extension.** `TYPE INTERNAL X : ...` makes the types of that TYPE
section INTERNAL as a POU is: seen inside its namespace and those in
it, and anywhere else [E0028](errors/E0028.md). `TYPE PUBLIC` is what a
type is anyway (decided 2026-11-20, florin: "build ... TYPE
INTERNAL"). A library given with `--library` is a namespace, so an
INTERNAL type of it is the library's own, as in CODESYS; outside any
namespace INTERNAL changes nothing, as for a POU.

**The vendors.** CODESYS and TwinCAT write an access specifier on a
data type in a library, INTERNAL keeping it to the library. TcOpen's
TcoCore declares `TYPE INTERNAL eMessageCondition`, which its
TcoMessenger uses; without the extension iecf read no type at all
there.

**To make a program portable.** Leave the word out: the type is then
public.

## var-inst: CODESYS's VAR_INST, a method's variables kept in its block's instance from call to call

**The standard.** A method's variables last for one call (Ed.3
6.6.5.4); what it keeps from call to call is a variable of its block. A
`VAR_INST` section is [E0005](errors/E0005.md), naming the extension.
`VAR_INST` is no keyword of the standard: where no section can begin it
stays a name.

**The extension.** `VAR_INST ... END_VAR` in a method of a
FUNCTION_BLOCK or CLASS, or in a property's GET or SET, declares
variables of the method kept in its instance: one for each instance,
set up once with it, its initial value then, and not reset when the
method is called again (decided 2026-11-20, florin: "go"). Each is
written, before the check, as a VAR of the block named by the block's
and method's path, `T.M.x` (in the C `T__M__X`), and the method's uses
of `x` are that variable's; a block extending T has it as it has T's
other variables. Messages and the symbol table name it `T.M.x`. One
named as another of the method's variables is
[E0012](errors/E0012.md); a VAR_INST anywhere but in a method is E0005.
The harness case `codesysvarinst` keeps one in a method and one in a
property's GET, for two instances and a derived block.

**The vendors.** CODESYS: "The `VAR_INST` instance variables are allowed
only in methods", kept "in the stack of the function block instance",
and "not reinitialized each time the method is called"
([VAR_INST](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_vartypes_var_inst.html)).
It says nothing of a property's GET or SET, which are methods; TcOpen
uses one in a GET, and TwinCAT compiles it.

**Why.** TcOpen declares them 17 times in 11 files; its TcoMessenger's
StringBuilder GET keeps the cycle it was last read in, and the whole
messenger was not read without it.

**To make a program portable.** Declare the variable as a VAR of the
block, named for its method.

## any-input: CODESYS's ANY as an input: a value of any type, given as its type class, address and size

**The standard.** ANY is a generic type of the standard functions
(Ed.3 Figure 5, 6.4.2): no user's POU declares a variable of it, and
`x : ANY` is [E0011](errors/E0011.md), naming the extension.

**The extension.** A FUNCTION's, method's or FUNCTION_BLOCK's
`VAR_INPUT x : ANY` takes a value of any type, and is, as CODESYS
passes it, a structure (decided 2026-11-20, florin: "go"):

```
x.TypeClass : __SYSTEM.TYPE_CLASS;   (* what the value is *)
x.pValue    : POINTER TO BYTE;       (* where it is *)
x.diSize    : DINT;                  (* how many bytes *)
```

A variable given is pointed to where it is, so writing through `pValue`
writes it; any other value - a literal, an expression - is put in a
temporary of its type first (an integer literal an INT, or a DINT or
LINT where it needs one; a real literal an LREAL). `__SYSTEM.TYPE_CLASS`
is an enumeration of `TYPE_BOOL`, `TYPE_BYTE`, ... `TYPE_STRING`,
`TYPE_WSTRING`, `TYPE_TIME`, `TYPE_DATEANDTIME`, `TYPE_TIMEOFDAY`, ...,
in the order CODESYS's runtime lists them as far as known; a program
compares them by name, and their numbers are iecf's. A structure, a
block or an array given is `TYPE_USERDEF` or `TYPE_ARRAY`, a pointer
`TYPE_POINTER`. Reading the value means the pointers extension (`p :=
x.pValue; p^`), which the `codesys` pack has. Only an input is ANY; the
narrower generic types (ANY_INT, ANY_NUM, ...) stay the standard
functions'. The harness case `codesysany` reads and writes through one.

**The vendors.** CODESYS: "The input variables (`VAR_INPUT`) have a
generic data type", passed as a structure of `typeclass :
__SYSTEM.TYPE_CLASS`, `pvalue : POINTER TO BYTE` and `diSize : DINT`
([ANY](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_any.html)).
TcOpen spells the fields `TypeClass`, `pValue` and `diSize`, as CODESYS
compares names, ignoring case. No page found gives TYPE_CLASS's
numbers.

**Why.** TcOpen's StringBuilder appends a value of any type
(`AnyToString`, `AppendAny`), and TcoMessenger passes its messages'
arguments through it.

**To make a program portable.** One FUNCTION for each type it takes.

## type-constants: a block's VAR CONSTANT read through its type's name, T.C, as TwinCAT reads one

**The standard.** A function block's variables are reached through an
instance (Ed.3 6.6.3); its type's name is no value. `T.C` naming a
block type T is [E0010](errors/E0010.md) or, where a constant is
wanted, [E0032](errors/E0032.md).

**The extension.** `T.C`, C a `VAR CONSTANT` of the FUNCTION_BLOCK or
CLASS T with an initial value, is that value, typed as C is (decided
2026-11-20, florin: "go"): anywhere a constant may be - a STRING's
length, an array's bounds, an initial value, a body. Written so before
the check: a value naming another of T's constants reads it the same
way, `TWICE : INT := N * 2`. A library's type is named by its short
name too, when one library has it (`TcoMessengerEnv.MAX` for
`TcoCore.TcoMessengerEnv.MAX`). Only constants: a VAR, an input or an
output still takes an instance. A POU declaring a variable named T
means its variable. The harness case `codesystypeconst` sizes a STRING
and an array by them.

**The vendors.** The CODESYS help documents constants read through an
instance ([constants](https://help.plc.abb.com/_cds_variable_constant.html));
a third-party article reports that CODESYS refuses `FBType.Member`
([source](https://industrialmonitordirect.com/blogs/knowledgebase/codesys-v3-resolving-external-var-stat-constant-access)).
TwinCAT takes it: TcOpen, which builds there, sizes its messages so.

**Why.** TcOpen's TcoMessenger declares its strings
`STRING(TcoMessengerEnv.MAX_MESSAGE_TEXT_LENGHT)`, 26 times.

**To make a program portable.** Put the constant in a `VAR_GLOBAL
CONSTANT`.

## union: CODESYS's and TwinCAT's UNION, its members sharing their bytes

**The standard.** Ed.3 has no UNION. Its way to let components share
memory is a structure with relative locations and OVERLAP (6.4.4.7):
`STRUCT OVERLAP r AT %B0 : REAL; d AT %B0 : DWORD; END_STRUCT`, which
iecf compiles (docs/c-output.md §4). `STRUCT OVERLAP` without relative
locations says nothing of which components share which bytes, and is
refused ([E0034](errors/E0034.md); decided 2026-11-13, florin: for
now). `UNION ... END_UNION` is [E0005](errors/E0005.md), naming the
extension.

**The extension.** `TYPE U : UNION ... END_UNION` (decided 2026-11-20,
florin: "go"):

```
TYPE U :
UNION
    r : REAL;
    d : DWORD;
    w : WORD;
END_UNION
END_TYPE
```

is read, before the check, as a `STRUCT OVERLAP` whose every member is
AT `%B0`: its size the largest member's, its bytes laid out as any
structure with relative locations is, the same in C memory and at an
address (docs/c-output.md §4). A variable of it is initialised through
one member, `v : U := (r := 1.5);`, as CODESYS writes it; without one,
the first member's type gives the bytes it covers, as for an OVERLAP.
A member's initial value in the type is allowed on one member at most
([E0034](errors/E0034.md)): CODESYS's pages allow one and say nothing
of several. The extension does not read a bare `STRUCT OVERLAP`
without relative locations as a union: give each member `AT %B0`
instead. The harness case `codesysunion` reads and writes one through
each of its members.

**The vendors.** CODESYS: "The members of the instance of a UNION data
type all reserve the same memory space. All members have the same
address offset and share the memory"; "The memory requirement is
determined by the largest member". TwinCAT: every member starts at the
same address, "and therefore co-written". PLCnext Engineer plans
relative addressing of structure elements through UNION types. Siemens
has no UNION; SCL overlays a variable with `AT` in a declaration
instead. The reference compiler (matiec) has neither.

**Why.** TcOpen declares UNION twice: `floatUnion`, a REAL read back as
its four bytes, in TcoUtilities' own conversion tests, and (credited
there to TcUnit) `F_AnyToUnionValue`, a member for every elementary
type, read from an ANY's bytes by its TypeClass (`Tc2_System.MEMCPY`)
23 times in a prototype of a task's result
(POUs/Prototypes/TcoTask/TaskResult) - the same TypeClass the any-input
extension gives an ANY.

**To make a program portable.** Write the union as `STRUCT OVERLAP`,
each member `AT %B0`; that is IEC 61131-3 and compiles without the
extension.

## action-call: an action called as `fb.act()`

**The standard.** An ACTION is run by the steps it is associated with,
under its qualifier (Ed.3 6.7.4); the grammar's Invocation names a
function block instance or a method, no action. `fb.act()` and, inside
the POU, bare `act()` are [E0016](errors/E0016.md), naming the action.

**The extension.** An action of a FUNCTION_BLOCK or PROGRAM called as a
statement, from inside by its name and from outside through the
instance (decided 2026-11-20, florin: "go"), runs once, against the
instance's own variables - the same it would run against if a step did.
It takes no arguments and gives no value. Refused still: an action
whose own body is a chart (Ed.3 6.7.4.1), which only a step runs, never
a single call; and one a step also runs, through its action
association - its own control (N, S, R, the timed qualifiers) and a
direct call would both decide when it runs. A METHOD without inputs or
a result does the same in the standard's own terms; the two may be
mixed, one action called and another left to its step, in the one POU.
The harness case `codesysactioncall` runs one bare, one through an
instance, and keeps a third to its step, which a direct call of it
still refuses.

**The vendors.** CODESYS and TwinCAT let a POU have actions without any
SFC, as named pieces of its body sharing its variables, and call them
`fb.act();` or, inside the POU, `act();`.

**To make a program portable.** Write the action as a METHOD without
inputs or a result; calling it is then Ed.3's own (6.6.5.4).

## recursion: a FUNCTION, block or method that calls itself, with a depth limit

**The standard.** "The recursive call of POUs and methods is
Implementer specific" (Ed.3 6.6.1). iecf refuses it
([E0016](errors/E0016.md)): a FUNCTION, a function block, a PROGRAM or
a method that can reach itself through calls, directly or through
others, a call through an instance, THIS, an in-out or an interface
counting as a call of every override or implementation that could run
there (decided 2026-11-12, florin; [Implementer
choices](dependencies.md)). Its stack would have no bound, and an
overflow of C's stack is no fault the program could report: it is
undefined behaviour, which the output never has (docs/c-output.md §1).
An ACTION is a node of its own in that graph, reached from its POU and
from a direct call of it (action-call), so two actions calling each
other are a cycle too.

**The extension.** Each call of a recursive POU, method or action - one
that can reach itself - is counted: one deeper in the context's
`depth` as it begins, one less as it ends. A call past
`IECF_DEPTH_MAX` (64 unless the platform defines its own, `iecf.h`) is
a fault, `IECF_FAULT_DEPTH` (14), which stops the task as an index out
of range does, rather than overflowing the stack; the platform sizes
its task's stack for the limit. Inside a FUNCTION, `F(n - 1)` calls F,
while its name alone is still its result; a method calls itself as
`THIS.m(...)`. Only what can reach itself is counted - the C function
renamed `<name>_r`, and `<name>` counting around it
(docs/c-output.md §5) - and a program with no recursive call is built
exactly as without the extension. The harness case `recursion` runs a
FUNCTION, a method and a block's body calling themselves, and one that
never ends, stopped by the limit.

**The vendors.** CODESYS and TwinCAT refuse a FUNCTION that calls itself
(CODESYS error C0224, "Call Recursion") and allow a method to, with a
warning that the stack's use cannot be worked out (C0298), silenced by
`{attribute 'estimated-stack-usage' := '...'}`. PLCnext Engineer
refuses every recursive call of a POU (SEM1030, "Recursive dependency
found in program organization unit") and of a method (SEM1259). Siemens
documents no rule; its CPUs stop when the block nesting depth is
exceeded. The reference compiler (matiec) can express none: a POU is
known only after its declaration. Not in the `codesys` pack: it takes a
FUNCTION's recursion too, which CODESYS refuses.

**To make a program portable.** Write the recursion as a loop, with an
array as its stack where it needs one.

## any-to: `ANY_TO_<type>(x)`, CODESYS's and TwinCAT's name for the conversion `TO_<type>(x)`

**The standard.** Ed.3 converts with `*_TO_**` named by both types and
with the overloaded `TO_**`, named by the result's alone (Table 22,
features 1a and 1b): `TO_REAL(d)`. It has no `ANY_TO_**`; `ANY_TO_REAL`
is [E0010](errors/E0010.md), naming `TO_REAL` and the extension.

**The extension.** `ANY_TO_<type>(x)` is `TO_<type>(x)`, written so
before anything is checked (decided 2026-11-21, florin: "research then
fix!"): what `TO_<type>` takes, gives and refuses, `ANY_TO_<type>`
does, the codesys pack's conversions included (`ANY_TO_BOOL` of a
BYTE, bit-string-to-bool). A FUNCTION the project declares with such a
name is called as it is. The harness case `codesysanyto` converts to
REAL, INT, STRING and BOOL.

**The vendors.** Schneider's Machine Expert, built on CODESYS, lists
"ANY_TO_<any data type>" among its conversions
([ANY_..._TO Conversions](https://product-help.schneider-electric.com/Machine%20Expert/V2.1/en/SoMProg/SoMProg/D-SE-0083736.html));
the CODESYS help page itself refused to be fetched. On the CODESYS
forum a user looking for a function that would decode an `ANY` input
was pointed to `TO_STRING`
([thread](https://forge.codesys.com/forge/talk/Engineering/thread/953c482972/)):
an `ANY` input is decoded by its type class (any-input), not by
`ANY_TO_STRING`.

**Why.** TcOpen calls `ANY_TO_REAL`, `ANY_TO_DINT` (TcoBytesToReal,
TcoRealToBytes) and `ANY_TO_STRING` (StringBuilder).

**To make a program portable.** Write `TO_<type>`.

## var-info: `__VARINFO(x)`, CODESYS's and TwinCAT's information on a variable: its name, type and size

**The standard.** Ed.3 has no way for a program to ask what it knows
of a variable: its name, its type, its size. `__VARINFO` is
[E0010](errors/E0010.md), naming the extension, and a name starting
with two underscores is [E0005](errors/E0005.md) (Ed.3 6.1.2).

**The extension.** `info := __VARINFO(x)` fills a `__SYSTEM.VAR_INFO`
with what is known of `x`: a variable, a field, an element or a bit of
one, not an expression (decided 2026-11-21, florin: "go with
__VARINFO"). The structure is CODESYS's and TwinCAT's, member by
member:

| member | type | what iecf gives |
|---|---|---|
| `Symbol` | `STRING(39)` | `x`'s name as written: a field's own (`io.Sig.Ready`: `Ready`), an element's or a bit's its array's or bit string's |
| `TypeName` | `STRING(79)` | `x`'s type as iecf writes it in a diagnostic: `INT`, `ARRAY[1..3] OF INT`, `STRING[20]`, a structure's or block's name |
| `TypeClass` | `__SYSTEM.TYPE_CLASS` | its class, as any-input gives it: `TYPE_INT`, `TYPE_ARRAY`, `TYPE_USERDEF` for a structure or an instance |
| `BitSize` | `INT` | its size in bits on the platform, `SIZEOF(x) * 8`; 1 for a bit |
| `BitNo` | `INT` | the bit's number for `x.5`, else 0 |
| `NumElements` | `UDINT` | an array's elements, all dimensions; 0 for anything else |
| `BaseTypeClass` | `__SYSTEM.TYPE_CLASS` | an array's elements' class |
| `ElemBitSize` | `UDINT` | an array's element's size in bits; 0 for anything else |
| `Comment` | `STRING(79)` | `''`: iecf keeps no comments |
| `MemoryArea` | `__SYSTEM.MEMORY_AREA` | `MEM_MEMORY`, CODESYS's initial value |
| `ByteAddress`, `ByteOffset`, `Area`, `BitAddress` | `DWORD`, `DWORD`, `DINT`, `UDINT` | 0 |

The last two rows are where CODESYS's runtime keeps the variable: its
memory area's number, the address and the offset in it. iecf's C has
no numbered areas, and an address in 32 bits is no address on a 64-bit
platform, so they are left at their initial values; a program that
needs one of them is not served, and says so only by reading 0.
`__SYSTEM.MEMORY_AREA` and its values (`MEM_GLOBAL`, `MEM_LOCAL`,
`MEM_MEMORY`, `MEM_INPUT`, `MEM_OUTPUT`, `MEM_RETAIN`) are declared,
for a program that compares with them. `x` is not read, nor its
indices worked out: what is given is its declaration's. `__VARINFO` of
an `ARRAY[*]`, whose size only the run knows, is
[E0016](errors/E0016.md). The value is assigned to a
`__SYSTEM.VAR_INFO`; read in place, `__VARINFO(x).Symbol`, it is not
compiled to C yet. The harness case `codesysvarinfo` reads every member
iecf fills.

**The vendors.** Beckhoff: "The operator returns information about a
variable", `<name of the info variable> := __VARINFO( <variable name>
);`, its table giving `Symbol` as "Variable name as STRING(39)"
([__VARINFO](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/3527777675.html)).
CODESYS: "The operator yields information about a variable", the same
table, `BitNr` for Beckhoff's `BitNo`, and `BitAddress` defined only
for a variable at `%I`, `%Q` or `%M`
([Operator: __VARINFO](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_varinfo.html)).
Neither says what `Symbol` is for a field: iecf gives the field's own
name, which is what a message naming a signal wants, and what fits in
39 characters.

**Why.** TcOpen calls `__VARINFO` 309 times, in nine libraries, and
reads only `Symbol`: a task waiting for a robot's or a press's signal
names it in its message (`__VARINFO(inoStatus.State.Inputs.Safety.ES_IsEmergencyStopped)`).
It was 627 of TcOpen's 2320 errors, the largest single cause
([TcOpen through iecf](testing/tcopen.md)).

**To make a program portable.** Write the name as a string where the
message is built.

## empty-arguments: an argument named and given nothing, `fb(x :=, y =>)`, left out as if not written

**The standard.** A formal argument is a name, `:=` or `=>`, and an
expression or a variable (Ed.3 Annex A, Param_Assign); `x :=,` with
nothing after it is [E0005](errors/E0005.md), naming the extension.

**The extension.** A parameter named and given nothing, `x :=` or
`y =>` before `,` or `)`, is left out of the call as if it were not
written (decided 2026-11-21, florin: "go"): an instance's input keeps
the value it had, as for any input not given; a FUNCTION's input is
its initial value; an output is not read. `NOT y =>` with nothing is
still E0005. The harness case `codesysemptyargs` calls a block with
empty inputs and outputs twice a cycle and a FUNCTION with an empty
input.

**The vendors.** Beckhoff's input assistant writes every parameter of
a call for the programmer to fill: "TwinCAT adds the call as follows:
CMD_TMR (IN:= , PT:= , Q=> ). All you have to add is the desired value
assignment"
([Object Function block](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2530279563.html)).
ABB's CODESYS-based help: "FBINST1(xx:= , yy=> ); ... But you not
necessarily must assign these parameters"
([Calling a function block](https://help.plc.abb.com/calling_a_function_block.html)).
Neither says in words what an empty one means; that the call compiles
with them unfilled, and that an input not given keeps its value, is
what iecf takes from both.

**Why.** TcOpen's EtherCAT tasks (TcoIo, TcoDrivesBeckhoff) call
Beckhoff's blocks as the assistant wrote them, `bBusy =>, bError =>,
tTimeout :=,`: 70 errors.

**To make a program portable.** Delete the empty arguments.

## bit: CODESYS's and TwinCAT's BIT, a structure's member one bit wide, packed into bytes

**The standard.** Ed.3 has no BIT. A structure's component placed at a
bit is a BOOL at a relative location, `a AT %X0.3 : BOOL` (6.4.4.7),
which iecf compiles (docs/c-output.md §4). `BIT` is a type not
declared, [E0011](errors/E0011.md), naming the extension.

**The extension.** A structure with a BIT member is read, before the
check, as a structure with relative locations, each BIT a BOOL at its
bit (decided 2026-11-22, florin: "go"):

```
TYPE S :                         TYPE S :
STRUCT                           STRUCT
    a : BIT;                         a AT %X0.0 : BOOL;
    b : BIT;            is           b AT %X0.1 : BOOL;
    n : INT;                         n AT %B2 : INT;
    c : BIT;                         c AT %X4.0 : BOOL;
END_STRUCT                       END_STRUCT
END_TYPE                         END_TYPE
```

- BITs in a row fill a byte from bit 0 up; the ninth is bit 0 of the
  next byte.
- A member of another type starts at the next whole byte, aligned to
  its own size: an array to its element's, a string to 1, nothing to
  more than 8. That is TwinCAT's default packing.
- A structure whose members are placed already keeps their places: a
  UNION's members (each at `%B0`), a STRUCT OVERLAP's, members `AT
  %I*`. A BIT there is a BOOL, at bit 0 of the byte it is placed at.
- A function block's BIT variable is a BOOL: nothing reads its bytes.
- A BIT reads and is written as a BOOL, TRUE or FALSE.

What is left out:

- A BIT beside a member whose size only its declaration elsewhere
  gives (a named structure, enumeration or alias) leaves the structure
  as written, and its BITs stay E0011, saying so. Its place would need
  that type's size before the check.
- `{attribute 'pack_mode'}` is not read. A structure TwinCAT packs
  with `pack_mode := '1'` puts a member right after the BITs, where
  this aligns it: TcOpen's `stEL9576_Status` has its UDINT at byte 2
  in TwinCAT, at byte 4 here.
- BIT anywhere else (a FUNCTION's, a PROGRAM's, a global variable, an
  array's element) stays E0011, as CODESYS refuses it there.
- The size of a structure ends at its last member's byte, unpadded,
  as any structure with relative locations is.

The harness case `codesysbit` writes BITs and an INT of a structure
and reads its bytes back through a UNION, reads a DWORD's bits through
an array of eight-BIT structures, and passes a BIT to a block's BIT
input.

**The vendors.** CODESYS: BIT is "valid only in structures for the
declaration of structure members or in a function block for the
declaration of variables"; "BIT variables that are declared in
succession are bundled in bytes", and a BIT "is exactly one bit",
TRUE (1) or FALSE (0)
([Data Type: BIT](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_bit.html)).
Neither CODESYS nor Beckhoff says which bit of the byte comes first,
or where the next member of another type starts. Bit 0 first is how
EtherCAT terminals number their status bits, which TcOpen's
structures map; that is an inference, not a vendor's word. Siemens,
PLCnext and the reference compiler (matiec) have no BIT.

**Why.** TcOpen declares 75 BITs: the status words of EtherCAT
terminals (TcoPneumatics' `stEL3058_ChannelStatus` and its kin, 16
BITs mapping two bytes), `BitStruct` (eight BITs, a REAL's bits read
through `floatUnion`), and one member of `F_AnyToUnionValue`: 76
errors.

**To make a program portable.** Give each member a relative location,
each BIT a BOOL `AT %Xbyte.bit`, as above.

## super-calls: CODESYS's SUPER^: the base's body called with arguments, from a method, more than once; a base's variable or property

**The standard.** `SUPER.m()` calls the base's method (Ed.3
6.6.5.7.3); `SUPER()` runs the base's body (6.6.7.2.9) with no
parameters (rule 2), in a function block's body and not a method (rule
5), once, and not in a loop. `SUPER.x` of a variable or a property is
not Ed.3's. Each is [E0031](errors/E0031.md), naming the extension.
`codesys-oo` reads `SUPER^` as `SUPER`, and these rules hold after it.

**The extension.** With `codesys-oo`'s spellings (decided 2026-11-22,
florin: "go"):

- `SUPER^(i := x, io := v, o => y)` is the base's body called as an
  instance of the base is called: its inputs set, its in-outs bound,
  the body run, its outputs taken. The base's inputs are the
  instance's own, so it is `i := x; SUPER^(); y := o;`. An in-out not
  given keeps the binding the instance's caller gave it.
- `SUPER^()` in a method runs the base's body on the instance, as it
  does in the block's body.
- `SUPER^()` may be called more than once in a body, and in a loop.
- `SUPER^.x`, a variable of the base or of its bases, is the
  instance's own `x`, read and written as `THIS^.x` is.
- `SUPER^.p`, a property of the base, calls the base's GET or SET
  itself, never an override's, as `SUPER^.m()` calls the base's own
  method.
- `SUPER^.x` that is neither is [E0010](errors/E0010.md).

A base that is a CLASS has no body: `SUPER^()` of it stays E0031. The
harness case `codesyssuper` calls a base's body twice in a derived
block's body, with an input, an in-out and an output and with an input
alone, and once from a method; reads a property the derived block
overrides through SUPER and through THIS; and reads and writes a base's
variable through SUPER.

**The vendors.** Beckhoff: "You can only use SUPER in method
implementations and associated function block implementations", with
`SUPER^();  // Call of FB-body of base class`, and
`nBase := SUPER^.nCnt;` reading a VAR_OUTPUT of the base
([SUPER](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2528837771.html));
a base method called with an argument,
`SUPER^.ExecuteProcess(bExecuteProcess := bExecuteProcess)`
([Use cases for inherited elements](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/3537664139.html)).
ABB's CODESYS-based help says the same: "You can use `SUPER` only in
methods and in the associated function block implementations", and
`iBase := SUPER^.iCnt;`
([SUPER](https://help.plc.abb.com/_cds_pointer_super.html)). No page
shows `SUPER^()` with arguments, says what it runs in a method, names
a property through SUPER, or limits how often it is called; TwinCAT
compiles each of those in TcOpen. The meanings above are iecf's
reading of that, not a vendor's words.

**Why.** TcOpen calls its bases so 65 times: `SUPER^(inoAtHomePos :=
inoAtHomePos, ...)` in its cylinders, robots and drives (24, two of
them in methods, and TcoCylinder twice in its body), `SUPER^()` in its
dialogs' `Show` methods (8), and `SUPER^.x` (33: properties such as
`ProfinetReady` and `IsAutoRestoreEnabled`, variables such as
`_currentStep`).

**To make a program portable.** Set the base's inputs and read its
outputs around a bare `SUPER()` in the body; call the body once; read
a base's variable as THIS's; for a base's property, give the base a
method that reads it, and call that with `SUPER.m()`.

## struct-blocks: a structure's function block members set up by their FB_init, THIS^ the structure there; ABSTRACT on a structure

**The standard.** A structure holds data (Ed.3 6.4.4.6); an instance
of a block may be one of its members, set up by its initial values. It
has no FB_init (fb-init's, CODESYS's), no THIS, and no ABSTRACT, which
Ed.3 gives blocks and methods (6.6.5.8). A member given arguments,
`m : T(args)`, is [E0005](errors/E0005.md) without fb-init and
[E0009](errors/E0009.md) with it; a variable of a structure whose
members have an FB_init to call is E0009; `ABSTRACT X : STRUCT` is
E0005. Each names the extension.

**The extension.** As TcOpen's TcoStruct, which its data structures
extend (decided 2026-11-22, florin: "go"):

```
TYPE ABSTRACT TcoStruct : STRUCT
    Parent : ITcoObject;
    THISSTRUCT : TcoParentedStructure(NO.Obj, THIS^);
END_STRUCT END_TYPE
TYPE Inspectors EXTENDS TcoStruct : STRUCT
    Dmc : TcoDataInspector(THISSTRUCT);
END_STRUCT END_TYPE
...
VAR Data : Inspectors := (Parent := THIS^); END_VAR   (* in a block *)
```

- A variable of a PROGRAM's or FUNCTION_BLOCK's `VAR` whose structure
  holds block members to set up (given arguments, or whose type has an
  FB_init) has each one's FB_init called as fb-init calls an
  instance's: at the end of the POU's set-up, after the variable's
  initial values (so `Parent := THIS^` is there), member by member in
  the order declared, a nested structure's at its place, each type's
  of a member's chain, the base's first.
- In those arguments a member's name, `THISSTRUCT`, is that member of
  the same variable; `THIS^` (or `THIS`) is the variable itself, the
  structure. Any other name is the POU's, as in its own declarations.
- A structure that extends `Base` binds a `REFERENCE TO Base`, as
  `THIS^` does for `ParentStruct : REFERENCE TO TcoStruct`: its C
  structure starts with its base's (struct-extends), and a reference
  names where it is.
- A member's initial value may give an interface field **another
  member of the structure that is an instance**: `inner : D := (Parent
  := THISSTRUCT)`, at any depth of the initial value. It is each
  variable's own member, set as the variable is set up, beside its
  initial values and before any FB_init (decided 2026-11-23, florin:
  "go"); the type's own initial value leaves that field NULL. Another
  member's data, `b : INT := a`, stays [E0010](errors/E0010.md): what it
  holds would hang on the order members are set up.
- `ABSTRACT` before a structure's name is taken and has no effect:
  TcOpen itself declares a variable of TcoStruct, `NO.Structure`.

A member that is an array of instances, given arguments or whose type
has an FB_init, has each element's called as fb-init's arrays do
(2026-11-23). An in-out of such a structure is bound, not set up.

Left out, each an error saying so: a variable of such a structure
anywhere but a PROGRAM's or FUNCTION_BLOCK's `VAR` (a FUNCTION's, a
method's, a VAR_TEMP, VAR_GLOBAL), an array of it, as a variable or a
member (E0009). The harness case
`codesysstructblocks` builds a TcoStruct of its own, a nested one given
its Parent so, and reads back what each member's FB_init was given.

**The vendors.** Beckhoff gives FB_init arguments to instances in a
PROGRAM and in an array, `ARRAY[1..2] OF FB_Sample[(nInitParam := 4),
(nInitParam := 7)]`, and runs a base's FB_init before its derived
type's
([FB_init](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/5094414603.html),
[call sequence](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/5482088715.html));
ABB's CODESYS-based help: "You can use THIS only in methods and in
function blocks"
([THIS](https://help.plc.abb.com/_cds_pointer_this.html)). None says
anything of a structure's member given FB_init arguments, of THIS in a
structure, of the order of a structure's members' FB_init, or of
ABSTRACT on a structure; TwinCAT compiles all of it in TcOpen, and the
meanings above are iecf's reading of that code.

**Why.** TcOpen's TcoStruct: 15 structures extend it, 42 members are
given `THISSTRUCT`, and its blocks give their structures `(Parent :=
THIS^)`: 48 errors.

**To make a program portable.** Give each member its parent through an
input or a method called from the first cycle, and pass a structure as
`REFERENCE TO` its own type.

## query-interface: CODESYS's `__QUERYINTERFACE(a, b)`, `b ?= a` giving whether it held; `__SYSTEM.IQueryInterface`

**The standard.** Ed.3 tries an interface with the assignment attempt,
`b ?= a` (6.6.6.7): `b` gets the instance `a` references if its type
implements `b`'s interface, NULL if not. iecf builds it. A statement,
it gives no value; `__QUERYINTERFACE` and `__SYSTEM.IQueryInterface`
are not Ed.3's. Without the extension the call is
[E0010](errors/E0010.md) and the interface
[E0011](errors/E0011.md), each naming the extension and the call
`?=` besides.

**The extension.** As TcOpen asks a parent for what it is (decided
2026-11-22, florin: "go"):

```
INTERFACE ITcoObject EXTENDS __SYSTEM.IQueryInterface ... END_INTERFACE
...
VAR _drive : ITcoDriveSimple; END_VAR
IF __QUERYINTERFACE(THIS^.Parent, _drive) THEN
    _drive.Halt();
END_IF
```

- `__QUERYINTERFACE(a, b)`, `a` an interface and `b` a variable, an
  element or a field of an interface, does `b ?= a` and gives a BOOL:
  TRUE when `b` then holds an instance. When `a` is NULL or its
  instance's type does not implement `b`'s interface, `b` is NULL and
  the call FALSE, as CODESYS sets it to 0. It is a call in an
  expression: what reads `b` after it in the same expression,
  `__QUERYINTERFACE(o, d) AND d.Speed() = 7`, reads what it stored.
- `__SYSTEM.IQueryInterface` (any case, `__System`) is an interface
  that declares nothing, there wherever a program names it. CODESYS
  asks that both interfaces extend it; iecf does not, every instance
  carrying its type, so a program for CODESYS still compiles.

Left out, each an error saying so: an instance first,
`__QUERYINTERFACE(inst, b)` ([E0016](errors/E0016.md)): whether its
type implements `b`'s interface is known as it is compiled, so it is
written `b := inst`. CODESYS's `__QUERYPOINTER` is the
[query-pointer](#query-pointer-codesyss-__querypointera-p-the-instance-an-interface-refers-to-as-a-pointer-to-its-block-its-type-checked) extension's, `__NEW`
and `__DELETE` the dynamic-memory extension's. The
harness case `codesysqueryitf` asks a motor, a lamp, NULL and a lamp's
derived type for a drive.

**The vendors.** CODESYS: "The operator returns a `BOOL` result.
`TRUE` means that CODESYS has performed the conversion successfully",
its first operand an "Interface reference or FB interface", both
interfaces derived from `__System.IQueryInterface`, which "is
implicitly available does not require a library"; its example leaves
the second 0 on FALSE
([__QUERYINTERFACE](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_queryinterface.html)).
TwinCAT takes it in TcOpen.

**Why.** TcOpen asks its parent for a drive, a state or a serviceable
object 27 times; 7 of its interfaces extend `__SYSTEM.IQueryInterface`:
53 errors.

**To make a program portable.** `b ?= a;` then `IF b <> NULL THEN`;
drop `EXTENDS __SYSTEM.IQueryInterface`.

## dynamic-memory: CODESYS's `__NEW` and `__DELETE`, memory taken while the program runs from its task's pool, each pointer to it checked

**The standard.** Ed.3 has no pointers and no memory taken while the
program runs: every variable is declared. Without the extension
`__NEW` and `__DELETE` are [E0010](errors/E0010.md), naming it.

**The extension.** With the pointers extension, as TcOpen's sequencer
keeps a list of step IDs that grows from cycle to cycle (decided
2026-11-22, florin: "go"):

```
VAR list, grown : POINTER TO INT; count : INT; END_VAR
grown := __NEW(INT, count + 1);       (* 0 when the pool has no room *)
FOR i := 0 TO count - 1 DO grown[i] := list[i]; END_FOR;
IF list <> 0 THEN __DELETE(list); END_IF;   (* list is 0 after it *)
list := grown;
```

- `__NEW(T)`, `__NEW(T, n)`: a `POINTER TO T` to `n` values of the data
  type `T` (1 without `n`) - an elementary type, a STRING, an array or a
  structure of them - each set to `T`'s initial value. It is the whole
  value assigned to a pointer, `p := __NEW(T, n)`, as CODESYS and
  TwinCAT take it ([E0016](errors/E0016.md) elsewhere). It gives 0 -
  NULL - when `n` is below 1 or the pool has no gap that holds them, as
  both vendors' does when it fails.
- `__DELETE(p)`, a statement of its own of a pointer variable: the
  block given back and `p` set to 0, as CODESYS does; `__DELETE` of 0
  does nothing.
- **The pool.** Each task has one, `--dynamic-memory BYTES` of data
  (rounded up to 8) and a slot for each 64 bytes, at least one, a slot
  for each block; a program that calls `__NEW` without the flag is
  E0016, naming it - as CODESYS asks for its "Use dynamic memory
  allocation" option. The pool is part of the context: no `malloc`, no
  system heap, its size fixed when the program is built, and taking or
  giving back a block walks at most its slots. A block is the first gap
  that holds it. `__NEW` takes from the running task's pool, so no task
  waits on another's; outside a task - FB_init at a start - it gives
  0 ([The C output](c-output.md) §3).
- **Each use is checked**, as every pointer's is: in its block's bytes
  (`IECF_FAULT_INDEX` past them) and **not given back**. A pointer to a
  block carries the block's slot and generation; `__DELETE` gives the
  slot a new one, so a copy used after it faults, `IECF_FAULT_NULL`,
  every time - the bytes are never read through it. `__DELETE` of a
  pointer `__NEW` did not give, of one moved from its block's start, or
  of another task's block is `IECF_FAULT_POINTER`; of a block given
  back already, `IECF_FAULT_NULL`. CODESYS and TwinCAT say nothing of
  these; in C they are undefined behaviour.
- **A start empties every pool.** In a project that calls `__NEW`, no
  pointer is RETAIN ([E0027](errors/E0027.md), not W0006): it may hold
  a block the start gave back.

Left out, each an error saying so: `__NEW` of a function block, a class
or an interface (E0027): an instance made while the program runs would
need its FB_init and FB_exit, and a pointer points to data only;
Beckhoff's `{attribute 'enable_dynamic_creation'}` is not needed, and
not read. A block not given back stays taken until the next start, as
the vendors' do: TcOpen's `TcoSequencerBase` leaves one when it
returns early from its loop, which this pool shows as `__NEW` giving 0
once it is full. The harness case `dynmem` grows a list over four
cycles, takes a structure with its initial value, asks for more than
the pool holds and reads a copy given back.

**The vendors.** Beckhoff: `__NEW` "dynamically reserves memory for
function blocks to instantiate user-defined data types or arrays of
standard data types"; "If the memory allocation attempt fails, __NEW
returns the value 0"; "Dynamic memory is allocated from the router
memory pool", and must be released "either already in the same task
cycle or at the latest before the PLC is shut down. Otherwise, a
so-called memory leak may occur"
([__NEW](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529171083.html)).
CODESYS: after `__DELETE` "the operand is set to zero", and "Two tasks
should not call __DELETE simultaneously"
([__DELETE](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_delete.html);
[__NEW](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_new.html)).

**Why.** TcOpen's sequencer checks that its steps' IDs are unique with
a list it grows with `__NEW` and gives back with `__DELETE`: 25 errors.

**To make a program portable.** An array of the most it will hold,
`ARRAY[0..MAX] OF INT`, and a count.

## program-instances: CODESYS's and TwinCAT's programs as single instances: `PRG.x` read and written, `PRG(...)` called, `PRG.m(...)`, from any POU

**The standard.** Ed.3 instantiates a PROGRAM only in a resource,
`PROGRAM p WITH t : PRG` (6.6.4, 3: "Programs can only be instantiated
within resources"), reaches its variables from outside only through
access paths (VAR_ACCESS, 6.8), and leaves "the object-orientation
features for programs" out of its scope (6.6.4, 5). Without the
extension `PRG.x` is [E0010](errors/E0010.md), `PRG()` in a POU
[E0016](errors/E0016.md) and a METHOD or PROPERTY in a PROGRAM
[E0005](errors/E0005.md), each naming it.

**The extension.** As CODESYS and TwinCAT have a program, which is one
instance and named by its name (decided 2026-11-23, florin: "go"):

```
PROGRAM counter
VAR_INPUT inc : INT; END_VAR
VAR_OUTPUT total : INT; END_VAR
VAR calls : INT; END_VAR
METHOD PUBLIC Bump : INT ... END_METHOD
total := total + inc;
END_PROGRAM

counter(inc := 100);              (* from a block, a program, a method *)
r := counter.Bump(1);
counter.calls := counter.calls + 10;
```

- A PROGRAM some POU names so - `PRG.x`, `PRG(...)`, `PRG.m(...)` - or
  that declares a METHOD or a PROPERTY **is one instance**, which every
  use of its name reaches: its inputs, outputs and its own variables,
  read and written, as TwinCAT compiles TcOpen's; its methods and
  properties; `PRG(...)` with arguments, as a block's call.
- **The configuration's instance is that one.** `PROGRAM c WITH tk :
  counter(inc := 1, total => sum)` runs it, with those connections, in
  its task: a task and the POUs that call it share it, so a program a
  task runs and a POU calls runs twice in that cycle, as in CODESYS.
  The configuration declares at most one instance of it (E0016); with
  none it is still there, set up at every start, run only when called.
  RETAIN when the configuration's instance is.
- `PRG.x` names the program even where `PRG` is also a value of an
  enumeration: a value has no members. TcOpen's `NO.Obj` is its PROGRAM
  NO's, not `eDialogAnswer.No`.
- Not from a FUNCTION: "A program call is not allowed in a function"
  (CODESYS), E0016.
- In the output the program is a FUNCTION_BLOCK of its name, its
  instance the global `__PRG_<NAME>` in the symbol table (§6 of [The C
  output](c-output.md)), and the configuration's instance a PROGRAM
  `__PRG_RUN_<NAME>` that calls it with the connections given.

Left out, each an error saying so (E0010, E0016): a PROGRAM with what
only a program has - a VAR_GLOBAL or a VAR_ACCESS of its own, a partly
located variable (`AT %I*`), a block the configuration gives a task of
its own (`fb WITH t`) - stays a PROGRAM, not reached by its name; a
configuration's VAR_ACCESS or VAR_CONFIG path through the instance of
one made a block (`R.C.calls`) does not reach it. A program in a
namespace (a library's: `--library` puts its PROGRAMs there too) is
named as any element of it, by its short name inside the namespace or
through a USING, `Lib.PRG.x` and `Lib.PRG.m()` from anywhere (since
2026-11-23; before, `Lib.PRG.x` was refused). The harness case
`prginst` runs a counter from its task and from another program, calls
its method, writes its variable, and reads `NO.v` beside two
enumerations with a value `No`; `codesyslibprograms` two libraries'
MAINs beside the project's, and a library's program called, read and
method-called by its short name and by `A.Counter`.

**The vendors.** CODESYS: "Programs and function blocks can call a
program. A program call is not allowed in a function. There are not
instances of programs." and "If a POU calls a program and values of the
program change as a result, then these changes are retained until the
next program call"; its example reads `erg := PLC_PRG.out2;`
([Program](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_obj_program.html)).
It gives no general rule for a program's other variables; TwinCAT
compiles TcOpen, which reads `RtcPrg`'s own VARs, calls its METHOD,
gives `NF_MAIN`'s PROPERTY to an FB_init, and calls programs from
MAIN.

**Why.** TcOpen: `NO.Obj`, its "no parent" object, in every
TcoStruct; `RtcPrg`, its clock, read and called from TcoRtc and
TcoContext; the examples' MAIN calling their programs: 21 errors.

**To make a program portable.** A FUNCTION_BLOCK and a global
instance of it, `VAR_GLOBAL counter_1 : counter; END_VAR`, called by a
PROGRAM the configuration runs.

## query-pointer: CODESYS's `__QUERYPOINTER(a, p)`, the instance an interface refers to as a `POINTER TO` its block, its type checked

**The standard.** Ed.3 takes an instance out of an interface with the
assignment attempt into a reference, `r ?= a` (6.6.6.7), `r` a `REF_TO`
the block: `r` NULL when the instance is of another type. iecf builds
it. `__QUERYPOINTER` and a `POINTER TO` a block are not Ed.3's; without
the extension the call is [E0010](errors/E0010.md), naming it, and the
pointer [E0027](errors/E0027.md) (the pointers extension's point to
data).

**The extension.** As TcOpen's test of `EqualsTo` asks what an
interface holds (decided 2026-11-23, florin: "go"):

```
VAR_INPUT Object : ITcoObject; END_VAR
VAR _object : POINTER TO TcoObjectEqualsTest; END_VAR
IF __QUERYPOINTER(Object, _object) THEN
    EqualsTo := _object^._someNumber = THIS^._someNumber;
END_IF;
```

- **A `POINTER TO` a function block or a class is a reference to an
  instance**, iecf's `REF_TO` it: `p^.x`, `p^.m()`, `p = 0`, `p <> 0`,
  `p = NULL`, `p := 0`, `p := q` of one type, `p := ADR(inst)` - which
  is `REF(inst)`, refused of a temporary ([E0026](errors/E0026.md)) -
  each use through NULL a fault, `IECF_FAULT_NULL`, as a reference's.
  No arithmetic, no `SIZEOF`, no `p[i]`, and no number: an instance is
  not bytes to read as others ([E0014](errors/E0014.md),
  [E0013](errors/E0013.md)). A `POINTER TO` data is the pointers
  extension's, unchanged.
- **`__QUERYPOINTER(a, p)`**, `a` an interface: `p ?= a` in the
  expression, TRUE when `p` then holds an instance - the one `a`
  refers to, **if it is of `p`'s block's type or one derived from it**;
  `p` 0 and FALSE when `a` is NULL or its instance is of another type.
  CODESYS leaves the type to the program ("Pointer_Dest is not typed
  and can be cast to any type. You have to make sure of the type"), so
  a pointer of the wrong type reads another block's memory; iecf's
  answer is FALSE instead. A program right for CODESYS behaves the
  same. It counts as an effect: what reads `p` after it in the same
  expression reads what it stored.

Left out, each an error saying so: an instance first,
`__QUERYPOINTER(inst, p)` ([E0016](errors/E0016.md)), written
`p := ADR(inst)`; a pointer to an interface or to a FUNCTION. TcOpen
also takes an instance's address as a number, `_Identity :=
ADR(THIS^)`, 5 times: an address is no number in iecf, so these stay
E0013. The harness case `querypointer` asks one interface for a Lamp,
a lamp's derived type, a Motor and NULL, and writes through
`ADR(inst)`.

**The vendors.** CODESYS: the operator "makes it possible to convert
the type of an interface reference of a function block to a pointer",
"returns a BOOL result", TRUE when "CODESYS has performed the
conversion successfully"; the pointer "receives the pointer to the
reference or instance of a function block that the ITF_Source
interface reference currently refers to"; the interface extends
`__SYSTEM.IQueryInterface`
([__QUERYPOINTER](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_querypointer.html)).
It does not say what the pointer holds on FALSE. Beckhoff's page was
not found; TwinCAT compiles TcOpen's use.

**Why.** TcOpen's `TcoObjectEqualsTest` compares two of its objects
through it: 3 errors.

**To make a program portable.** `r : REF_TO T;` and `r ?= a;`, then
`IF r <> NULL THEN`.

## expression-statements: an expression standing as a statement, `fb.Prop;`, its calls made and its value unused, warned

**The standard.** Ed.3's statements are assignments, calls, RETURN and
the control statements (7.3.3, Table 72): an expression on its own, `fb.IsTrue;`,
is none. It is [E0005](errors/E0005.md), naming the extension.

**The extension.** An expression on its own is a statement: every
call in it - a FUNCTION, a method, a block, a property's GET - is made,
in order, as it would be were the value assigned, and the value is
dropped; one with no call does nothing, and no code is written for it,
so an index in it is not checked. Each is warned,
[W0014](errors/W0014.md), saying which (decided 2026-11-23, florin: "go
with A"). `a = b;` stays [E0005](errors/E0005.md): `=` compares. The
harness case `codesysexprstmt` runs a property's GET, a method and a
property of the interface it gives, a FUNCTION with an in-out inside
a sum, a field read and an element out of bounds.

**The vendors.** CODESYS compiles it with its message C0139: "The code
<code> has no effect. Is this the intent?", "The written code is
syntactically correct but does not do anything"
([C0139](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_error_c0139.html)).
TwinCAT 2's list has the warning "Expression contains no assignment.
No code was generated" ([Compiler Errors](https://infosys.beckhoff.com/content/1033/tcplccontrol/925418891.html)).
Neither says whether a call in it is made. TwinCAT 3 makes it:
TcOpen's own test `T360_TaskExecuteMethodNotCalled` runs
`_to_A._sut_A.Invoke().Done;` on the PLC and expects the message only
`Invoke()` and `Done`'s GET give.

**Why.** TcOpen's tests read properties so for what their GETs do,
`_sutNotInitialized.IsTrue;`, `_sutInvalid.AtHome;`: 16 errors.

**To make a program portable.** Assign the value, `dummy :=
fb.IsTrue;`, or call the method on its own.

## header-semicolon: a `;` closing a POU's or a method's header, `METHOD M : BOOL;`, before its declarations

**The standard.** A header ends with its name, its result type, its
EXTENDS or IMPLEMENTS; its declarations follow it (Ed.3 Annex A). A
`;` there, `METHOD M : BOOL;` followed by `VAR_INPUT`, is
[E0005](errors/E0005.md), naming the extension. Followed by a
statement it is the empty statement it always was, with or without.

**The extension.** A `;` right after a POU's or a method's header,
before its declarations (a VAR section, a METHOD, a PROPERTY), closes
the header and is dropped: the declarations are the POU's or the
method's (decided 2026-11-23, florin: "A, a new extension, 1 commit").
The harness case `codesyshdrsemi` declares a FUNCTION and two methods
so.

**The vendors.** CODESYS's [Object: Method](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_obj_method.html)
gives the header as "METHOD <access specifier> <method name> : <type
of return value>" and says nothing of a `;`. TwinCAT compiles TcOpen,
which writes one 19 times.

**Why.** Without it TcoCognexVision's TcoInsight lost a method's
declarations to its block, `METHOD SetUserDataAsByte : BOOL;`: 12
errors in it, 4 in the tests that read it.

**To make a program portable.** Leave the `;` out.

## auto-addresses: TwinCAT's `%I*`, `%Q*`, `%M*` no VAR_CONFIG completes, a global's too, placed by iecf in the image, warned

**The standard.** A `*` address, `AT %I*`, `%Q*` or `%M*`, is completed
per instance by the configuration's VAR_CONFIG (Ed.3 6.5.5.4): one
missing is [E0023](errors/E0023.md); a global, having one instance,
writes its address in full ([E0022](errors/E0022.md)). CODESYS and the
tools built on it - ABB's Automation Builder, Schneider's Machine
Expert, Bosch Rexroth's ctrlX PLC - do the same: "The final address
specification is done in the variable configuration"
([CODESYS AT Declaration](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_at_declaration.html)),
or the variable is mapped in the device's I/O mapping.

**The extension.** As TwinCAT: "If the placeholders \* (%I\*, %Q\* or
%M\*) are used, TwinCAT automatically performs flexible and optimized
addressing"
([Allocated variables](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/12081664011.html)),
and links them to the hardware in its I/O configuration. A `*`
variable no VAR_CONFIG entry gives - a program instance's, a global's
own (`INs AT %I* : ARRAY[0..15] OF BOOL` in VAR_GLOBAL), a `*`
component of a global (`Robot1 : TcoIrc5_IO_v_1_x_x`) - is placed by
iecf: in its area, after the last byte any address the program states
reaches, one after another in the order of their paths, each its own
type's width, a BOOL a whole byte (decided 2026-11-24, florin: "build
the extension B"). A VAR_CONFIG entry still gives one its address. Each
declaration, and each program instance, with such variables is warned,
[W0015](errors/W0015.md). The image's sizes count them, and the JSON
symbol table's `"partly"` lists each, its path, type and address,
`"placed": true`, for the platform to link it ([C output](c-output.md) §4, §6). A VAR_EXTERNAL of
such a global names it, as an in-out does; an array of a type with `*`
components, an input or an output of one, stay E0022. Not in a pack:
it changes the image a platform provides, and only TwinCAT does it.
The harness case `autoaddr` places each kind.

**Why.** TcOpen's test projects declare their devices' I/O so, in their
global variable lists: 29 E0022 and 5 E0023.

**To make a program portable.** Give each its address in VAR_CONFIG,
or write it in full.

# Proposed, not built

These are extensions iecf could offer and does not yet: the standard's
behaviour stays the default, and each would be switched on by name as
those above are. None is proposed today.
