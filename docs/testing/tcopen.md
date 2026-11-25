# TcOpen through iecf

TcOpen is an open framework for TwinCAT 3, Beckhoff's CODESYS-based
system: 1147 files of Structured Text in 38 projects, written for
TwinCAT's compiler and never for the standard. Checking it with iecf
shows what code written for CODESYS and TwinCAT needs that iecf does not
yet take. TcOpen's code is not in this repository; how it is measured
is below.

## How it is measured

TcOpen's repository holds 38 TwinCAT PLC projects under `src/Tco*`: 17
libraries (TcoCore, TcoIo, ...) and the projects that use them, each
library's tests, examples and readme. Each project's POUs, types and
global variable lists are taken out of TwinCAT's XML files (`.TcPOU`,
`.TcDUT`, `.TcIO`, `.TcGVL`) as `.st` files, one folder per project, a
TcPOU's actions as `ACTION ... END_ACTION` after its body, and the
`END_FUNCTION_BLOCK` (or `END_FUNCTION`, ...) TwinCAT leaves implicit,
named by the declaration's first keyword past its comments, `(* *)` and
`//`. 1147 files.

The projects are checked as TwinCAT compiles them. The libraries are
checked once, together, each a `--library` (its files are its
namespace, as TwinCAT compiles a library); each other project is
checked with all the libraries and its own global variable lists, a
`--gvl` each, `--qualified-gvl` where the list says
`{attribute 'qualified_only'}`, and only its own files' errors are
counted. So each error is counted once, in the project it is in, and
the test projects' lists (several of them named `GVL`) do not meet.

```text
iecf check --format json --extension codesys --extension recursion \
    --extension auto-addresses \
    --dynamic-memory 65536 \
    --library TcoCore=TcoCore ...            (each library)
    --qualified-gvl TcoEthercat=TcoIo/gvl/TcoEthercat.st ...
    [--gvl GVL=TcoCoreTests/gvl/GVL.st ...]  (the project's own)
    FILE...
```

The `codesys` pack is on, and `recursion`, which is not in it: TcOpen's
methods reach themselves through the type-level call graph (a base's
`Restore` through an override's, `DriveRef`'s getters through
interfaces), and TwinCAT compiles them; and `auto-addresses`, not in it
either: TwinCAT places a `*` address itself, and TcOpen's test
projects declare their devices' I/O so, in their global variable lists
(since 2026-11-24). `--dynamic-memory 65536` gives each task a pool
for `__NEW`, which the codesys pack's dynamic-memory needs; TcOpen's
sequencer takes a few bytes of it.

## Where the errors come from

On 2026-11-25, with auto-addresses: 483 errors in 83 of the 1147
files. They are counted by their cause, not by code; an error a missing
declaration causes further on counts with it. An error is a library's
when its message names the library or something it declares; the
counts were taken from the run's JSON on 2026-11-23.

**What is not TcOpen's to give: 463.**

| cause | errors |
|---|---|
| Beckhoff's own libraries: `Tc2_MC2`, `Tc2_System`, `Tc2_Standard`, `Tc2_EtherCAT`, `Tc2_Utilities`, `Tc3_JsonXml` and what they declare (`AXIS_REF`, `MC_*`, `T_MaxString`, `ADSLOG*`, `EC_DEVICE_STATE_*`, `RAD_TO_DEG`, `F_ToASC`, `FB_EcCoeSdoRead`, `AMSADDR`) | 420 |
| of those, `TcoAxisRef EXTENDS Tc2_MC2.AXIS_REF`: each `.Status`, `.NcToPlc`, `.ADS` of an axis | 92 |
| `IBaseLibrary`, a CODESYS library | 40 |
| types TwinCAT generates from the I/O configuration, `SyncUnitTask_AB2F5079`, `InfoData_BD06A15F` | 3 |

They stay unless the libraries' declarations, and the I/O
configuration's, are given; they are Beckhoff's and CODESYS's, not
iecf's to write.

**What CODESYS and TwinCAT have and iecf does not yet:** none of the
groups above is left.

