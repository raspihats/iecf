# Implementer extensions

IEC 61131-3 Ed.3 clause 5 c asks an implementation for "a document that
specifies all Implementer specific extensions. These are any features
accepted by the system that are prohibited or not specified." This is
that document for iecf.

By default iecf accepts none: it checks the standard as written. Each
extension is switched on by name, for one run:

```
iecf check --extension implicit-externals FILE...
```

A program that needs an extension is not portable to another compliant
tool as it stands; the section of each extension says what to change to
make it so.

## implicit-externals: a POU uses a global variable without declaring it VAR_EXTERNAL

**The standard.** Ed.3 6.5.2.2: global variables are "only accessible
to a program organization unit via a VAR_EXTERNAL declaration", with
the global's type, and `CONSTANT` if the global is. Without the
extension, a global used undeclared is E0010.

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

## time-integer-conversions: TIME and LTIME to and from integers, in ms and ns

**The standard.** Ed.3 Figure 11 defines no conversion between TIME or
LTIME and an integer or bit-string type ("-"), and Ed.2 named none.
Without the extension, `DINT_TO_TIME`, `TIME_TO_UDINT`, `TO_TIME(i)`
and the like are E0013. A duration is made from a number with TIME
arithmetic: `T#1ms * n`.

**The extension.** `<int>_TO_TIME`, `TIME_TO_<int>`, the same for LTIME,
the overloaded `TO_TIME`/`TO_LTIME` from an integer, and the same with
BYTE, WORD, DWORD and LWORD. A TIME converts as its count of
milliseconds, an LTIME as its count of nanoseconds: `DINT_TO_TIME(1500)`
is `T#1s500ms`, `TIME_TO_DINT(T#2s)` is 2000. A value that does not fit
the target keeps its low bits and is flagged, as any integer conversion
(docs/c-output.md §5).

**Why.** CODESYS, Beckhoff TwinCAT and Siemens all have these
conversions, counting milliseconds; programs written for them use them.
(matiec has them too, counting seconds: a program carried over from
matiec waits a thousand times less here.)

**To make a program portable.** Replace `DINT_TO_TIME(n)` with
`T#1ms * n` (Table 35, MUL). The standard has no way back from a
duration to a number (Table 35 divides a TIME only by a number), so keep
a duration in a TIME throughout and compare it with TIME literals.

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
