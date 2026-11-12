# The harness: matiec's C and iecf's, run side by side

(It also runs any program on both compilers, side by side, and compares
every elementary variable of its symbol table tick by tick, not only the
Q and M bytes: `iecf-harness compare --reference DIR FILE.st...`, on
pseudo-random inputs, the same for both and at every run (src/compare.rs).
A host given `vars 0 0 00` as the script's first line prints the
variables after each line: iecf's from the symbol table, matiec's
through a `vars.c` the harness writes from its VARIABLES.csv
(src/vars.rs), for the same leaves: every array element (the first 64
of a big array), structure field and function block member, named from
iecf's `<config>.symbols.json`. CI runs it with `--expect-same` on the cases
`compare-cases` lists: random inputs reach where iecf decided
differently in the others, and that file says where.

It also checks the book's examples: `iecf-harness book` builds and runs
every row of each table marked `<!-- checked -->` under docs/, and
compares the result and the flag with the row; src/book.rs.)

PROPOSAL.md §5 step 2: iecf's C reaches matiec's parity, compared in CI.
This directory is that comparison. matiec is the *reference compiler*: the
harness's files, commands, options and CI steps say "reference"
(`reference_host.c`, `reference-skip`, `iecf-harness reference
--reference DIR`), and only these documents name it.

## A case

`cases/<name>.st` is a program and its CONFIGURATION, plain IEC 61131-3,
the same file for both compilers. `cases/<name>.cycles` says how many
ticks to run and what the inputs are:

```text
extension reference-to  # optional: an extension iecf builds it with
ticks 6                 # one tick is the CONFIGURATION's base interval
0 %IB0 16#03            # at tick 0, before it runs
3 %IX1.2 TRUE
4 %IW2 -5
5 warm                  # a warm restart before tick 5: iecf's <p>_warm,
                        # matiec's config_init__ (it has no warm one)
```

An `extension NAME` line switches on an extension of
docs/extensions.md for iecf; the case is then not IEC 61131-3, and in
reference-skip.

A case can be more than one file: each `.st` in `cases/<name>.d/` is
built with `<name>.st`, in name order, as `iecf build` takes a project
of several files. Only iecf builds those (`namespaces`: a namespace two
files add to); such a case is in reference-skip.

The program shows its results by writing them to `%Q` and `%M`.
`cases/<name>.trace` is the trace: one line after initialisation and
one per tick, the Q and M bytes in hex, as far as the program addresses
them. Addresses count bytes and are little-endian (docs/c-output.md §4).