Built on 2026-11-21: `__VARINFO` (var-info), 627 errors; `x :=,` and
`y =>,` (empty-arguments), 70. On 2026-11-22: `BIT` (bit), 76; `SUPER^(x := ...)`, `SUPER^()` in a
method, `SUPER^.x` (super-calls), 65; `THIS^` in an initial value
(codesys-oo), 6; TcoStruct (struct-blocks), 48; `__QUERYINTERFACE` and
`__SYSTEM.IQueryInterface` (query-interface), 53; PUBLIC and FINAL on an
interface's method (codesys-oo), 23; `p[i]` (pointers), 7; `__NEW` and `__DELETE`
(dynamic-memory, `--dynamic-memory=65536`), 25. On 2026-11-23: a program
read, called and given methods by its name (program-instances), 21;
`__QUERYPOINTER` and a `POINTER TO` a block (query-pointer), 3; a
derived FB_init adding or renaming inputs (fb-init), 8; a structure member
given another member, `(Parent := THISSTRUCT)` (struct-blocks), 1;
arrays of instances given FB_init arguments, `ARRAY[0..2] OF
DiagnosticsMessenger(THIS^)` (fb-init), 5; and 7 in-outs of a type with an
FB_init, which iecf took for instances to set up (E0009), fixed; each
library's own `MAIN`, which clashed with every other's while a library's
PROGRAMs stayed outside its namespace (E0012, 31, and 1 E0028), fixed;
the cylinders' PROPERTY `StopTask`, whose `StopTask.Restore()` was taken
for TcoDrivesBeckhoff's FUNCTION_BLOCK StopTask (E0028, 8), fixed;
another block's internal instance called, `_inspectorTask._failTimer(IN
:= TRUE, PT := ...)`, `_to_A._sut_A()` (internal-reads), 40; TcoTixonFeeding's
steps giving their own Config, Control and Status, each extending a
generic one, to their tasks' generic in-outs (struct-extends), 30; an
interface initialised or set to 0, `_drive : ITcoDriveSimple := 0`
(codesys-oo), 19; a property read as a statement for what its GET does,
`_sutNotInitialized.IsTrue;`, `_sut_A.Invoke().Done;`
(expression-statements, warned), 16; TcoInsight's `METHOD
SetUserDataAsByte : BOOL;`, its declarations taken for the block's
(header-semicolon), 16; a method's variable named as its block's,
`_state` beside `_State` (shadowing), 5, and 24 that followed from
them; a property with no SET whose GET gives a `REFERENCE TO`, assigned
whole, `_tcoInspectorDialogue.HideOverride := TRUE` (properties), 21;
a block's in-out of an I/O structure with `*` components, `inoData :
TcoIrc5_IO_v_1_x_x`, 22; the test projects' `*` addresses no VAR_CONFIG
gives, in their global variable lists and a MAIN (auto-addresses,
warned), 34.

**What iecf refuses on purpose, and what is left to look into: the
rest, 20.** By code: E0005 1 (TcoTaskResult's `AsBool := ` with nothing after it, TcOpen's own); E0027 1 (TcoSignalInfo keeping
an input's pointer); E0022 4 (TcoEthercatMasterDevice's inputs, output and
method variable of generated I/O types with `*` components); E0010 4
and E0011 2 (`Rtc` on ITcoContext, a
block read as a variable, `END_GET` read as a name; the types `ICylinder`
and `ANY`); E0026 3; E0032
3; E0016 1 (`inEnableAxis3` passed twice); E0012 1 (an interface named
twice). Each is its own question: what TwinCAT allows, what Ed.3
says.

## History

| date | errors | files | what changed |
|---|---|---|---|
| 2026-11-20 | 2405 | | the extractor fixed: 41 files had lost END_FUNCTION_BLOCK |
| 2026-11-21 | 2339 | | a TcPOU's actions extracted |
| 2026-11-21 | 2320 | 339 | ref-this, any-to |
| 2026-11-21 | 1693 | 286 | var-info |
| 2026-11-21 | 1258 | 277 | measured project by project, each with its own global variable lists, and with recursion |
| 2026-11-21 | 1188 | 277 | empty-arguments |
| 2026-11-22 | 1112 | 272 | bit |
| 2026-11-22 | 1048 | 260 | super-calls (65 gone; 1 new, TcOpen's own: `TcoMultiAxisTest` passes `inEnableAxis3` twice, the second meant for axis 4) |
| 2026-11-22 | 1042 | 260 | `THIS^` in an initial value |
| 2026-11-22 | 998 | 241 | struct-blocks (48 gone; 4 new, `NO.Obj`, a PROGRAM's variable, met once TcoStruct parses) |
| 2026-11-22 | 945 | 233 | query-interface (53 gone, none new) |
| 2026-11-22 | 922 | 226 | PUBLIC and FINAL on an interface's method (23 gone, none new) |
| 2026-11-22 | 915 | 226 | `p[i]`, a pointer indexed (7 gone, none new) |
| 2026-11-22 | 890 | 224 | dynamic-memory (25 gone, none new: TcoSequencerBase's 21, `__DELETE` in two of TcoCoreTests' blocks) |
| 2026-11-23 | 879 | 221 | program-instances (21 gone: `NO.Obj`, `RtcPrg`, `NF_MAIN`, `MAIN_PRG`, `EXAMPLES_PRG`, MAIN's calls; 10 new, `Tc2_EtherCAT`, Beckhoff's, in `RtcPrg`'s method, now read) |
| 2026-11-23 | 876 | 220 | query-pointer (3 gone; `_Identity := ADR(THIS^)`, 5, now E0013 instead of E0027: an address is no number) |
| 2026-11-23 | 868 | 218 | a derived FB_init adding inputs (5) or naming its base's its own way (3): 8 gone, none new |
| 2026-11-23 | 867 | 217 | a structure member's initial value giving an interface another member (1 gone, none new) |
| 2026-11-23 | 855 | 214 | arrays of instances set up by their FB_init (5 E0016 gone), in-outs no longer taken for instances to set up (7 E0009 gone); none new |
| 2026-11-23 | 828 | 186 | a library's PROGRAMs in its namespace (32 gone: each library's `MAIN`; 5 new, E0023, `AT %Q*` in TcoCognexVisionTests' MAIN, read now) |
| 2026-11-23 | 820 | 183 | a block's members, its bases' too, hide a library's names (8 gone: `StopTask.Restore()`, `.Invoke()`; none new) |
| 2026-11-23 | 780 | 179 | another block's internal instance called, warned (40 gone, 45 W0012 more; none new) |
| 2026-11-23 | 750 | 176 | a structure given to an in-out of a structure it extends (30 gone, none new) |
| 2026-11-23 | 731 | 175 | 0 given to an interface, its NULL (19 gone, none new) |
| 2026-11-23 | 715 | 169 | an expression standing as a statement (16 E0005 gone, 16 W0014 and 2 W0012 more, `_sut_A` read now; none new) |
| 2026-11-23 | 672 | 166 | a header's `;` (16 gone), a method's variable hiding its block's (29 gone); 2 new in TcoInsight's code read now (`F_ToASC`, Beckhoff's; a DINT for a REAL), 50 W0013 and 1 W0010 more |
| 2026-11-23 | 651 | 163 | a property with no SET whose GET gives a REFERENCE TO, assigned whole (21 E0015 gone, none new) |
| 2026-11-23 | 633 | 148 | the extractor fixed: 18 files whose declaration opens with a `//` comment (CheckBounds and TcoCore's other implicit checks, TcoTaskResult, 7 example and test blocks) had lost their END_ (18 E0006 gone, none new) |
| 2026-11-23 | 611 | 141 | an in-out of a type with `*` components, the caller's bound (22 E0022 gone, none new) |
| 2026-11-24 | 577 | 126 | measured with auto-addresses: the test projects' `*` addresses placed (29 E0022 and 5 E0023 gone, 30 W0015; none new) |
| 2026-11-24 | 549 | 118 | a block with methods in a method, copied and an input of its own type (28 E0031 gone, none new) |
| 2026-11-24 | 538 | 111 | an ABSTRACT block with no abstract method (11 E0031 gone, none new) |
| 2026-11-24 | 534 | 109 | a derived block adding an in-out standing for its base (4 E0031 gone, none new) |
| 2026-11-24 | 531 | 108 | an empty VAR in an interface's property accessor (3 E0031 gone, none new) |
| 2026-11-24 | 530 | 107 | var-info's `__SYSTEM.VAR_INFO` declared after the libraries' USINGs (1 E0005 gone, none new) |
| 2026-11-24 | 522 | 99 | a `;` that ends nothing, `END_TYPE;` and `;;` (8 E0005 gone, none new) |
| 2026-11-24 | 520 | 98 | a last `,` before `)` in a call (1 E0005 and the E0016 it caused gone, none new) |
| 2026-11-25 | 516 | 97 | a literal too wide for the other operand widening the operation, `256 * _b0` (4 E0020 gone, none new) |
| 2026-11-25 | 513 | 95 | implicit-narrowing into a real: LREAL to REAL, DINT to REAL (3 E0013 gone, now W0010; none new) |
| 2026-11-25 | 505 | 93 | implicit-narrowing comparing a ULINT and a signed integer by value (8 E0014 gone, now W0010; none new) |
| 2026-11-25 | 503 | 92 | implicit-narrowing keeping the low bits of a literal outside its type, `-1` into a UINT (2 E0020 gone, now W0010; none new) |
| 2026-11-25 | 501 | 91 | a DT literal without its seconds, or with a fraction of one (dt-literals; 2 E0020 gone, the fraction's now W0016; none new) |
| 2026-11-25 | 496 | 87 | a pointer to a ULINT, a number naming its place: `_Identity := ADR(THIS^)` (5 E0013 gone, now W0017; none new) |
| 2026-11-25 | 489 | 84 | a type or method named with a leading `_`, `_internals_TcoContext`, found in its namespace (7 E0030 gone, an iecf bug; none new) |
| 2026-11-25 | 483 | 83 | a method's in-out kept past its call by a block it gives it to, each call of the method checked instead, `RunWithManualRestore(Value)` (method-in-outs; 6 E0027 gone, none new) |
