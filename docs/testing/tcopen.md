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
TcPOU's actions as `ACTION ... END_ACTION` after its body. 1147 files.

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
    --library TcoCore=TcoCore ...            (each library)
    --qualified-gvl TcoEthercat=TcoIo/gvl/TcoEthercat.st ...
    [--gvl GVL=TcoCoreTests/gvl/GVL.st ...]  (the project's own)
    FILE...
```

The `codesys` pack is on, and `recursion`, which is not in it: TcOpen's
methods reach themselves through the type-level call graph (a base's
`Restore` through an override's, `DriveRef`'s getters through
interfaces), and TwinCAT compiles them.

## Where the errors come from

On 2026-11-22, with a pointer indexed, `p[i]`: 915 errors in 226 of the 1147
files. They are counted by their cause, not by code; an error a missing
declaration causes further on counts with it.

**What is not TcOpen's to give: about 440.**

| cause | errors |
|---|---|
| Beckhoff's own libraries: `Tc2_MC2`, `Tc2_System`, `Tc2_Standard`, `Tc2_EtherCAT`, `Tc3_JsonXml` and what they declare (`AXIS_REF`, `MC_*`, `T_MaxString`, `ADSLOG*`, `EC_DEVICE_STATE_*`, `RAD_TO_DEG`) | ~400 |
| of those, `TcoAxisRef EXTENDS Tc2_MC2.AXIS_REF`: each `.Status`, `.NcToPlc`, `.ADS` of an axis | 92 |
| `IBaseLibrary`, a CODESYS library | 40 |

They stay unless the libraries' declarations are given; they are
Beckhoff's and CODESYS's, not iecf's to write.

**What CODESYS and TwinCAT have and iecf does not yet: about 30.**

| what | errors |
|---|---|
| `__NEW` and `__DELETE`, memory allocated while the program runs, in TcoSequencerBase | 21 |
| a PROGRAM's variable read by its name, `NO.Obj` in TcoStruct (once for each block holding one) | 4 |
| a derived block's FB_init adding inputs to its base's, `TcoParentedStructure` (an override's signature, E0031) | 1 |
| a structure member's initial value naming another member, `(Parent := THISSTRUCT)` | 1 |
| `__QUERYPOINTER`, a pointer to the instance an interface references, in the Station001 examples | 2 |

Built on 2026-11-21: `__VARINFO` (var-info), 627 errors; `x :=,` and
`y =>,` (empty-arguments), 70. On 2026-11-22: `BIT` (bit), 76; `SUPER^(x := ...)`, `SUPER^()` in a
method, `SUPER^.x` (super-calls), 65; `THIS^` in an initial value
(codesys-oo), 6; TcoStruct (struct-blocks), 48; `__QUERYINTERFACE` and
`__SYSTEM.IQueryInterface` (query-interface), 53; PUBLIC and FINAL on an
interface's method (codesys-oo), 23; `p[i]` (pointers), 7.

**What iecf refuses on purpose, and what is left to look into: the
rest, about 470.** By code: E0010 98 and E0011 37 (names and types
not found, many of them following from the groups above), E0013 57 (an
integer for an interface; an in-out given a type it is not), E0022 55
(`*` addresses no VAR_CONFIG completes), E0031 54, E0012 45 (a POU and
a type of one name; a method's variable named as its block's), E0005
30, E0028 24, E0006 21, E0015 21, E0014 15, E0027 13 (a method's in-out
handed to a block), and fewer of others. Each is its own question: what
TwinCAT allows, what Ed.3 says.

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
