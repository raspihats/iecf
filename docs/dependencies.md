# Implementer specific dependencies

IEC 61131-3 Ed.3 clause 5 d asks an implementation for "a document that
specifies all Implementer specific dependencies": the choices the
standard leaves open, and its limits. This is that document for iecf,
growing as the compiler does.

Each choice is made after checking what the big vendors do (CODESYS and
the systems built on it, Beckhoff TwinCAT, Siemens), and says so: a
program that compiles there should, as far as the standard lets it,
mean the same here.

## Implicit conversions (Ed.3 6.6.1.6)

Figure 11's implicit conversions, which keep the value, apply in an
assignment, to an input and an output (`=>`) of a block or function, and
between the operands of an operator or overloaded function (rules 3-5,
7); never to an in-out (rule 6). A date or time with a long operand is
widened to the long type: `DT + LTIME` is an LDT.

- **An operation is done in its operands' type**, the narrower operand
  widened to the wider one's, and its result widened only where it is
  used: Ed.3 6.6.1.6 Example 3, `myUDInt := myUInt1 * myUInt2`, "The
  multiplication has a UINT result which is then implicitly converted
  to an UDINT at the assignment". So `d := i * j` of two INTs overflows
  as an INT, flagged. CODESYS "computes temporary results always with
  the native size that is defined by the target device", at least 32
  bits, and TwinCAT "always calculates intermediate results with the
  native size specified by the target system": there the same product
  is not truncated below 32 bits, but `lint := dint * dint` is. RuSTy
  widens every integer narrower than DINT to 32 bits first. iecf keeps
  the standard's rule, which is the same on every target.
- **Two integers neither of which widens to the other** are both
  widened to the narrowest signed type that holds them both: SINT and
  USINT to INT, INT and UINT to DINT, DINT and UDINT to LINT, and so
  for other pairs (SINT and UINT to DINT). ULINT and LINT have none, and
  are refused (E0014). It holds wherever operands are made one type: an
  operator, a comparison, an overloaded function such as MAX. Ed.3 says
  nothing on it. Decided 2026-11-09 (florin): as Siemens SCL, which is
  reported (by third-party restatements of the TIA help, not its own
  pages) to give "the next larger signed integer type", SINT + USINT an
  INT. Others: RuSTy computes in a signed type of 32 bits or more and
  warns that a large unsigned value may turn negative; CODESYS and
  TwinCAT compute in the target's register width, and CODESYS warns
  (C0195) on a signed value made unsigned; PLCnext reports anything but
  a widening as an error (SEM1189). The common type keeps both values,
  as Figure 11's implicit conversions must.
- **An overloaded function's generic class is not entered by a
  conversion**: `SQRT(i)` of an INT is E0014 (see Untyped literals).

## Typed functions (Ed.3 6.6.2.4, Table 21)

Table 21, footnote a: "If feature 2 is supported, the Implementer
provides an additional table showing which functions are overloaded and
which are typed in the implementation." iecf supports it: every
overloaded standard function whose inputs share one type has a typed
form for each type of its class, `ADD_INT`, `MUX_UINT_REAL`. The table
is in the book ([Typed functions](behaviour/typed-functions.md)). A
name a table of the standard gives a function of its own - `ADD_TIME`,
`MUL_TIME`, `CONCAT_DT` - is that function, not a typed form.

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
- A standard function given untyped literals alone - `MAX(10, 11)`,
  `SQRT(2.0)`, `SEL(TRUE, 1, 2)` - stays untyped the same way, and so
  does a quotient of real literals, `1.0 / 3.0`. It is computed as the
  program runs, in the type the place needs: `v := ADD(30000, 30000)`
  is 60000 for a DINT `v`, and for an INT `v` wraps to -5536 and is
  flagged OVERFLOW, as `v := a + b` of two INTs is. Integers stay
  integers where a real is wanted: `r := 7 / 2` is 3.0, and so is
  `r := MAX(7, 2) / 2`, computed in LINT. Where nothing decides - both
  sides of a comparison, `GT(MAX(1, 2), 1)` - integers are computed in
  LINT and reals in LREAL. (2026-11-12; florin has not decided this,
  and no vendor's documentation was checked for it. The harness case
  `litcalls` runs each.)

Vendors: CODESYS documents `SQRT` as taking "any numeric base type" and
gives `q := SQRT(16)` as its example. It goes further and takes integer
*variables* too; iecf does not: an INT variable where ANY_REAL is
required is E0014 (convert it, `INT_TO_REAL`).

## Recursion is refused (Ed.3 6.6.1)

"The recursive call of POUs and methods is Implementer specific." iecf
refuses every recursive call, of a FUNCTION, a block, a PROGRAM or a
method, directly or through others (E0016), a call that may run an
override or another implementation counting as a call of each. Decided
2026-11-12 (florin), after the vendors: PLCnext refuses all (SEM1030,
SEM1259); CODESYS and TwinCAT refuse a FUNCTION's (C0224) and warn of a
method's (C0298); Siemens documents no rule (a Siemens forum expert:
within the CPU's block nesting depth); the reference compiler cannot
express one. An opt-in with a depth limit and a fault is proposed in
docs/extensions.md.

## SINGLE and an INTERVAL that is a variable (Ed.3 6.8.2)

A TASK's SINGLE and INTERVAL are a Data_Source: a constant, a global or
one field of one, a program's output, an address. iecf schedules
nothing (docs/c-output.md §3), so the task table gives the platform a
function per task that reads each from the context as it is now, and
the platform runs the task on SINGLE's rising edges and at the interval
while SINGLE is FALSE, as Ed.3's rules a and b say. Implemented
2026-11-12 at florin's word; the shape of the table (`single`,
`interval`, each NULL when unused) is my choice. Until then the checker
refused a variable there, and a SINGLE task was E0009.

## RETAIN at an address (Ed.3 6.5.6, Loc_Var_Decls)