The trace is matiec's, written by the harness and checked by hand. A case
where iecf decides differently from matiec on purpose - overflow, the
counters' limits (PROPOSAL.md §1b) - has `cases/<name>.iecf.trace`
beside it, and the reason in the case's comment: `overflow` and
`narrow` (an integer overflow or conversion out of range flagged; the
bytes are matiec's), `edges` (the counters count over their type's
range), `infinity` (an infinite or NaN real flagged) and `unconverted`
(a real that does not fit an integer leaves its statement undone, where
matiec's C is undefined), `subrange` (a value outside a subrange
clamped and flagged, where matiec stores it), `strings` (a string
position past the end flagged), `dates` (a TOD wrapped at midnight,
where matiec's runs past 24 hours), `durations` (a TIME in whole
milliseconds, where matiec's holds nanoseconds), `sfcdemo` (SFC: actions
before transitions and a final scan, where matiec tests transitions
first), `enable` (EN = FALSE
leaves the statement undone, where matiec assigns 0; ENO FALSE on the
function's own overflow), `fbenable` (a block with EN = FALSE leaves its
outputs untaken, where matiec takes them; ENO FALSE on an overflow in
its body), `enobody` (a body's `ENO := TRUE` cannot hide an overflow,
where matiec has no flag), and `retain` and `fbretain` (RETAIN
kept across a warm restart, which matiec's host does as a cold one). A flag ends the line with
`flag CODE POU LINE`.
A fault ends the line with `fault CODE POU LINE` (docs/c-output.md §5).

The cases cover the Ed.2 core a slice at a time: `image` (each size of
address), `arith`, `bits`, `control` (every ST statement), `timers`,
`edges` (counters, edges, bistables), `pous` (user FUNCTIONs and
FUNCTION_BLOCKs, VAR_IN_OUT, globals), `types` (enumerations, subranges,
arrays, structures), `real`, `functions` (the operators written as functions, SEL, EXPT),
`strings` (STRING and Table 34's functions), `dates` (DATE, TOD and DT,
Table 30), `parts` (CONCAT_DATE, CONCAT_TOD, CONCAT_DT and the SPLITs,
Table 36; reference-skip: matiec has none of them), `splitwrap`,
`splitindex`, `splitclamp` (a SPLIT's part too big for its output,
wrapped and flagged; an output element out of bounds, nothing stored;
a part clamped into a subrange; reference-skip too), `fbenable` (EN and
ENO on a function block call; matiec takes the outputs with EN FALSE),
`fbskip` (EN FALSE evaluates no input; reference-skip: matiec cannot
compile `NOT ENO =>`), `progio` (a program's VAR_IN_OUT bound to a
global in the CONFIGURATION; reference-skip, as `connections`),
`enobody` (ENO set in a FUNCTION's and a FUNCTION_BLOCK's body),
`splitenable` (EN and ENO on a SPLIT; reference-skip, as `parts`),
`ldates` (LDATE, LTOD, LDT and their conversions; reference-skip: matiec
has no L dates), `located` (time and date types at an address, each its
own width, a BOOL at a byte; reference-skip: matiec refuses an LTIME
there, and its TIME is a structure), `locdata` (arrays, structures and
a STRING at an address, packed; reference-skip: matiec refuses an array
there), `refs` (REF_TO, REF(), `^`, NULL and comparisons, on both
compilers: matiec's iec2c is given `-r`, which switches Ed.3's references
on), `refimage` (references into the image, to function blocks and to
references, REF() as an initial value, a NULL reference dereferenced;
reference-skip: matiec's C for `r^ := ...` does not compile and REF_TO a
function block crashes it), `refto` (CODESYS's REFERENCE TO: REF= to a
variable, another REFERENCE TO, an element, an address, a structure
and a block, a block's REFERENCE TO input, __ISVALIDREF, an unbound
one used; the extension `reference-to`, reference-skip), `widen`
(implicit conversions in an assignment, to a block's input and from its
output, to a function's input, between operands; INT * INT overflowing
as an INT into a DINT; reference-skip), `fnout` (a FUNCTION's
outputs into variables, at an address, their initial values at each
call, RETURN; on both compilers), `fnoutx` (negated, widened, in order,
into an element, a call that faults, a FUNCTION without a result;
reference-skip), `fninout` (a
FUNCTION's in-outs, passed on to another; on both compilers), `fnorder`
(a FUNCTION's externals, and the order of evaluation once calls have
effects: left operand first, a fault before a call stops it;
reference-skip), `chars` (CHAR: literals, at an address, in an array
and a structure, in calls, to and from STRING and BYTE; reference-skip:
Ed.3's), `implicit` (the implicit-externals extension in a PROGRAM, a
FUNCTION_BLOCK and a FUNCTION; reference-skip), `fneffen` (EN and ENO
on a FUNCTION with an in-out, an output, a VAR_EXTERNAL, and without a
result; reference-skip, as `fnorder`), `inoutcopy` (an in-out given a
variable at an address or of a subrange, a copy stored back, clamped;
reference-skip), `stringchars` (`s[i]`
read and written, past the end and below 1, in an array of strings;
reference-skip: Ed.3's), `wstrings` (WSTRING and WCHAR: a surrogate
pair, Table 34's functions, `ws[i]`, in an array, a structure and a
block, to and from STRING, CHAR, WORD and INT, a character a STRING
cannot hold skipped and flagged; reference-skip: Ed.3's), `wlocated` (a
WSTRING at an address, at an odd byte, in a structure and an array
there; reference-skip), `methods` (CLASS and METHOD: Ed.3's COUNTER,
THIS, a PUBLIC variable, a block with methods and a body, an in-out, an
input set from outside, a method of an instance in an instance, EN;
reference-skip), `inherit` (EXTENDS, OVERRIDE, SUPER: Ed.3's CIRCLE
and LIGHT2ROOM, Figure 21's SUPER() chain, a derived block behind a
base-typed in-out and reference, an ABSTRACT class as a FUNCTION's
in-out, a method through a NULL reference; reference-skip),
`interfaces` (INTERFACE, IMPLEMENTS, ?=: Ed.3's ROOM and ROOM_CTRL,
THIS given to an interface, an interface extending another, an
ABSTRACT block leaving a method to its derived type, an interface as a
FUNCTION's input, ?= into an interface and a reference and between
references, a method through a NULL interface; reference-skip),
`itfplaces` (interfaces wherever a variable can be: array elements and
structure fields, a VAR_GLOBAL, initial values that are a local's and
a global's instance, a FUNCTION's result, outputs and an input left to
its initial value, a block's output taken into an element and read
from outside, a method's result through an interface's table and a
method's output, ?= into an element, = of elements, NULL in a field
faulting; reference-skip), `itfprog` (a PROGRAM's interface inputs and
outputs connected in the CONFIGURATION: from a global instance, a global
interface and NULL, into a global interface of a base type;
reference-skip), `flexarrays` (ARRAY[*] in-outs of a FUNCTION, a
method and a FUNCTION_BLOCK, Ed.3's SUM and a two-dimensional one,
passed on, LOWER_BOUND and UPPER_BOUND, a dimension out of range
faulting; a block's in-outs of an array, a structure and a STRING;
bounds, a STRING's length, a subrange and a CASE's labels naming
constants; a row of an array of arrays and of an array of two dimensions given
to an ARRAY[*], copied, at an address, and taken of an ARRAY[*, *];
reference-skip), `flexio` (ARRAY[*] inputs and outputs of a FUNCTION
and a method, the caller's array itself: read, passed on, an output's
elements set to their initial value first, given by name and in order,
a constant array and a row given, an index out of range faulting;
reference-skip), `instvalues` (a function block instance as a value:
assigned, in a structure and an array copied, an input of a FUNCTION
and of a block, a block's output, a standard block copied, an input
set apart from a call, a FUNCTION's result, output and own instance;
reference-skip: matiec refuses an instance in a structure, as a
FUNCTION's result and as a FUNCTION's variable),
`callparts` (a part of a FUNCTION's result read at the call: a field,
an element by a constant and a computed index, a variable of an
instance, in an expression and a condition, a call with effects made
once, an index out of range faulting; reference-skip: matiec refuses
a part of a call), `holders` (a structure and an array that hold instances as a block's
and a FUNCTION's input, output, in-out and result, an ARRAY[*] of
instances, a PROGRAM's connections of an array, a structure and an
instance; reference-skip), `flexcopy` (a whole ARRAY[*] assigned from another and from an array
of fixed bounds, in two dimensions, onto itself, sizes that differ
faulting; reference-skip), `inoutdata` (an array and a structure at an address, a row of one, and
one through a reference, given to an in-out and to an ARRAY[*] input,
output and in-out of a FUNCTION and a block; reference-skip), `fbargs` (a block's in-out given and its output taken into an element
by a constant and a computed index, a field, a variable through a
reference and one at an address, widened and negated; an enumeration
in place with named values; reference-skip), `constinit` (initial values that are constant
expressions: constants of each type, of constants, global, a block's in
its method and in a derived block, in a TYPE; reference-skip), `constfns` (initial values that are standard functions of
constants: worked out by the compiler, the mathematics of Table 28
checked by it and computed by the target as the variable is set up - in
a program, a TYPE, a field, an array, a global, a block's and a
FUNCTION's input; a part of a constant array or structure; reference-skip), `selectdata` (SEL, MUX and MOVE of arrays, structures, instances,
interfaces and references, NULL among them; MUX's K out of range last;
reference-skip), `extcompare` (GT, GE, EQ, LE, LT of more than two inputs, a FUNCTION
with effects among them called once; the negation of a TIME; the
shifts of a BOOL; reference-skip), `subparams` (a subrange as a FUNCTION's input, output, in-out and
result and a block's in-out; a subrange starting at its lower limit;
reference-skip), `stdinout` (a standard block's instance as a VAR_IN_OUT of a
FUNCTION, one that passes it on, and a FUNCTION_BLOCK; reference-skip), `litcalls` (a standard function given literals alone, a quotient of
real literals: computed in the type wanted, integers staying integers
where a real is wanted; reference-skip), `enums` (types with named values used as their base
type, a value not given, a WORD base, INT when none is named; an enumeration as a FUNCTION's
input, output and result and a block's in-out, a result not set and an
input not given the type's initial value; one declared in place;
reference-skip),
`fnparams` (a FUNCTION's and a method's STRING, array and structure
inputs, outputs, in-outs and results: a STRING input cut and flagged
where it is declared, an input left to its initial value, an array
result given to another's input, a STRING result through an interface
and an override, a WSTRING one; reference-skip), `instances` (instances besides a POU's own variable: an ARRAY OF TON
given `[2(PT := T#20ms)]`, arrays of a block and of a class each
element its own initial value, a STRUCT with a TOF field, called and
read there, an element given to an in-out and an interface, an index
out of bounds faulting before the call, a block holding an array of
instances, global instances through VAR_EXTERNAL, RETAIN and plain
arrays of a block that keeps a count across a warm restart;
reference-skip), `wrefs` (REF_TO a WSTRING at an odd byte and
in C memory, one reference switching between them, read, stored, by
`r^[i]`, a structure holding one copied through a reference;
reference-skip), `durations` (TIME times and divided by a number), `enable` (EN and ENO
on calls), `sfcdemo` (Prokop's SFC demo), `sfc` (every qualifier,
divergence and convergence, PRIORITY; reference-skip: matiec reads no
action's Q nor PRIORITY), `blockdata` (arrays, structures and STRINGs in and out of a
FUNCTION_BLOCK), `convert` (to and from
STRING, where matiec's text is iecf's),
`tasks` (two intervals), `temp` (VAR_TEMP), `fbretain` (RETAIN and
NON_RETAIN inside a FUNCTION_BLOCK),
`overflow` and
`narrow` (where iecf flags), `infinity`, `unconverted`, `subrange`. Apart
from those five they stay
inside what both compilers define: no overflow, no conversion out of
range.
A test runs `iecf check` on each: errors fail it, warnings (a deprecated
form both compilers read) do not.

## Running it

```text
cargo run -p iecf-harness -- reference --reference ~/matiec          # compare
cargo run -p iecf-harness -- reference --reference ~/matiec --bless  # write the traces
cargo run -p iecf-harness -- reference --reference ~/matiec timers   # one case
cargo run -p iecf-harness -- iecf                                    # iecf's C
cargo run -p iecf-harness -- iecf --bless overflow                   # write its .iecf.trace
```

`--reference` (or `$IECF_REFERENCE_DIR`) is a built `beremiz/matiec` checkout:
`iec2c` at its top, `lib/` beside it. CI builds commit 7949c0b, the one
the traces were written with. `$CC` is the C compiler, `cc` by default.

How matiec's C is run: `reference_host.c` keeps matiec's located variables
in its own variables, copies the image into them before a tick and the
changed ones back after (matiec has no process image of its own), sets
`__CURRENT_TIME` to the tick times the base interval, and calls
`config_run__`. `reference_fixes.h` covers two gaps between matiec's code
generator and its library, as Prokop does (PROPOSAL.md §1b).

The iecf side builds each case with iecf (the library, as `iecf build`
does), compiles the C as strict C99 with every warning an error
(docs/c-output.md §1), and runs it under `iecf_host.c`: the platform side
of the contract in a page - the image, the context, a scheduler that
runs each task on the ticks its interval divides, by priority. CI runs it
with the sanitizers on. No matiec is needed.

`reference-skip` lists the cases matiec cannot compile (PROPOSAL.md §1b):
the reference side skips them, and each has only a `.iecf.trace`, checked
by hand - `connections`, a program's inputs and outputs connected in the
CONFIGURATION; `sfc`; `parts` and the four `split*` cases, Table 36's
CONCAT and SPLIT functions, which matiec lacks; `fbskip`, a negated
output, which crashes it; `progio`, program connections again; `ldates`,
the L dates, which matiec lacks; `located`, an LTIME at an address,
and `locdata`, an array at an address, which matiec refuses; `refimage`,
`r^ := ...` and REF_TO a function block, which it cannot compile;
`refto`, REFERENCE TO, an extension it does not have; `widen`,
implicit conversions, which matiec (Ed.2) does not make; `fnoutx`,
whose `NOT q =>` crashes it and whose output into an array element it
cannot compile; `fnorder`, VAR_EXTERNAL in a FUNCTION, which it (Ed.2)
refuses; `chars`, CHAR, which it (Ed.2) has not; `implicit`, the
implicit-externals extension; `namespaces`, NAMESPACE and USING (Ed.3),
which it has not, over two files; `globalns`, the global-namespace
extension; `fneffen`, EN on a FUNCTION with effects, with VAR_EXTERNAL
in a FUNCTION again; `inoutcopy`, whose array element given as an
in-out the reference compiler's C cannot compile; `stringchars`, a STRING's character `s[i]` (Ed.3);
`wstrings`, WCHAR and `ws[i]` (Ed.3); `wlocated`, the same and an array at an address.

The generator is built in slices; `iecf-pending` lists the cases it does
not cover yet. Each must fail with E0009 alone, and the run fails when
one of them builds - it comes off the list then, and its trace must
match.
