# TcOpen through iecf

TcOpen is an open framework for TwinCAT 3, Beckhoff's CODESYS-based
system: 1104 files of Structured Text in 18 libraries, written for
TwinCAT's compiler and never for the standard. Checking it with iecf
shows what code written for CODESYS and TwinCAT needs that iecf does not
yet take. TcOpen's code is not in this repository; how it is measured
is below.

## How it is measured

Each library's POUs, types and global variable lists are taken out of
TwinCAT's XML files (`.TcPOU`, `.TcDUT`, `.TcGVL`, `.TcIO`) as `.st`
files, a TcPOU's actions as `ACTION ... END_ACTION` after its body. All
of them are checked as one project:

```text
iecf check --format json --extension codesys \
    --library TcoCore=TcoCore --library TcoIo=TcoIo ...   (one per library)
    --qualified-gvl TcoVisionParams=TcoCognexVision/gvl/TcoVisionParams.st \
    --qualified-gvl TcoEthercat=TcoIo/gvl/TcoEthercat.st \
    FILE...
```

Each library is a `--library`: its files are its namespace, as TwinCAT
compiles a library. Only the `codesys` pack is on (see
[Extensions](../extensions.md)); `recursion`, which is not in it, is
not.

## Where the errors come from

On 2026-11-21, with var-info: 1693 errors in 286 of the 1104 files
(2320 in 339 before it). They are counted by their cause, not by code;
an error a missing declaration causes further on counts with it.

**What the measurement does not give iecf: about 815.**

| cause | errors |
|---|---|
| Beckhoff's own libraries, not part of TcOpen: `Tc2_MC2`, `Tc2_System`, `Tc2_Standard`, `Tc2_EtherCAT`, `Tc3_JsonXml` and what they declare (`AXIS_REF`, `MC_*`, `T_MaxString`, `ADSLOG*`, `EC_DEVICE_STATE_*`, `RAD_TO_DEG`) | ~400 |
| of those, `TcoAxisRef EXTENDS Tc2_MC2.AXIS_REF`: each `.Status`, `.NcToPlc`, `.ADS` of an axis | 92 |
| the global variable lists of TcOpen's test and example projects, not passed (`GVL_IxLinker` 117, `GVL` 46, `ST_01_IO` 36, and `IO.A1._1`: "INT has no members" 40) | ~240 |
| the recursion extension, not in the codesys pack: TcOpen's methods reach themselves through the type-level call graph (a base's `Restore` through an override's, `DriveRef`'s getters through interfaces) | 138 |
| `IBaseLibrary`, a CODESYS library | 40 |

With `--extension recursion` added, 138 fewer. The Beckhoff libraries'
errors stay unless their declarations are given; they are Beckhoff's,
and not iecf's to write.

**What CODESYS and TwinCAT have and iecf does not yet: about 310.**

| what | errors |
|---|---|
| `BIT`, a structure's member one bit wide | 75 |
| `SUPER^` as iecf does not take it: `SUPER()` with arguments, in a method, other forms | ~70 |
| `__QUERYINTERFACE`, `__NEW`, `__DELETE`, `__QUERYPOINTER` | 70 |
| an output left unconnected, `Done =>,` | 62 |
| an access specifier on an INTERFACE's method | 23 |
| `THIS^` and `THISSTRUCT` in an initial value, `(Parent := THIS^)` | ~10 |
| `__VARINFO`: 627 until var-info, built 2026-11-21 | 0 |

**What iecf refuses on purpose, and what is left to look into: the
rest, about 570.** By code: E0030 106 (INTERNAL methods and properties
called where iecf finds them out of reach; TwinCAT's INTERNAL reaches
the library, and whether iecf's reach is that is to look into), E0013 57 (an integer for an
interface; an in-out given a type it is not), E0012 30 (a POU and a
type of one name; a method's variable named as its block's), E0022 27
(`*` addresses no VAR_CONFIG completes), E0028 23, E0006 21, E0015 21,
E0014 15, E0027 13 (a method's in-out handed to a block), and fewer of
others. Each is its own question: what TwinCAT allows, what Ed.3 says.

## History

| date | errors | files | what changed |
|---|---|---|---|
| 2026-11-20 | 2405 | | the extractor fixed: 41 files had lost END_FUNCTION_BLOCK |
| 2026-11-21 | 2339 | | a TcPOU's actions extracted |
| 2026-11-21 | 2320 | 339 | ref-this, any-to |
| 2026-11-21 | 1693 | 286 | var-info |