Ed.3's grammar lets a located variable be RETAIN (`'VAR' ( 'CONSTANT' |
'RETAIN' | 'NON_RETAIN' )? ( Loc_Var_Decl ';' )*`). Such a variable is
in the process image, which the platform owns, not in `ctx->retain`: its
initial value is written at a cold start only, and at a warm start iecf
leaves the bytes, which the platform keeps across the restart. Decided
2026-11-12 (florin).

## Several CONFIGURATIONs and the scope of globals (Ed.3 6.5.2.2, 6.8.1)

A project may hold several CONFIGURATIONs, each a controller of its own.
Each has its own globals: two may declare `g`, each its own, and a POU
two of them run reaches each one's (E0017 checks it in each, the
message naming the one that lacks it). Decided 2026-11-13 (florin):
each is built into files of its own, as if alone (docs/c-output.md §2).
CODESYS and TwinCAT build each application or PLC project on its own
too; Siemens each CPU.

Ed.3 scopes a VAR_GLOBAL to the configuration, the resource or the
PROGRAM that declares it ("the VAR_GLOBAL block of the associated
program, configuration or resource"). A resource's globals are its own:
two resources may each declare `g`, and a POU reaches the `g` of the
resource it runs in, or else the configuration's; a resource declaring
a name its configuration declares is E0012, as the standard gives no
rule for one hiding the other. Decided the same day (florin): where one
type's instances reach different globals by one name, each instance
holds a pointer to its own (docs/c-output.md §3). Built 2026-11-13.

A PROGRAM's own VAR_GLOBAL is Table 47's features 8a and 8b; Ed.3's
grammar for Prog_Decl leaves it out, and 6.5.2.2 says only that a
VAR_GLOBAL CONSTANT of a "program organization unit" binds "any element
contained within the containing element". So: each instance of the
program has its own, the program uses it as a variable of its own, and
it is seen by what the instance contains - the instances it holds,
through what they hold, and the FUNCTIONs it calls - beside what its
resource sees. A program declaring a name its resource or configuration
sees is E0012, as for a resource. In the symbol table it is a variable
of the program instance, `R.P.Z`, and VAR_CONFIG gives it an initial
value for one instance as it does any variable of the program's.

What follows from the pointer, and is mine, for florin to confirm:

- **An instance belongs where it is declared.** Its globals are those
  of the resource whose program, or whose VAR_GLOBAL, holds it, through
  whatever holds it in between; a VAR_TEMP's or a FUNCTION's own
  instance, those of the instance or resource that is running. Called
  from another resource - through a reference kept in a
  configuration's global - it still reaches its own.
- **A whole copy keeps them**: `a := b` copies b's values and leaves a
  reaching what it reached, as a copy keeps `*` addresses.
- **A FUNCTION reaches one global wherever it runs.** It has no
  instance to hold which, so a FUNCTION whose VAR_EXTERNAL (or whose
  own instance's) would be `g` of one resource here and of another
  there is E0017; the message names the places. Its caller can pass
  the value in. So is one that reads a PROGRAM's own global, called by
  a program with two instances.
- **CONSTANT globals of one name hold one value.** A constant is
  worked out by the compiler, once: for a bound or an initial value in
  a TYPE or a POU, which is one type whichever configuration or
  resource runs it. So globals of one name in different scopes - two
  configurations', two resources', two PROGRAMs' - are all CONSTANT or none is, and
  the CONSTANT ones hold the same integer value, or E0012; constants of
  other types are named apart.

A VAR_GLOBAL at the top of a file, a global variable list as CODESYS
writes one, is seen by every configuration; one of a configuration
with the same name is E0012.

## `*` addresses and VAR_CONFIG (Ed.3 6.5.5.4, 6.4.4.8, 6.8.2)

The standard gives the paths, that every `*` address is completed for
every instance, and that VAR_CONFIG's initial values "always prevail
type specific initial values" and are not given to VAR_TEMP,
VAR_EXTERNAL, CONSTANT or VAR_IN_OUT variables. CODESYS reads
VAR_CONFIG with paths from the PROGRAM's name (the `var-config-list`
extension), asks for the same area and type as the `*` declaration, and
reports an address missing; its help shows an entry with an initial
value and says nothing of what it does. TwinCAT links `%I*` variables in
its I/O configuration, outside the source text, and has no VAR_CONFIG;
Siemens has neither. The reference compiler (matiec) parses
VAR_CONFIG and generates nothing for it (its `generate_c_configbody.cc`
leaves it a TODO).
Implemented 2026-11-13; the choices below are mine, for florin to
confirm:

- **The order**: the `*` addresses are set before any initial value is
  written, so a `%Q*` variable's own initial value lands at its
  address; VAR_CONFIG's initial values are set after all others, in
  the order the entries are listed. A whole structure's value sets the
  fields it does not name to their types' initial values, as any
  structure's initial value does: `x.p.y := 6` before `x.p := (x :=
  3)` leaves y 0.
- **A warm start** sets VAR_CONFIG's initial values again, but for a
  variable it keeps: a RETAIN one, or one inside an instance declared
  RETAIN that is not NON_RETAIN - as it sets the type's own.
- **An input**: an entry's initial value for a variable at an input
  address, `%I` or `%I*`, writes nothing, as a declaration's does not:
  the input image is the platform's.
- **A RETAIN variable at a `*` address** is in the image, as one at an
  address: its initial value at a cold start only, the bytes the
  platform's to keep.
- **A copy** of a whole instance or structure holding `*` variables
  (`a := b`) copies each such variable's value from b's address to a's;
  each keeps the address VAR_CONFIG gave it.
- **A structure with `*` components at an address** is refused (E0022):
  each component is where VAR_CONFIG puts it.

## Program connections (Ed.3 6.8.2)

What a connection names is Ed.3's grammar, Prog_Data_Source and
Data_Sink: a constant, an enumerated value, a global or one field of
one (Global_Var_Access), or an address; an element or an expression is
E0016 (2026-11-12; until then the checker took any expression there).
Global_Var_Access is `( Resource_Name '.' )? Global_Var_Name ( '.'
Struct_Elem_Name )?`: `R.g` names the global `g` of R, in a connection
and in a TASK's SINGLE and INTERVAL (built 2026-11-13, the harness case
`resaccess`). Mine, for florin to confirm: R is the resource the
connection is written in, and another resource's name there is E0016,
as 6.8.1 limits a global's scope to "the configuration or resource in
which it is declared" and excepts only access paths; and where a
global variable is itself named R, `R.x` is its field, as it was.
iecf's own choices there (2026-11-12; florin has not decided these):

- A STRING input or output is copied and cut to what it is stored in,
  a value given to a subrange input, or taken into a subrange global,
  is clamped - flagged, as any store. Such a flag is recorded at the
  CONFIGURATION and the connection's line: there is no POU's statement
  to record it at.
- A program's in-out connected to a global at an address is given a
  copy, made before the run and stored back after it, as a FUNCTION's
  in-out given one is; so is a subrange global of other bounds than the
  in-out's, stored back clamped and flagged.

## SEL, MUX and MOVE of user-defined types (Ed.3 Table 32, footnote d)

Table 32 gives MOVE and SEL inputs of ANY and MUX's of ANY_ELEMENTARY,
and footnote d, on all three: "It is allowed, but not required, that
the Implementer support selection among variables of user-defined data
types". iecf supports it for all three, MUX too (2026-11-12; florin has
not decided this): arrays, structures, instances, interfaces and
references, the input selected where it is, NULL beside an interface
or a reference, an instance beside an interface it implements. MIN, MAX
and LIMIT are ANY_ELEMENTARY in the table and have no such footnote:
they stay elementary. No vendor's documentation was checked for this.

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

Undone means it faults on nothing it computes after the conversion
(2026-11-12; florin has not decided this). Until then the conversion's
placeholder, 0, went on into the statement: `10 / REAL_TO_INT(x)` with
x out of range stopped the task on a division by zero, and
`a[REAL_TO_INT(x)]` on an index outside `a`, though the statement was
to be left undone. Now a fault the statement's own checks raise is
dropped once one of its conversions has failed (iecf.h `iecf_undone`);
a fault inside a FUNCTION it calls stands, recorded with that
function's line.

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

A subrange variable starts at its lower limit, as Ed.3 6.4.4.4.2 says:
"The default initial values for data types with subrange shall be the
first (lower) limit of the subrange." Until 2026-11-12 iecf started one
at 0, which is outside `INT (2..5)`. As a FUNCTION's input it is
clamped where the input is declared, the flag's POU and line the
function's, not the caller's (2026-11-12; florin has not decided this:
it follows a STRING input, cut and flagged there).

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

## FOR: the variable after the loop (Ed.3 7.3.3.4.2)

"The value of the control variable after completion of the FOR loop is
Implementer specific." Decided 2026-11-07 (florin), found by comparing
iecf's variables with matiec's on Prokop's programs: **one step past the
end** (`FOR i := 1 TO 10` leaves 11), as the standard's own examples
assume ("terminates when the value of the variable I reaches 0"; Table
72's J "will have the value 101"), as CODESYS ("at the end, i always will
be one step outside the range", its forum; its help: the loop runs "as
long as the counter is not greater than the end value") and as matiec.
At the type's limit the step past wraps and is flagged, and the loop
ends; on CODESYS it never does ("an infinite loop results"). A subrange
variable stays inside: the loop ends at its last value.

## EN and ENO (Ed.3 6.6.1.5)

The standard has the implementer "specify the behavior in detail" when
EN is FALSE (rule 1), leaves the outputs and the function result to the
implementer when ENO is FALSE (rule 4), and lets the implementer specify
another disposition of an error than ENO = FALSE (rule 3). Decided
2026-11-07 (florin), for a function call:

- **EN = FALSE: the call does not happen**, Ed.3's "external
  implementation" (its Example 2): the inputs are not evaluated, the
  function is not called, ENO is FALSE, and the statement the call is in
  is left undone - an assignment keeps its target's value - unflagged.
  No value is invented for the result. CODESYS: "If EN has the value
  FALSE at the time of the call, the operations defined in the POU are
  not executed" (CODESYS help, FBD/LD box with EN/ENO). matiec assigns
  the function's default result instead.
- **ENO is FALSE when the function's own operation raised a flag**
  (an overflow, a conversion that does not fit, a string length or
  position outside its string): rule 3's default, and Siemens's, whose
  instructions clear ENO on such errors. A flag raised while computing
  an argument is not the function's. CODESYS's ENO only mirrors EN.

A function block call, decided 2026-11-08 (florin), the same way:

- **EN = FALSE: the block is not called**, Ed.3's Example 4: its inputs
  are neither evaluated nor set in the instance, its in-outs not bound,
  its body does not run - a timer's or counter's state stays as it was -
  its outputs are not taken (`Q => x` leaves x), and ENO is FALSE,
  unflagged. CODESYS: the POU's operations "are not executed"; Panasonic
  FPWIN GR: the block is skipped and the connected variable keeps its
  value; logi.CAD/Neuron: ENO FALSE and the outputs not updated. matiec
  takes the outputs anyway.
- **ENO is FALSE when a flag is raised while the block runs**: in its
  body or anything it calls. A flag from computing its inputs, or from
  storing its outputs into the caller's variables (a subrange clamped),
  is not the block's.
- A block that declares EN or ENO among its own parameters gets them as
  any other parameter: the body then decides (Example 1).

ENO set in a POU's own body, decided 2026-11-08 (florin), after the
vendors - none of which documents how the body's ENO and an error
combine:

- **ENO is there in every FUNCTION and FUNCTION_BLOCK** without a
  declaration (Ed.3 rules 2 and 7), TRUE as each run starts; the body may
  read it and set it. A PROGRAM has none. A POU that declares ENO keeps
  it as an ordinary output (above).
- **The caller's ENO is FALSE when the body set it FALSE or a flag was
  raised while the POU ran**: a body can clear ENO, and cannot hide an
  error with `ENO := TRUE` (rule 3: on an error, ENO "shall be reset to
  FALSE by the programmable controller system").
- What the vendors document: Siemens S7-1200 "SCL also allows you to set
  the ENO parameter for a code block", and its compiler sets ENO "when the
  code block finishes" with "Set ENO automatically" (S7-1200 System
  Manual 04/2012, 6.5.4); PLCnext lets a function "set ENO = FALSE on
  specific conditions" and leaves a user function block's ENO to its
  body; logi.CAD/Neuron presets ENO TRUE, lets the block assign it, and
  sets the enclosing POU's ENO FALSE on an index out of range or a
  division by 0; Panasonic: ENO is EN unless the body sets it; CODESYS
  and TwinCAT: "The ENO output has the same value as the EN input". None
  says which wins when the body sets ENO TRUE after an error.

## SFC (Ed.3 6.7)

Decided 2026-11-07 (florin), after CODESYS's and TwinCAT's documented
processing order ("Processing order in SFC", Beckhoff InfoSys; CODESYS
help, SFC sequence of processing):

- **Actions, then transitions.** Each scan runs the action control and
  the actions of the steps active when it began, then tests their
  transitions; a step entered in this scan runs its actions from the
  next. CODESYS and TwinCAT: IEC actions are executed, then "the
  transitions are evaluated". matiec tests the transitions first and runs
  the actions of the steps active after, in the same scan.
- **With final scan** (Table 60, feature 1, Figure 23 a): an action runs
  once more, its Q FALSE, after Q falls - P included, which therefore
  runs twice, as CODESYS documents ("executed exactly two times"). The
  final scan and P0 follow a falling edge that happened; the figure's
  F_TRIGs, whose M starts FALSE (Table 44), would fire on the first scan
  too, which no vendor does.
- **Two passes**, as CODESYS and TwinCAT: the actions ending (final scan,
  P1, P0) before those whose Q is TRUE, each in source order (CODESYS
  takes alphabetical order).
- **Selection**: of the transitions leaving one step, the first true by
  PRIORITY (lowest first, Table 61 2b), then source order (2a, as
  CODESYS's leftmost branch); unflagged.
- **Unsafe** (a step entered while active), two time-related
  associations active, SD with SL stored: flagged `IECF_FAULT_SFC`, the
  task goes on; Ed.3 6.7.5 and 6.7.4.6 d call them errors.
- `S.T` counts whole milliseconds from the task's clock and stops at
  TIME's largest.

## A FUNCTION's outputs (Ed.3 6.6.1.4, 6.6.2.2)

A FUNCTION may declare VAR_OUTPUT; a call takes one with `=>`, or in
order after the inputs (6.6.1.4), negated with `NOT q =>` for a BOOL,
widened as an assignment is (6.6.1.6, rule 5).

- **An output starts at its initial value at every call**: a FUNCTION
  has no memory (6.6.2.2), so an output the body does not set gives its
  initial value, not what the last call left.
- **The outputs are stored when the call returns**, at its end or at a
  RETURN, with the values they have then. **A call that faults stores
  nothing**: the statement stops before any output or its own target is
  written, as for any statement that faults.
- **An output is stored before the statement's own target**: in
  `x := f(a, q => x)` x is f's result. Ed.3 does not order them; the
  vendors were not checked.
- An output the call does not take is dropped.
- **With EN = FALSE a FUNCTION with effects is not called**, as any
  other (EN and ENO, above): its in-outs and the globals it writes stay
  as they were, and **its outputs are not stored** - each output's
  variable keeps its value, as a SPLIT's do and as PLCnext keeps "the
  last valid output value". ENO is FALSE when the call raised a flag;
  storing its outputs afterwards is not the call, as for a function
  block, so an output clamped into a subrange leaves ENO TRUE.

## A FUNCTION's in-outs and externals (Ed.3 6.6.2.2)

A function "may change the value of in-out and VAR_EXTERNAL
variable(s)" (6.6.2.1; 6.6.2.2 rules 4-6), so its calls are not pure:

- **The order of evaluation is the standard's.** "When an operator has
  two operands, the leftmost operand shall be evaluated first" (7.3.2,
  rule 3), and a function's arguments in order. In `x + f(x)` with f
  changing its in-out x, x is read first; in `f(w) - f(w)` the left call
  runs first. A call that faults stops the statement before anything
  right of it runs.
- **An in-out takes a variable of its own type** (6.6.1.6, rule 6), and
  every call must pass it (E0016).
- **An array or a structure at an address, or reached through a
  reference, is given to an in-out as a copy too** (2026-11-11; florin
  has not decided this), and so to an `ARRAY[*]`: read before the call,
  stored back after it, not at all if the call faults. The reason is
  the one below: the image's bytes are no C object. A component of a
  structure with relative locations is given so too (2026-11-13), in C
  memory as at an address: it lies in its structure's bytes, not as a
  C object of its own. Siemens copies a
  structured in-out where the caller's memory and the block's differ:
  "If structures are transferred to a block as in/out parameters, they
  are transferred by default as pointers (Call by reference). However,
  this does not apply if the memory area of the transferred actual
  parameter and the called block have different optimization settings
  ... the parameter is always transferred as copy (Call by value)",
  and warns of what a copy means: "If the values are copied back again
  to the operands originally transferred as actual parameters after the
  block processing, the changes made asynchronously to these operands
  are overwritten." CODESYS and TwinCAT document no rule for located
  arrays. Two in-outs of one call given the same variable there are two
  copies; the one stored back last stays.
  [Siemens, copy or pointer](https://docs.tia.siemens.cloud/r/en-us/v20/programming-basics/block-calls/parameter-transfer-at-block-call/transfer-parameter-as-copy-or-as-pointer).
- **A variable at an address or of a subrange is given as a copy**, to
  a FUNCTION or a FUNCTION_BLOCK: read before the call, stored back
  after it as an output is - clamped into the subrange and flagged
  CONVERSION, as any store into one; not at all if the call faults; and
  after ENO is decided, so a clamp leaves ENO TRUE. An address's bytes
  are no C variable a pointer may reach, and a pointer would let the
  function write outside the subrange. So is an elementary component of
  a structure with relative locations (2026-11-13): its bytes, in
  C memory too, are read and written byte by byte. Siemens passes every elementary
  in-out so: "the value of the actual parameter is copied into the
  instance data or from the instance data to the actual parameter"
  (Siemens forum, quoting its help), copied back at the end of the call.
  The difference from a pointer shows only if the function also reaches
  the variable another way, as a VAR_EXTERNAL: it sees the value from
  before the call, and its writes there are overwritten by the store
  back.
- **VAR_EXTERNAL in a FUNCTION** is Ed.3's; Ed.2 had none, and the
  reference compiler refuses it.

A FUNCTION without a result (Ed.3 6.6.2.1) is called as a statement
only: "A function without result shall not be called inside an
expression" (E0016, E0014), and its name, which holds no result, is not
assigned (E0015). Ed.2 had none, nor has the reference compiler.

Fixed 2026-11-09: a call taking an output was run before its
statement's operands left of it, so `x + f(q => x)` read x after f had
stored it; now the order is the standard's (below).

The reference compiler takes a FUNCTION's outputs into variables; its C
for one into an array element does not compile ("assignment to
expression with array type", 7949c0b), and `NOT q =>` crashes it.

## SPLIT_DATE, SPLIT_TOD, SPLIT_DT outputs (Ed.3 Table 36)

The outputs are ANY_INT and "the Implementer specifies the provided data
types" (NOTE 2). iecf takes any integer variable for each; a part that
does not fit it wraps and is flagged, as any integer conversion
(PROPOSAL.md §6.3).

With EN and ENO (decided 2026-11-08, florin): EN FALSE leaves every
output as it was, as EN FALSE leaves any statement undone here; ENO is
FALSE when a part did not fit its output, the output's type being the
call's choice. Vendors:

- logi.CAD / Neuron (SPLIT_DT, a function with EN and ENO): ENO FALSE
  for an input past DT#2106-02-07-06:28:14.999, the other outputs then
  "in principle undefined"; with EN FALSE a function's outputs get their
  initial value unless the start option `lc3.fbdPreserveFunctionOutputs`
  keeps them. Nothing on an output too small for its part.
- PLCnext: with EN FALSE "the last valid output value is kept".
- No vendor documents whether a narrowed output clears ENO.

## CONCAT_DATE, CONCAT_TOD, CONCAT_DT: a part out of range (Ed.3 Table 36)

The inputs are ANY_INT; what an impossible date or time gives is the
implementer's. Decided 2026-11-08 (florin), after the vendors: **the
type's default value, flagged** `IECF_FAULT_CONVERSION`, the task going
on - `D#1970-01-01`, `TOD#00:00:00`, `DT#1970-01-01-00:00:00`. Out of
range: a month not 1..12, a day not in its month (2023-02-29,
04-31), an hour not 0..23, a minute or second not 0..59, a millisecond
not 0..999, any part negative, or a date or DT outside
1970-01-01 .. 2106-02-07-06:28:15. A DT's millisecond is checked, then
dropped, as any millisecond coming to a DT is.

- CODESYS (CAA DTUtil, DateConcat, DTConcat, TODConcat): "In case of
  invalid entries D#1970-01-01 will be returned"; `DT#1970-01-01-00:00`
  and `TOD#00:00` the same, with an error through a pointer.
- PLCnext (CONCAT_LDATE, CONCAT_LTOD): LD#1970-01-01 for a part out of
  range and for "Invalid date for month (e.g. 30.02.2021)", the TOD at
  midnight, ENO FALSE.
- matiec has none of Table 36's CONCAT and SPLIT functions.

Rolling over (2026-02-31 as 03-03, as C's mktime) and leaving the
statement undone were rejected: no vendor documents either.

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
| LDATE, LTOD, LDT | signed 64-bit, ns (since 1970, since midnight): 1677-09-21 to 2262-04-11 | 64-bit, ns: the same range | LTOD, LDT: ns |

TIME is signed because the standard has negative durations; the unit is
every vendor's. A TIME result outside the range wraps and is flagged,
as an integer's does (PROPOSAL.md §6.3).

### LDATE, LTOD and LDT

Decided 2026-11-08 (florin), after the vendors:

- **The range is all of the signed 64 bits**: LDT from
  1677-09-21-00:12:43.145224192 to 2262-04-11-23:47:16.854775807, LDATE
  from LD#1677-09-22 to LD#2262-04-11 (its midnights), LTOD up to
  23:59:59.999999999. Ed.3 Table 10 notes n to p make each "a signed
  64-bit integer with unit of nanoseconds"; the data type pages of
  CODESYS, TwinCAT and PLCnext give this range. CODESYS's and Beckhoff's
  literal pages give 1970 to 2554 instead, the unsigned range, against
  their own data type pages.
- **A long value made a short one drops what the short type is too
  coarse for, rounding down, unflagged** - LDT_TO_DT the fraction of a
  second, LTOD_TO_TOD, LDT_TO_TOD and LTIME_TO_TIME what is below a
  millisecond; `LTIME#-1500us` is `T#-2ms`. As a DT drops milliseconds
  (above).
- **A long value the short type cannot hold gives the short type's
  default value, flagged** `IECF_FAULT_CONVERSION`: LDT_TO_DT,
  LDT_TO_DATE and LDATE_TO_DATE outside 1970-01-01 ..
  2106-02-07-06:28:15 give 1970-01-01; LTIME_TO_TIME beyond TIME gives
  `T#0s`; LDT_TO_LDATE on 1677-09-21, which has no midnight an LDATE
  holds, gives LD#1970-01-01. As CONCAT_DATE's part out of range.
  Ed.3 Table 22: "Value range errors give an Implementer specific
  result"; Beckhoff's conversion page: "Undefined result if the value
  range is exceeded". No vendor documents a value.
- Following the short types' rules already decided, without a new
  choice: an LTOD wraps at midnight, unflagged; an LDT, or a difference,
  outside its type wraps and is flagged `IECF_FAULT_OVERFLOW`; DT, DATE,
  TOD and TIME widen exactly (Figure 11's implicit conversions).

No vendor page found documents conversion rounding, LTOD wrapping or
LDT overflow (CODESYS, TwinCAT and PLCnext data type and conversion
pages). matiec has none of the L types.

### A duration times or divided by a number (Ed.3 Table 30)

MUL_TIME and DIV_TIME take ANY_NUM; how the result becomes a TIME is
the implementer's. Decided 2026-11-07 (florin):

- **By an integer: exact**, an overflow wrapped and flagged
  `IECF_FAULT_OVERFLOW`, a division toward 0 as DINT's (`T#-7ms / 2`
  is `T#-3ms`); by 0 a fault.
- **By a REAL or LREAL: computed in LREAL, rounded to whole
  milliseconds half to even**, as REAL_TO_DINT rounds: `T#1s * 2.5` is
  `T#2s500ms`, `T#3ms * 1.5` is `T#4ms`. A result outside TIME, or
  infinite or NaN, is flagged `IECF_FAULT_CONVERSION` and its statement
  left undone, as any real made an integer (PROPOSAL.md §6.3); by 0.0 a
  fault. An LTIME is rounded to whole nanoseconds, in LREAL's 53 bits.

The vendors found keep less: Panasonic's MUL_TIME_REAL rounds the REAL
to a whole number before multiplying ("The REAL value is rounded off to
the nearest whole number"), PLCnext's MUL converts it to TIME first;
CODESYS and TwinCAT document neither. matiec holds nanoseconds in a
TIME, so its `T#10s / 3` is `3.333333333s`.

### DATE, TOD and DT at their edges

Decided 2026-11-07 (florin). No vendor documents these in a primary
source. PLCnext keeps a TOD of 24 hours or more as plain nanoseconds,
with no error (ADD_LTOD_TIME); CODESYS's TOD is an unchecked UDINT of
milliseconds; secondary sources say Siemens wraps.

- **A TOD wraps at midnight, unflagged**: `TOD#23:00:00 + T#2h` is
  `TOD#01:00:00`, `TOD#01:00:00 - T#2h` is `TOD#23:00:00`. A time of
  day is a clock, and each TOD stays one a literal can write.
- **A DATE or DT outside 1970-01-01 .. 2106-02-07-06:28:15 wraps and is
  flagged** `IECF_FAULT_OVERFLOW`, as an integer's overflow; so is a
  difference (`SUB_DT_DT`, `SUB_DATE_DATE`, `SUB_TOD_TOD`) outside
  TIME's ±24d20h31m23s647ms. Two dates 25 days apart already are: a
  longer span wants LTIME and LDT.
- **A DT holds whole seconds**: milliseconds that come to one -
  `DT + T#1500ms`, `CONCAT_DATE_TOD` of a TOD with them - are dropped,
  rounding down, unflagged, as CODESYS's DT (seconds) drops them. A DT
  literal with a fraction other than 0, or a TOD literal finer than a
  millisecond, is E0020, since iecf cannot hold it.
- DATE and DT start at 1970-01-01, so that is their default initial
  value (`D#1970-01-01`, `DT#1970-01-01-00:00:00`): Ed.3 Table 10 leaves
  it to the implementer whose first day is not 0001-01-01, and
  CODESYS's is the same.

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

## CHAR (Ed.3 6.4.2, Tables 6, 25, 27)

A CHAR is one Latin-1 character, its code in a byte, `'$00'` at start,
as a STRING's characters are. Of the vendors checked (2026-11-09) only
Siemens S7-1200/1500 ("CHAR 8 ASCII character set", its S7-1500
comparison table) and RuSTy have a CHAR; CODESYS, TwinCAT and PLCnext
list none, and give a string's character as a BYTE.

- **An untyped `'X'` of one character is a CHAR where a CHAR is
  wanted** - given to one, or compared with one - as Ed.3 Table 6 row 1b
  has it ("String of length one or character CHAR"). Siemens writes
  `varChar := 'A'`; RuSTy takes it as an initial value but wants
  `CHAR#'a'` in a comparison.
- **A CHAR goes where a STRING goes** (Figure 11: implicit), a string of
  that one character; `'$00'` gives the empty string, as `'$00'` ends
  any string. RuSTy refuses CHAR to STRING; iecf follows Figure 11.
- **`STRING_TO_CHAR` of an empty string gives `'$00'`, flagged
  CONVERSION.** Ed.3 Table 27: "if the string is empty the target
  variable is undefined". No vendor page says; RuSTy's library reads the
  string's first byte, so it gives `'$00'` with no flag. iecf gives the
  same value and flags it, as it does for any value a target cannot
  hold (an LDT before 1970 made a DT) (decided, florin, 2026-11-09).
- `CHAR_TO_BYTE`, `BYTE_TO_CHAR` and `CHAR_TO_WORD`, `_DWORD`, `_LWORD`
  are Table 25's binary transfers, `CHAR_TO_USINT` (Figure 11) the code;
  RuSTy has the first five and copies the byte.
- **A STRING's character, `s[i]`, is Ed.3's: from 1** ("starting with
  position 1", 6.4.2), **a CHAR**. Ed.3 6.4.2 Example 1 d) gives its
  equivalents, `STRING_TO_CHAR(MID(s, 1, i))` and `REPLACE(s, c, 1,
  i)`, and leaves a position outside the string open. A read there
  gives `'$00'`, flagged STRING_POSITION, as that MID gives. A write
  there leaves the string as it is, flagged STRING_POSITION: REPLACE's
  results for a bad P (appended past the end, empty below 1) are not
  taken - a write to a character the string has not changes nothing
  (decided, florin, 2026-11-09). Writing `'$00'` cuts the string
  there, as `'$00'` ends any string and as CODESYS and TwinCAT do, whose
  STRING ends with a 0 byte as iecf's ("value 0 will be the end
  terminator", CODESYS forum); Siemens, which keeps the length in a
  header byte, keeps the `'$00'` as a character inside (decided, florin,
  2026-11-09). CODESYS and TwinCAT count from 0 and
  give a BYTE (CODESYS's "Index access to pointers" page, TwinCAT's on
  infosys: "STRINGs are zero-based"); Siemens counts from 1 and gives a
  CHAR; RuSTy has none. No vendor documents a position outside the
  string.

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

## WCHAR and WSTRING's characters (Ed.3 6.4.2, Tables 6, 25, 27)

Ed.3 Table 10 gives WCHAR 16 bits, footnote l the character set ISO/IEC
10646, and Table 27 rows 3 and 8 name UTF-16. Decided 2026-11-10
(florin), after the vendors:

- **A WCHAR is one UTF-16 unit; a character above U+FFFF is two units
  in a WSTRING, a surrogate pair**, and LEN, the positions of Table 34
  and `ws[i]` count units. A WCHAR literal holds one unit, so
  `WCHAR#"${1F600}"` is E0020; a lone surrogate, `${D800}`, is no
  character, E0020. CODESYS's WSTRING page: "According to the IEC
  61131-3 standard, the data type WSTRING follows UCS-2 encoding ...
  exactly the fixed length 2 bytes", U+0000-U+D7FF and U+E000-U+FFFF:
  the same units for those, and silent above them.
- **A character a STRING cannot hold, one above U+00FF, is skipped by
  WSTRING_TO_STRING - a surrogate pair whole - and gives `'$00'` from
  WCHAR_TO_CHAR; both flagged CONVERSION.** Ed.3 Table 27 rows 1 and 6:
  "converted in an Implementer-dependent way". Beckhoff's Tc2_Utilities:
  WSTRING_TO_STRING2, "Characters that cannot be converted are skipped
  during the conversion"; WCHAR_TO_CHAR, "The conversion is possible
  only if the WSTRING character corresponds to the STRING character.
  Otherwise no character is returned." CODESYS's conversion pages say
  nothing; no Siemens page was found (third-party pages disagree: '?'
  or dropped). iecf skips as Beckhoff, and flags, as it flags any value
  its target cannot hold. STRING_TO_WSTRING and CHAR_TO_WCHAR give each
  Latin-1 code as its unit.
- **WSTRING_TO_WCHAR is Table 27's, though Figure 11 has none.** Ed.3
  contradicts itself: Table 27 row 2 defines it ("The first character of
  the string is transferred; if the string is empty the target variable
  is undefined"), and Figure 11 marks WSTRING to WCHAR "-" where it marks
  STRING to CHAR "e". iecf takes Table 27's (decided, florin,
  2026-11-10): the first unit; an empty string `"$0000"`, flagged
  CONVERSION, as STRING_TO_CHAR.
- An untyped `"X"` of one unit is a WCHAR where a WCHAR is wanted (Table
  6, 2b), as `'X'` is a CHAR; `'X'` is never a WCHAR, nor `"X"` a CHAR.
  `WSTRING#'..'` and `WCHAR#'X'` (Table 6, 4c and 4d) read `$hh`, as
  single quotes do; double quotes read `$hhhh` (Table 6, 2h).
  `WCHAR#16#263A` is the unit of that code, as `CHAR#16#41`.
- WCHAR_TO_WORD, `_DWORD`, `_LWORD` (Table 25) and WORD_TO_WCHAR
  (Figure 11) are binary transfers.
- A WSTRING's text read as a number (`WSTRING_TO_INT`) is read as a
  STRING's; a character outside ASCII as one no literal has.
  `<T>_TO_WSTRING` gives the STRING's text, each byte a unit.
- A WSTRING at an address is laid out as any part there (below): its
  units back to back, each little-endian, at an odd byte too; a WCHAR
  there is 16 bits, as a WORD. A reference to one reads and writes it
  there the same way (`iecf_wref_get`, `iecf_wref_put`).

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

### A variable's size at its address (Ed.3 6.5.5.3)

Ed.3 6.5.5.3: the address "defines the start address of the memory
location and does not need to be of equal or bigger size than the given
direct representation, i.e. empty memory or overlapping is permitted";
its own examples put a REAL at `%IB12`. CODESYS writes "The data type
should match the size prefix", and puts a BOOL at `%QW0` in its own
example; its pages say nothing of a mismatch. Decided 2026-11-08
(florin):

- **A variable starts at its address and takes its own type's size**
  from there, little-endian: BOOL a bit; BYTE, SINT, USINT 1 byte; WORD,
  INT, UINT 2; DWORD, DINT, UDINT, REAL, TIME, DATE, TOD, DT 4; LWORD,
  LINT, ULINT, LREAL, LTIME, LDATE, LTOD, LDT 8. A time or date type is
  its value as iecf holds it (TIME milliseconds, DATE seconds since 1970,
  ..., above), the bytes CODESYS gives (its TIME, DATE, TOD and DT are a
  DWORD, the long ones an LWORD). A size prefix other than the type's
  is warned (W0004), not refused.
- **A BOOL at a byte, word or longer takes that whole byte**, as
  CODESYS documents: TRUE written as 1, FALSE as 0, and any value but 0
  read as TRUE. Only a BOOL starts at a bit: a wider type at `%MX0.3`
  is E0022.

### Arrays, structures and strings at an address

Ed.3 6.5.5.3 places them too (`OUTARY AT %QW6: ARRAY[0..9] OF INT`) and
says nothing of how they lie. CODESYS and TwinCAT pad a structure to
natural alignment by default (pack mode 4 or 8, "depending on the
device description"), and `{attribute 'pack_mode' := '1'}` packs it with
"no memory gaps"; a BOOL in a structure takes a byte ("BOOL types, which
reserve 8 bits each"), bits are packed only in their own BIT type, which
an array may not hold; a STRING(n) is n + 1 bytes. Decided 2026-11-08
(florin):

- **No gaps**: from its address, an array's elements and a structure's
  fields back to back, in order, each its own size, as CODESYS's
  pack_mode 1 - the same on every target, and as a fieldbus lays out its
  process data. A multi-dimensional array goes row by row, its last
  index fastest.
- **A BOOL inside takes a byte**, 1 or 0, read TRUE for any value but 0,
  as a BOOL at a byte does. ARRAY[0..15] OF BOOL is 16 bytes.
- **A STRING[n] is n + 1 bytes**, its characters then a 0; bytes after
  the 0 are left as they were. **A WSTRING[n] is 2(n + 1) bytes**, its
  units little-endian then a 0 unit; units after the 0 are left as they
  were too.
- **W0004 compares the size prefix with the first element or field**:
  `ARRAY[0..9] OF INT` at `%QW6` is silent, at `%QB6` warned. An array,
  structure or string starts at a byte: at a bit it is E0022.
- Its initial value, if the declaration or its type gives one, is
  written at the start (not for `%I`, the platform's) - an
  enumeration's first value and a subrange's lower limit included,
  inside a structure or array too (until 2026-11-13 those were left
  at 0).
- Not yet: a RETAIN one, a FUNCTION_BLOCK parameter at an address
  (E0009).
- A structure with relative locations is laid out by its own offsets,
  at an address as anywhere (below).

### Structures with relative locations and OVERLAP (Ed.3 6.4.4.7)

A structure type may place its components itself, `head AT %B0 : INT;`,
`flag1 AT %X3.0 : BOOL;`, with gaps; with `STRUCT OVERLAP` components
may share bytes. The standard gives the offsets and leaves the rest
open: OVERLAP's "memory layout ... is Implementer specific", and
"Overlapping of strings is beyond the scope of this standard". None of
the vendors has this: TwinCAT refuses AT on a structure's component
("the AT declaration is not permitted here"); CODESYS and TwinCAT have
their own UNION, every member "the same address offset", "the largest
member" its size, a variable initialised by one member; PLCnext plans
relative addressing of structure elements through UNION types too; the
reference compiler (Ed.2) has neither. So iecf follows its own rules
for an address (above), the same on every host; implemented 2026-11-13,
the choices mine, for florin to confirm:

- **The structure is its bytes**, n of them, n its last component's
  last byte plus one: in C memory (`uint8_t b[n]`) as at an address,
  each component read and written byte by byte at its offset - so what
  one component of an OVERLAP writes another reads, and a STRING
  overlapping another component is its bytes there, read up to its 0 or
  its length.
- **Little-endian**, a BOOL at a byte takes the whole byte (1 or 0, TRUE
  for any value but 0), a BOOL at a bit that bit; an array or a plain
  structure inside lies back to back, as at an address.
- **The size prefix**: `%X` is a bit location and only a BOOL is at one
  (E0022); `%B` is a byte location for a component of any width, as the
  standard's own example has it (`head AT %B0: INT`, `data2 AT %B5:
  REAL`); `%W`, `%D` and `%L` name a width too, warned when it is not
  the type's (W0004). The number counts bytes.
- **Initial values**: the bytes 0, the gaps too, then each component its
  own. In an OVERLAP none is given (6.4.4.7.2, E0034), so each
  component takes its type's; where two share bytes, the one declared
  first has its own there - the first member of a C union is the one
  initialised, and CODESYS initialises a UNION through one member.
- **Refused**: a component without a relative location among ones with
  (E0034: "the name of each component ... shall be followed by the
  keyword AT"), two sharing a bit without OVERLAP (E0034), a full
  address on a component (E0022: every variable of the type would be
  there), a relative location outside a structure (E0022).
- **STRUCT OVERLAP without relative locations is refused** (E0034):
  nothing says which components share which bytes. Placing each at
  `%B0` gives what a CODESYS or TwinCAT UNION is. Decided 2026-11-13
  (florin): refused for now; a CODESYS-like UNION may come later as an
  extension, proposed in docs/extensions.md.

## A bit string to BOOL (Ed.3 Figure 11, Table 25)

Figure 11 defines no conversion from BYTE, WORD, DWORD or LWORD to BOOL;
Table 25 lists them as "Binary transfer of the rightmost bit into the
target". CODESYS and TwinCAT accept them and give TRUE for any value but
0. Decided 2026-11-08 (florin, PROPOSAL.md §6.8): as CODESYS and
TwinCAT, and an extension, off by default as every extension is
(`bit-string-to-bool`, [extensions](extensions.md)); without it, E0013.

## References (Ed.3 6.4.4.10)

`REF_TO`, `REF()`, `^` and `NULL`. Ed.3 calls dereferencing NULL "an
error" and leaves the check to the implementer ("at compile time, by the
runtime system, or by the application program"). CODESYS and TwinCAT
have no REF_TO: their POINTER TO and REFERENCE TO are set with ADR() and
`REF=`; on an invalid pointer TwinCAT's runtime "is stopped as soon as
any access takes place via this pointer", unless the project's
CheckPointer function, which has "no default implementation", catches
it; CODESYS cautions not to "return Pointer-TO variables of functions
and methods to the caller or assign them to global variables", and
warns ("invalid assignment target") when a reference is taken to an
input. The reference compiler accepts REF_TO with its `-r` option and
dereferences a raw C pointer. Siemens' pages were not reached. Decided
2026-11-08 (florin):

- **A NULL reference dereferenced is a fault**, `IECF_FAULT_NULL`, as an
  index out of range is: the statement stores nothing and the task
  stops.
- **REF() of a temporary is an error** (E0026), as Ed.3 says: a
  VAR_TEMP, or any variable of a FUNCTION but a VAR_EXTERNAL. So is
  REF() of EN or ENO (Ed.3 6.6.1.5, rule 8).
- **REF() of what cannot be assigned is an error** (E0026): a CONSTANT,
  a function block's output taken from outside. Ed.3 says nothing; a
  reference would write past the checker.
- **REF() of a variable at an input address warns** (W0005), as CODESYS
  does: the next inputs overwrite what is written through it.
- **A reference into the process image** is built: its area, byte and
  bit, read and written as the variable is there (docs/c-output.md §4).
- **A RETAIN reference is an error** (E0027), and so is a retained
  structure, array or function block instance that holds one: after a
  restart it would reference nothing. So is a reference at an address,
  and, as Ed.3 says, a reference as a VAR_IN_OUT.
- **Two references compare with = and <>**, equal when they reference
  the same variable, and either with NULL. Table 12 lists only the
  comparison with NULL; the reference compiler compares two as well.
  Ordering them would be the reference arithmetic Ed.3 calls "not
  recommended".
- A reference takes only its own type: arrays of other bounds, or a
  subrange and its base, are other types (E0013). That holds for any
  array assignment too, which iecf used to let through for arrays of
  one element type and number of dimensions.
- CODESYS's REFERENCE TO is built as an extension, off by default as
  every extension is (`reference-to`, [extensions](extensions.md)): a
  REFERENCE TO is a `REF_TO` dereferenced wherever it is used, so an
  unbound one used faults as NULL does. What the CODESYS and TwinCAT
  pages do not say, iecf infers: a call that does not pass a function
  block's REFERENCE TO input leaves it bound as it was (an IEC input
  keeps its value), and `r.f`, `r[i]` and `r(...)` go through it.
- Not yet: a FUNCTION's input or result of a reference type (E0009), as
  for every type that is not elementary.

### Interfaces kept and connected (Ed.3 6.6.6.5)

Decided 2026-11-11 (florin), after the vendors:

- **A RETAIN interface is an error** (E0027), as a RETAIN reference is,
  and so is a retained structure, array or block instance that holds
  one: Ed.3 calls an interface "a reference to an instance", and CODESYS
  "always a reference to the assigned function block instance". After a
  new build the instance may be elsewhere. CODESYS and TwinCAT only warn
  of a POINTER TO in retained memory ("their addresses could change"),
  and move interface references themselves at an online change, which
  iecf does not do; neither documents an interface in RETAIN. An
  interface in an instance kept whole for a RETAIN variable of its own
  is kept across a warm start of the same build, as a reference there
  is.
- **A PROGRAM's input or output of an interface is connected** as an
  elementary one is (6.8.2): an input set before each run from a global
  interface, a global instance or NULL; an output stored after it into
  a global interface, of its own or a base interface. CODESYS and
  TwinCAT give a program's inputs by a call (`PLC_PRG(in1 := 2)`) and
  have no CONFIGURATION connections, so no vendor says more.

Sources: [CODESYS, interfaces](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_implementing_interface.html),
[CODESYS, PERSISTENT](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_var_persistent.html),
[Beckhoff, remanent variables](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/27021597900419211.html),
[CODESYS, program](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_obj_program.html).

## Array bounds and ARRAY[*] (Ed.3 6.4.4.5, 6.5.3)

Chosen 2026-11-11, after the vendors, where Ed.3 leaves it open:

- **A constant in a bound** is a POU's own `VAR CONSTANT`, or a
  `VAR_GLOBAL CONSTANT` through its `VAR_EXTERNAL CONSTANT` (Ed.3
  6.6.1.2.4: "constant variables"). A `TYPE` declaration sees the global
  constants directly: Ed.3 does not say what a TYPE outside a
  CONFIGURATION sees, and CODESYS takes a GVL's `VAR_GLOBAL CONSTANT`
  in any declaration's bounds (its error C0161, "Border ... of array is
  no constant value", for a plain `VAR_GLOBAL`). A name the POU declares
  hides a global constant of that name.
- **Bounds are within DINT**, as CODESYS documents ("maximum of the
  data type DINT"): LOWER_BOUND and UPPER_BOUND give a DINT, as
  PLCnext's do.
- **`ARRAY[*]` is an in-out**, of a FUNCTION, a method or a
  FUNCTION_BLOCK: where CODESYS and TwinCAT allow it ("only allowed in
  VAR_IN_OUT", Beckhoff).
- **`ARRAY[*]` is also a FUNCTION's or method's input or output, the
  caller's array itself and not a copy** (decided 2026-11-11, florin:
  "the Siemens way"). Ed.3 allows the declaration and says nothing of
  how it is passed. Siemens' Programming Guideline for S7-1200/S7-1500
  (V1.5, Table 3-5, "Overview for transfer of parameters") has an FC's
  Input, Output and InOut of a structured data type all passed as
  "Reference", and an FB's Input and Output as "Copy", its InOut as
  "Reference"; its 3.6.3 has `Array [*]` "transferred to functions and
  function blocks". So in iecf: the input is read only (E0015 to write
  it), each is given an array at every call (E0016), and a
  FUNCTION_BLOCK takes one as an in-out only, its inputs and outputs
  being kept in the instance. Beyond Siemens, iecf sets the output's
  elements to their initial value as the call starts, as Ed.3 6.6.2.2
  has a FUNCTION's variables. Siemens' help says the same of where it
  is declared: "In functions (FC) you can use ARRAY[*] in all
  declaration subsections. In function blocks (FB) you can declare
  ARRAY[*] only as an in-out parameter in the "InOut" section."
- **A constant expression in an initial value is worked out by the
  compiler** (2026-11-11; florin has not decided this), with the
  constants a bound names; a method also names its type's and its
  type's bases', and a `TYPE`'s and a `VAR_GLOBAL`'s initial value the
  global constants directly, as a `TYPE`'s bound does. Ed.3 Figure 6
  has "constant expression" for each type and its grammar says only
  "a constant expression must evaluate to a constant value at compile
  time". So: an integer one is worked out exactly, as a bound is, and
  must fit the variable (E0020) - `K * 200` with K an INT of 300 is
  60000, refused for an INT and taken by a DINT, where the same
  expression in a statement wraps in INT and is flagged; a REAL one in
  single precision, an infinite result E0020; a division by zero and a
  constant that names itself E0032. No vendor's documentation was
  checked for these.
- **A standard function of constants in an initial value is worked out
  too, and a variable there is refused** (2026-11-12; florin has not
  decided this). Those whose result is exact - `ABS`, `MIN`, `MAX`,
  `LIMIT`, `SEL`, `MUX`, the shifts, the conversions between numbers, a
  duration times a number - are worked out as the operators are, what
  the run would flag being E0020 (`INT_TO_SINT(300)`) and what it would
  fault on E0032 (`MUX(2, 5, 6)`). The mathematics of Table 28 and
  `**` come from the target's `<math.h>` or the platform's own
  (docs/c-output.md §1), so the compiler cannot give their value: it
  works them out only to tell that the result is a number the type
  holds - refusing one within a factor of four of the type's largest,
  E0020 - and the target computes it as the variable is set up. A
  variable or a FUNCTION of the program's in an initial value is E0032:
  Ed.3's grammar has a constant expression there. CODESYS takes any
  expression ("Initialization with ... any valid ST expression", and
  computes it as the application starts) and so does TwinCAT; that
  could be an extension, and is not one yet. Quoted from memory of the
  CODESYS help, not checked against it.
- **A part of a constant array or structure is a constant** in an
  initial value (2026-11-12; florin has not decided this): `KA[2]`,
  `KP.x`, the index a constant expression. Ed.3's grammar does not say
  whether a constant's element is a constant expression; CODESYS and
  TwinCAT take one (from memory, not checked). The index is checked by
  the compiler: outside the array is E0032, where the run would fault.
- **A whole `ARRAY[*]` is assigned from another or from an array of
  fixed bounds; never to an array of fixed bounds** (2026-11-11; florin
  has not decided this). Ed.3 says nothing of it. Siemens: "The
  assignment of ARRAY[*] to ARRAY[*] is permitted if the number of
  dimensions and the data type match", "The assignment of an ARRAY with
  known limits to an ARRAY[*] is permitted", "The assignment of
  ARRAY[*] to ARRAYs with fixed limits is not permitted" (said of
  supplying parameters), and of two arrays compared, "The exact ARRAY
  limits do not have to match". CODESYS and TwinCAT document no
  assignment of one. What Siemens does when the sizes differ as it runs
  is not documented; iecf faults (INDEX) and copies nothing, as for an
  index out of bounds.
- **Arrays, structures and instances are not compared** (E0014): Ed.3
  Table 33 compares ANY_ELEMENTARY. CODESYS ("All elementary data types
  are permitted as data types for the operands"; C0069 "Compare not
  possible on objects of type ..." for arrays), TwinCAT (the same
  sentence) and PLCnext (IN1, IN2 "ANY_(SAFE)ELEMENTARY") refuse it.
  Siemens SCL alone compares them, with `=` and `<>` only ("PLC data
  types; ARRAY; STRUCT", S7-1500 firmware 2.0 on). iecf follows the
  standard; Siemens' comparison would be an extension
  (docs/extensions.md), not built.
- **An array indexed in fewer dimensions than it has** is the array of
  the others: `A2[2]` of an `ARRAY [1..20, -2..2]` is an `ARRAY [-2..2]`
  (decided 2026-11-11, florin). Ed.3's Example 2 writes `SUM(A2[2])`;
  no rule of 6.4.4.5 states it, and CODESYS, which has both this form
  and the array of arrays ("an alternative syntax for multidimensional
  arrays"), documents a row of neither. iecf takes the example as the
  rule, for both forms, and anywhere an array is used, not only as an
  argument.
- **A dimension out of the array's**, `LOWER_BOUND(a, 3)` of a
  two-dimensional `a`, is an error when it is a constant (Ed.3 Table 15:
  "error"), and a fault `INDEX` when it is computed, as an index out of
  bounds is. PLCnext gives 0 and ENO FALSE; with no ENO on LOWER_BOUND
  in Ed.3, iecf stops the statement as any out-of-range access.

Sources: [CODESYS, ARRAY](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_array.html),
[CODESYS, C0161](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_error_c0161.html),
[Beckhoff, declaring an array](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2526591371.html),
[PLCnext, LOWER_BOUND](https://engineer.plcnext.help/latest/LOWER_BOUND.htm),
[Siemens, Programming Guideline for S7-1200/S7-1500](https://media.automation24.com/manual/se/90885040_81318674_Programming_guideline_DOC_v15_en.pdf),
[Siemens, basic information on ARRAY](https://docs.tia.siemens.cloud/r/en-us/v20/data-types/array/basic-information-on-array),
[Siemens, transferring ARRAY tags](https://docs.tia.siemens.cloud/r/en-us/v20/data-types/array/transferring-tags-of-the-array/array-data-type),
[Siemens, relational expressions](https://docs.tia.siemens.cloud/r/en-us/v20/creating-scl-programs/basics-of-scl/relational-expressions),
[CODESYS, EQ](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_operator_eq.html),
[CODESYS, C0069](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_error_c0069.html),
[Beckhoff, EQ](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529004427.html),
[PLCnext, EQ](https://engineer.plcnext.help/latest/EQ.htm).

## Function block instances as values (Ed.3 6.6.1.4, 6.6.3.4, Table 72)

Chosen 2026-11-11, after the vendors (florin has not decided these).
They are for a plain function block; an instance of a class or of a
block with methods, a base or an interface is neither copied nor an
input ([the objects proposal](design/objects.md) D11).

- **`b := a` copies every variable of the instance**, the instances in
  it too. Ed.3 has the assignment as a feature (Table 72, 1d,
  "Instances of function block type") and says nothing of what it
  does. CODESYS: "When an instance is assigned to another instance, all
  elements and subelements are copied from the one instance to the
  other instance" (rule SA0014, a performance warning; an error only
  under its `no_assign` attribute). Beckhoff: "When such an assignment
  is made, a copy of the object is made", and advises against it for a
  block that holds addresses. PLCnext refuses it ("The assignment of
  function blocks is not supported", SemError 1134), and Siemens does
  not claim the feature (its Ed.3 compliance list of 2014 leaves 1d
  unticked). iecf copies, as the two that allow it do, and so copies an
  array or a structure that holds instances.
- **An instance given as a VAR_INPUT is a copy**, read in the POU that
  takes it, not written and not called. The read-only part is Ed.3's
  (6.6.1.4 rule 4: "if declared as VAR_INPUT the function block
  variables can only be read"; 6.6.3.4 rule 7 lets only a VAR_IN_OUT or
  VAR_EXTERNAL one be called). That it is a copy is CODESYS's ("The
  transfer method used is by value", "the callee receives a private
  copy") and TwinCAT's, and PLCopen's FAQ says the same of the
  standard: "Function block instances should be passed as VAR_IN_OUT
  parameters otherwise the values produced within the internal
  invocation will not be preserved". Siemens passes an instance as an
  InOut only ("Instances of called blocks can be defined as InOut
  parameters", Programming Guideline 3.2.6).
- **Two instances are not compared**: no vendor documents `a = b` for
  them (CODESYS's EQ takes "all elementary data types"), so it is
  E0014.

Sources: [CODESYS, SA0014](https://content.helpme-codesys.com/en/CODESYS%20Static%20Analysis/_san_rule_sa0014.html),
[CODESYS, no_assign](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_pragma_attribute_no_assign.html),
[CODESYS, parameters](https://content.helpme-codesys.com/en/LibDevSummary/parameters.html),
[Beckhoff, use of function blocks](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/12050023819.html),
[PLCnext, SemError 1134](https://engineer.plcnext.help/latest/SemError_1134.htm),
[Siemens, IEC 61131-3 compliance](https://cache.industry.siemens.com/dl/files/748/109476748/att_845621/v1/IEC_61131_compliance_en_US.pdf),
[PLCopen, FAQ](https://www.plcopen.org/standards/logic/iec-61131-3/faqs/).

## Enumerations and named values (Ed.3 6.4.4.2, 6.4.4.3)

Chosen 2026-11-11, after the vendors, where Ed.3 leaves it open
(florin has not decided these):

- **A named value not given a number** is one more than the value
  before it, the first 0. Ed.3's grammar lets the number be left out
  and says nothing of what it is. CODESYS: "Starting at 0, the values
  are continuously incremented by 1"; Beckhoff's example `eRed,
  eYellow, eGreen := 10, eBlue, eBlack` gives "eBlue = 11, eBlack = 12".
- **The base type** is an integer or a bit string type but BOOL, as
  CODESYS lists them (INT to ULINT, BYTE to LWORD). With none named it
  is INT, as CODESYS and TwinCAT take it (decided 2026-11-11, florin;
  it was DINT before, for no reason but being iecf's first choice), for
  an enumeration and for a type with named values alike. A value an INT
  cannot hold is E0020, and needs the type named: `DINT (far :=
  100000)`.
- **A name two in-place declarations with named values share** is used
  without a type's name where both give it the same number and base
  type (2026-11-11; florin has not decided this): an in-place
  declaration has no name to write before `#`, and used as the base
  type the value is the same whichever is meant. Otherwise it is the
  usual error, "a value of 2 enumerations".
- **A type with named values is used as its base type** (6.4.4.3:
  "other constants can be assigned, or can arise through
  calculations"), which is the standard's text. CODESYS and TwinCAT
  refuse arithmetic and other constants under their `strict` attribute,
  which both now add by default; iecf follows the standard, and its
  plain enumerations (6.4.4.2) are the strict kind.

Sources: [CODESYS, enumerations](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_datatype_enum.html),
[Beckhoff, enumerations](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529504395.html).

## Namespaces (Ed.3 6.9)

Decided 2026-11-09 (florin), each against the vendors in
[the proposal](design/namespaces.md) §2: Siemens TIA Portal V21 and
PLCnext Engineer, the two that put namespaces in programs, write the
global namespace `_.`; CODESYS and TwinCAT have only a library's
namespace and `qualified_only`; RuSTy and matiec have none. Neither
Siemens nor PLCnext documents what a short name two namespaces have
means, so iecf takes C#'s rules, whose wording Ed.3 6.9.4 follows.

- A PROGRAM may be in a namespace (6.9.2's text; its grammar leaves
  programs out), and a USING may follow a PROGRAM's name (6.9.4: "a
  POU").
- A short name is found from the inside out: the POU, its namespace,
  each one around it, the global namespace; at each, a namespace's own
  element before what its USINGs bring in. Two USINGs bringing in one
  name is an error where it is used (E0029), as C#. A dotted name's
  first part is found the same way, as Siemens's "searches relative to
  the calling block".
- A USING at the top of a file is that file's, as C#'s.
- A namespace holds no variables (Annex A); `N.x` is E0028.
- A namespace and an element of one name side by side is E0012, as
  Siemens ("Do not give names to program elements that you already use
  as a namespace") and CODESYS (a namespace may not be a data type's
  name).
- Ed.3's `PUBLIC.TON` for the standard timer cannot be parsed; `_.`, as
  Siemens and PLCnext, is the extension `global-namespace`
  ([extensions](extensions.md)).
