# iecf - the IEC Forge, an IEC 61131-3 compiler for Prokop

PROPOSED 2026-11-05. Decided the same day: the name, the repository, the
edition, the implementation language, the semantics and RuSTy first
(§6); open: the IEC name policy (§6.5). florin asked
whether Prokop could have a matiec-like compiler of its own, what
matiec's limitations have been during Prokop's development, and said it
must be IEC 61131-3 compliant. This file answers with the limitations as
the repository recorded them (§1), what the compiler would be (§2-3),
what it costs (§4), an order (§5), and florin's decisions (§6).

**Working name: `iecf`, the IEC Forge** (florin, 2026-11-05), in its own
repository (§6.6).
Free on crates.io, npm and PyPI; the GitHub account `iecf` is a dormant
personal one, so the repository would be `raspihats/iecf`. IECF is also
the Independent Electrical Contractors Foundation, a US trade charity.
Every name starting with "iec" carries one open question: IEC is the
International Electrotechnical Commission's name, and its policy on use
of the name has not been read (§6.5).

Paths such as `spec/abi.md`, `toolchain/MATIEC.md` and `README.md` are in
the Prokop repository (`raspihats/prokop`), not this one.

Statements about the standard's text that come from memory, not from
the text or a measurement in this repository, are marked **(check)**.
None of them may become a compliance claim until checked against the
purchased standard. Ed.2 (2003) and Ed.3 (2013) are in `docs/`, kept
locally and git-ignored (copyright); clause numbers below are theirs.
Ed.4 is not bought yet.

## 1. matiec's limitations, as Prokop met them

Sources: toolchain/MATIEC.md §7, spec/standard-functions.md,
spec/abi.md §6, spec/io-tags.md §7, spec/graphical-languages.md §1,
spec/debugging.md, and the Workbench's generators and pre-checks. matiec
is beremiz/matiec, measured on 7949c0b.

### 1a. IEC 61131-3 it does not implement

matiec implements the standard's 2nd edition (2003).

| Missing | As met |
| --- | --- |
| LTIME, LDATE, LTOD, LDT | Ed.3 types; absent (standard-functions §2a, abi §6) |
| `STRING[n]` | Parsed, never finished: the type checker gives it no type and the C generator dies - an internal error as a global, a segfault as a local (io-tags §7). Every STRING is 126 characters (`STR_MAX_LEN`) |
| Characters above 127 | The lexer refuses `'héllo'` in UTF-8 and in Latin-1; programs write `'$E9'`. There is no encoding (abi §6) |
| Ed.3 object orientation | METHOD, INTERFACE, CLASS, EXTENDS, namespaces - none; all are Ed.3 (Ed.3 Annex B: "Class, including method, interface", "Namespaces"), absent from Ed.2. No namespaces is why tags and globals.gvl share one flat namespace (io-tags §8) |
| Implicit widening conversions | `INT + DINT` and `r := 42` into a REAL are refused, so Blockly writes `42.0` and refuses mixed types (lblk-generate.ts). Right for Ed.2, which wants the conversion functions (2.5.1.4, figures 7-8). Ed.3 6.6.1.6 adds implicit conversion that "shall keep the value and accuracy" (Figures 11-12; Example 6 divides an INT by a REAL) |
| LD and FBD | No textual front end: both exist only inside a comment in `iec_bison.yy`, since 2003 (graphical-languages §1). Prokop generates ST from both, so neither has its own diagnostics or debug mapping |
| An unnamed `ARRAY[..] OF` across `VAR_EXTERNAL` | "Data type mismatch": each side is a different anonymous type (io-tags §7) |

### 1b. Bugs

- **A duplicate POU** is "Internal compiler error in file
  ../util/symtable.cc at line 120", no file, no line. The Workbench
  refuses it first (`build-files.ts`, `duplicatePouFiles`).
- **TIME arithmetic did not build**: the ST printer writes
  `_____time_add`, the library defines `__time_add`.
  `matiec_overrides.h` maps the four names.
- **Error lines and words that mislead**: a located and a plain variable
  in one block is reported on the plain line, with words that change with
  the order; a variable named `dt` draws the same words.
  `matiec_hints.py` exists to explain them.
- **`iec_lib_*` left undefined**: the generated code calls math, printf
  and `memcmp` wrappers matiec does not define. Each failed at link until
  musl was vendored (`toolchain/libm/`, `toolchain/printf/`).
- **`REAL_TO_STRING`** prints `%.10g`, `'0.1000000015'` for 0.1;
  overridden to 7 and 15 significant digits.

### 1c. Behaviour that differs by how a thing is written

Both editions call division by zero and a result out of its type's
range errors (Ed.2 Annex E, Table E.1; Ed.3 7.3.2 and the numerical
functions), and leave what happens on an error to the implementer, who
must document it (Ed.2 1.5.1; Ed.3 5 e). matiec documents nothing and is
inconsistent (standard-functions §2b):

- **Integer division by zero**: ST `/` and IL `DIV` trap (FAULT); ST
  `MOD` gives 0; the `DIV()`/`MOD()` functions - and so FBD and LD - give
  0 with ENO FALSE; IL `MOD` traps.
- **Overflow**: `INT` and `SINT` wrap through C's promotion to `int`;
  `DINT` and `LINT` trap on the smallest value divided by -1. Signed
  overflow in the generated C is undefined behaviour, which clang may
  optimise on.
- **`DINT_TO_TIME(1500)` is 1500 seconds**; CODESYS and Siemens count
  milliseconds. Neither edition defines an integer-to-TIME conversion:
  each is an implementer extension.
- **An infinite REAL** converts through LINT to a quietly wrong integer.
- **Array indexes are not checked**: an index out of range writes a
  neighbour, inside the wasm sandbox.

### 1d. What it does not give a platform

- **No symbol table.** `VARIABLES.csv` is paired with
  `LOCATED_VARIABLES.h` by position, checked by count
  (`matiec_symbols.py`). It lists no array, structure, enumeration or
  function block internals, so `matiec_aux.py` and `matiec_retain.py`
  rebuild them from the program text.
- **RETAIN is a flag.** `__INIT_VAR` resets the value on every init; the
  RETAIN of an array or structure is dropped silently. `matiec_retain.py`
  keeps retain field by field (abi §6).
- **Contracts that are only observed**: the instance global's name
  (`RES0__INST`) and `POUS.c` included twice, both load-bearing.
- **One file, one pass**: the CONFIGURATION last; `{#include}` only at
  the top level; every identifier uppercased (Python bindings,
  toolchain/PYTHON.md); one PROGRAM per RESOURCE at least, so a
  Python-only project gets a hidden idle PROGRAM.
- **No statement-level line map.** IEC languages step a scan at a time;
  Python tasks have breakpoints (debugging §3).
- **Errors only at Build.** The Workbench re-implements matiec's rules in
  TypeScript to say them earlier: located-vars.ts, empty blocks,
  reserved words, type mixing, duplicate POUs.
- **GPLv3**, so it stays an external fork, invoked, its output not
  captured (README.md, ARCHITECTURE.md §11).

### 1e. Not matiec's fault

A compliant compiler keeps these and only says them better:

- An empty `VAR ... END_VAR` - Ed.2's grammar wants at least one
  declaration (B.1.4.3, `var_declarations`). **Ed.3 allows it**: its
  grammar repeats `( Var_Decl_Init ';' )*` (Annex A). So iecf, on Ed.3 or
  later, accepts it.
- `DT`, `TOD` as names - Ed.2 Table C.2 lists "Data type names" and
  "Function names" as keywords, which "shall not be used for any other
  purpose, for example, variable names" (2.1.3), so `LT`, `ADD`, `MOD`
  are refused too. Ed.3 6.1.3 keeps the rule.
- Located and plain declarations in one block - separate productions in
  both grammars: Ed.2 `located_var_declarations` and `var_declarations`
  (B.1.4.3), Ed.3 `Loc_Var_Decls` and `Var_Decls` (Annex A). CODESYS
  relaxes it.

## 2. What iecf is

- **Input**: ST, SFC's textual form, the declarations, CONFIGURATION /
  RESOURCE / TASK - written against the normative text of IEC 61131-3,
  Ed.4 (2025) (§3a, §6.1). No IL, which Ed.4 removed.
- **Output: portable C**, as generic as C can be (florin, 2026-11-05).
  ISO C99, no compiler extensions or built-ins, no target assumptions:
  `<stdint.h>` fixed-width types, no word size, endianness or libc
  taken for granted. What a platform must supply - the fault hook, the
  time base, the process image access - is a small header of functions
  and macros the platform implements, nothing more. Any C99 compiler
  for any target builds the output; a program that compiles but behaves
  differently on two targets is an iecf bug. Prokop is one consumer: it
  compiles the C with zig/clang to wasm32 as today, and the prokop-1
  ABI, the process image, the retain and aux regions, `matiec_glue.c`'s
  role, the musl vendoring and the MicroPython link stay on Prokop's
  side of that header.
- **A symbol table and layout it emits itself**, with arrays,
  structures, enumerations and function block members, replacing
  `VARIABLES.csv`, the positional pairing and most of the aux and retain
  bridges ("the compiler-emitted symbol table replaces it", abi §6).
- **Semantics it defines**, not C's (§6.3): an index out of range, a
  division by zero and an integer overflow FAULT, the same in every
  language, checked by plain C comparisons before each operation (no
  `__builtin_*_overflow`, which is GCC/clang only), so the generated C
  has no undefined behaviour on any compiler; integer-to-TIME counts
  milliseconds. Each is in the
  compliance statement and its error document (Ed.3 clause 5 d, e).
- **Its own front end in the browser** (§2a, §6.7): the compiler itself -
  parser and checker, not its output - built to wasm so it runs in the
  editor and gives errors on the right file and line as the user types.
  This is about where iecf runs, not what it emits.
- **Standard text as its input** (§2b): a set of IEC 61131-3 source
  files, never text a caller has edited first, and no project format of
  its own; IEC 61131-10 XML as a later import.
- **A statement-to-C line map**, so ST breakpoints are the tier-2 work
  Python already had (debugging §2b).
- **Apache-2.0 OR MIT**, like the rest of the repository; the GPLv3
  boundary goes away with matiec.

### 2a. Where the checking runs: in the browser

Decided 2026-11-05 (§6.7), on a general ground rather than on how
Prokop is built today: a hosted service with thousands of users.

| | the server checks as one types | the browser checks (wasm) |
| --- | --- | --- |
| Server load | a request per user per pause in typing - thousands a minute, scaled and paid for | none: each user's machine checks its own code |
| Delay | a network round trip, 50-300 ms, plus queueing | microseconds per POU: errors as one types |
| Server down or slow | no errors shown | still works, offline too |
| The server still does | everything | builds, which give the authoritative verdict |

Typing is what users do most; checking it in the browser costs the
service nothing. Measured 2026-11-05: the parser and checker of
`dbfd009`+`7c1d847` as a wasm module are 189 KB, 71 KB gzipped - a
complete compiler several times that is still smaller than the code
editor a web IDE loads anyway.

What it asks of iecf, and why each is good design regardless:

1. **A core without I/O.** `iecf-syntax`, `iecf-sema` and what follows
   take text and return trees and diagnostics: no files, no threads, no
   network, no native libraries. CI builds the core for
   `wasm32-unknown-unknown` on every push, so the rule cannot erode. The
   same library serves the browser, a Language Server for desktop
   editors, CI, and the build.
2. **One version everywhere.** The browser's wasm and the build's binary
   come from one commit, released together; a web IDE loads the wasm of
   the iecf its server builds with. Otherwise the editor says fine and
   the build says error.
3. **The same input as the build** - §2b.
4. **Diagnostics as JSON**, as `iecf check --format json` gives them;
   the browser entry point is a thin wrapper around that.

The build stays the authority: the browser's check is advice given
early, by the same code.

### 2b. The input is IEC 61131-3 itself

Decided 2026-11-05 (florin: generic, not tied to Prokop, or a
standard). A compiler must not need its input text edited first, and
iecf needs no project format of its own: the standard already says, in
its own text, everything a project description would.

| What a platform must tell the compiler | The standard's own way | Ed.3 |
| --- | --- | --- |
| which programs run, on which tasks, how often | `CONFIGURATION`, `RESOURCE`, `TASK`, program instances | 6.8, Annex A |
| the global variables | `VAR_GLOBAL` in the configuration or a resource | 6.5.2 |
| the I/O | a variable at an address: `start AT %IX0.0 : BOOL;` | 6.5.5, Table 16 |
| a POU reused across machines, wired per instance | `AT %I*` in the POU, the full address in the configuration's `VAR_CONFIG` | 6.5.5.4 |
| a POU's use of a global | `VAR_EXTERNAL` | 6.5.2.2 |

So:

1. **iecf's input is a set of IEC 61131-3 source files**, in any order;
   a name declared in one is seen in the others (as `iecf check` does
   already). There is no manifest.
2. **The configuration is ordinary IEC 61131-3 source.** iecf needs to
   know which programs run on which tasks, and the names, types and
   addresses of the I/O; the standard's `CONFIGURATION` says exactly
   that, and is part of the program like any POU. Who writes it is not
   iecf's business: the user writes it by hand, or a tool generates it
   from its own interface (a task editor, an I/O table). A tool that
   generates it puts it in a file of its own rather than splicing it into
   the user's text: the user's files stay as written, diagnostics point
   at lines the user recognises, the generated file can be regenerated
   freely, and the browser and the build (§2a) read the same files. Any
   compiler that follows the standard reads them too.
3. **The hardware stays outside the compiler.** IEC 61131-3 stops at
   the direct address: `%IX0.0` is all a compiler sees. Which card, bus
   or pin it is belongs to the runtime, as do the fieldbus device
   descriptions (PROFINET GSDML, EtherCAT ESI, IO-Link IODD). Prokop's
   `io_map.json` is one such mapping, and stays in Prokop.
4. **IEC 61131-10 (2019), the PLCopen XML exchange format**, is the
   standard project file: POUs, data types, configurations, resources,
   tasks and global variables, its XML schema in Annex A. iecf imports
   it, later, so projects move between iecf and other tools. Text stays
   the main input: what people write and what version control diffs.
   Part 10 must be bought too before it is relied on.
5. **`VAR_EXTERNAL` is required**, as Ed.3 6.5.2.2 says: global
   variables are "only accessible to a program organization unit via a
   VAR_EXTERNAL declaration". Using a global without one is an
   implementer extension: off by default, switched on by an explicit
   option (`--extension implicit-externals`), listed in the document of
   extensions (`docs/extensions.md`, Ed.3 clause 5 c). Whether
   a platform switches it on, or has its editor write the declarations
   where the user sees them, is the platform's choice, not iecf's.
6. **The implementation's own limits** - default STRING length, the
   ranges of the time types, identifier length - are fixed by iecf and
   documented in the compliance statement (Ed.3 clause 5 d), not set
   per project. A setting exists only where the standard leaves a choice
   to the user's target.

`AT %I*` with `VAR_CONFIG` is read and checked (E0022-E0024): a `*`
address stands only where a path can reach it (`VAR` or `VAR_OUTPUT` of
a PROGRAM or FUNCTION_BLOCK, or a structure field), and every `*` of
every program instance gets its address, in the same area. The
`VAR_EXTERNAL` extension is in, off by default. `VAR_ACCESS` is still
reported as not yet compiled (E0009).

**Written in Rust** (§6.2): one native binary for the build
service - the same pattern as `iec2c` today - and the same front end as
wasm for the Workbench.

**Why C out and not Rust out** (florin asked, 2026-11-05): IEC's memory
model is shared mutable state - globals, `VAR_EXTERNAL`, `VAR_IN_OUT` by
reference, `REF_TO`, function block instances reached from several
places - which is `static mut` and raw pointers in Rust, so the generated
code would be mostly `unsafe`. rustc on large generated code is several
times slower than clang; the module links C anyway (glue, musl,
MicroPython); IL's `JMP` has no `goto` in Rust. The wasm sandbox already
contains a fault, and the checks Rust would add, iecf can emit in C. If
C is ever left, the step is LLVM IR or wasm directly, not Rust.

## 3. Compliance

IEC 61131-3 asks an implementation to state which features it supports
and its implementation-dependent parameters. Ed.3 clause 5 asks a tool
claiming compliance to provide: a compliance statement (5.3: "any
consistent subset of the features listed in the feature tables", each by
table and feature number and language, Figure 4 an example); documents
of its extensions, of its implementation dependencies and limits, and of
every error it detects and how. Ed.2 had the same in 1.5.1, its
parameters in Annex D and its errors in Annex E. Ed.4 keeps 5.3 and
Figure 4 (IEC preview, contents). iecf ships the statement and those
documents, generated from its own tables:

- the feature tables supported, by number;
- the parameters: STRING length, TIME range and resolution, division by
  zero, overflow, index out of range, identifier length;
- every deliberate difference from matiec, while matiec is still the
  comparison (§4).

The outside checks: PLCopen's conformance material and the TC6 XML
exchange format (check what PLCopen offers today). There is no formal
certification of a compiler to rely on.

The standard itself must be bought - Ed.4 (2025), and Ed.3 (2013) to
read Annex B against - and is the source for every rule. Nothing is taken
from CODESYS's documentation as if it were the standard.

### 3a. The editions

An edition is a version of the standard; each cancels and replaces the
one before.

| Edition | Year | In short |
| --- | --- | --- |
| Ed.1 | 1993 | The five languages: IL, ST, LD, FBD, SFC |
| Ed.2 | 2003 | Revision. matiec implements this one |
| Ed.3 | 2013 | "a compatible extension of the second edition": object orientation (classes, OO function blocks, methods, interfaces), namespaces, references (`REF_TO`), new types (LTIME, LDATE, LTOD, LDT), variable-length arrays, implicit conversions, `CONTINUE`. Deprecated: octal literals, overloaded `TRUNC`, **IL**, directly represented variables in POU bodies, the action block "indicator" variable (Ed.3 Annex B) |
| Ed.4 | 2025-05 | "This fourth edition cancels and replaces the third edition published in 2013" (IEC preview). UTF-8 strings; Annex B lists every addition, deletion and deprecation against Ed.3 |

What Ed.4 changes against Ed.3. Verified from the IEC preview (foreword
and table of contents) where marked *preview*; the rest from Stefan
Henneken's comparison (2025-06-11, read from the standard but not
tested on any tool), a second-hand source until the text is bought:

- **Added**:
  - UTF-8 strings: `USTRING`, `UCHAR`, literals `U#'...'`, their
    conversions, `LEN_MAX`, `LEN_CODE_UNIT` (*preview*: "inclusion of
    UTF-8 strings and their associated functions").
  - Character codes in literals: `'${1F579}'`.
  - Synchronization of concurrent execution, §6.9 (*preview*): `MUTEX`
    and `SEMA` function blocks with methods, and the functions
    `MUTEX_LOCK`/`_UNLOCK`/`_TRYLOCK`, `SEMA_ACQUIRE`/`_RELEASE`/
    `_TRY_ACQUIRE`.
  - Properties: `PROPERTY_GET`/`PROPERTY_SET` on CLASS, FUNCTION_BLOCK
    and INTERFACE.
  - `ASSERT(IN)`.
  - STRING/WSTRING/USTRING to and from `ARRAY OF BYTE`; some new
    `*_TO_BOOL` and `*_TO_CHAR` conversions.
- **Removed**:
  - Instruction List, deprecated since Ed.3 (Annex B). Ed.4 clause 7
    has only 7.2 Structured text (*preview*); "no longer included in
    Edition 4" (Henneken).
  - Octal literals (`8#267`).
  - `TRUNC(x)` with no type in the name; `TRUNC_DINT(x)` and
    `LREAL_TRUNC_DINT(x)` stay.
- **Deprecated**: the BCD functions (`IS_VALID_BCD`, `*_BCD_TO_*`,
  `*_TO_BCD_*`).
- **Changed**: STRING to WSTRING and CHAR to WCHAR are implicit;
  `ARRAY[1..1]` and `ARRAY[1..0]` defined.
- **Still not in Ed.4** (Henneken): optional parameters, method
  overloading, implicit enumerations.

Who implements Ed.4, as searched 2026-11-05:

| Vendor | Ed.4 | Source |
| --- | --- | --- |
| Beckhoff TwinCAT PLC++ | "almost fully compliant with the fourth edition of IEC 61131-3"; "we have dispensed with Instruction List" (Hans Beckhoff, 2024-10-30). No public feature list found | beckhoff.com product page and news |
| Beckhoff TwinCAT 3 PLC | No Ed.4 claim; documentation lists all five languages, IL included | infosys.beckhoff.com |
| CODESYS | No Ed.4 claim found; "OOP according to 61131-3 3rd Edition". CODESYS 4 page not read (HTTP 429) | codesys.com |
| Siemens TIA Portal | No statement found | - |
| PLCopen | "The current version is 4.0 from 2025"; no Ed.4 conformance of any tool | plcopen.org |

So the market is Ed.3 plus vendor extensions, and the programs users
bring will be written that way. Ed.4 keeps all of Ed.3 but IL, octal
literals and untyped `TRUNC`, so targeting Ed.4 covers that code too.

Sources: the IEC 61131-3:2025 preview (foreword and contents,
webstore.iec.ch); stefanhenneken.net/2025/06/11/iec-61131-3-comparison-of-edition-3-and-edition-4/;
beckhoff.com/en-us/products/automation/twincat-plc/;
beckhoff.com/en-us/company/news/with-energy-and-optimism-into-the-future.html;
us.codesys.com/products/engineering/development-system/;
plcopen.org/standards/logic/.

## 4. Cost and risk

- **The bulk**: the type checker, overload resolution of the ANY_*
  generic functions, and the standard library - conversions (large,
  mechanical), TON/TOF/TP, CTU/CTD/CTUD, R_TRIG/F_TRIG, SR/RS, the string
  functions. A project of months, not weeks.
- **matiec is the oracle** while it lasts: every program in
  `toolchain/examples/`, every program in docs/usage.md and the pipeline
  tests (`toolchain/tests/test_pipeline.py`,
  `test_division_by_zero.py`, ...) build with both, run on a real scan
  engine, and must agree - except the differences decided and written
  down.
- **Behaviour changes are allowed**: pre-alpha, no field devices. The
  "stick to matiec" decisions of 2026-10-04 (abi §6: 12-byte TIME,
  seconds in `DINT_TO_TIME`; standard-functions §2b: division by zero)
  were made with matiec as the compiler; iecf decides them again (§6.3),
  and the matiec comparison lists them as expected differences.
- **An existing compiler to read first**: RuSTy (PLC-lang/rusty), ST in
  Rust over LLVM, with some object orientation; could reach wasm32.
  LGPL-3.0 (GitHub, 2026-11-05), active (last push 2026-11-04). iecf is
  Apache-2.0 OR MIT, so ideas are taken from it, not code (§6.4). Read
  in RUSTY.md: CODESYS dialect rather than the standard as written; no
  IL, SFC, configurations, namespaces or runtime checks; its parser
  needs LLVM, so it cannot serve the Workbench as wasm.

## 5. Order, if accepted

0. ~~**Read RuSTy**~~ DONE 2026-11-05: [RUSTY.md](RUSTY.md), what iecf
   takes as design and what it avoids. Read, not run: Prokop's examples
   were not put through it (no Rust toolchain or LLVM 21 here). No code
   taken.
1. **Front end only** - parser and checker, the Workbench's live errors.
   STARTED 2026-11-05: `crates/iecf-syntax` (lexer, parser, syntax
   tree, no dependencies, builds for wasm32), `crates/iecf-diagnostics`
   (codes, each explained in `docs/errors/`, short and JSON output),
   `iecf check`. Reads the Ed.2 core, SFC and CONFIGURATION; Ed.3
   object orientation, namespaces and properties are reported as not
   yet compiled (E0009). The checker, `crates/iecf-sema`, same day:
   names across files, VAR_EXTERNAL against VAR_GLOBAL, Ed.3 Figure 11
   transcribed as data for implicit conversions, the standard function
   blocks of Tables 43-46, calls by Ed.3 6.6.1.4, conditions, literal
   ranges, reserved names, SFC and tasks (E0010-E0021). Silent on all 18
   Prokop example programs as the build hands them to matiec. Then
   `AT %I*` with `VAR_CONFIG` (E0022-E0024, Ed.3 6.5.5.4 and 6.8.2).
   Then the `VAR_EXTERNAL` extension behind `--extension
   implicit-externals` (§2b), and `docs/extensions.md`, the document of
   extensions Ed.3 clause 5 c asks for, its sections checked by a test.
   Next: the standard functions' signatures in full
   (today only their result types); the browser entry point (§2a) once
   the checker covers enough of the language to be worth showing.
   Useful at once, no risk to a running program. The TypeScript
   re-implementations of matiec's rules (§1d) retire as it covers them.
2. **C at matiec's parity, Ed.2**, compared against matiec in CI. The
   default once it agrees; matiec behind a flag for a while, then gone.
3. **Ed.3**: `STRING[n]`, LTIME and the other L types, implicit
   widening, `REF_TO`, namespaces, then METHOD/INTERFACE/CLASS.
4. **Ed.4 additions**: USTRING/UCHAR, the §6.9 mutexes and semaphores,
   properties, `ASSERT`; BCD last, marked deprecated.
5. **ST breakpoints and stepping** from the line map.
6. **LD and FBD** compiled with their own network mapping, if the ST
   route (graphical-languages) is not enough by then.

## 6. Decisions

1. ~~Which edition?~~ DECIDED 2026-11-05 (florin): **Ed.4 (2025), no
   IL** (§3a). IL has been deprecated since 2013 and Ed.4 removed it;
   Beckhoff's PLC++ made the same choice, and Ed.4's UTF-8 strings fix
   matiec's 7-bit limit. Prokop's IL (`toolchain/examples/demo_il.il`)
   stays on matiec while matiec lasts, or its examples are rewritten in
   ST. The build order is still Ed.2 core, Ed.3, then Ed.4 additions
   (§5).
2. ~~Written in what?~~ DECIDED 2026-11-05 (florin): **Rust**. One native
   binary for the build service, the same front end as wasm in the
   Workbench.
3. ~~matiec's semantics or new ones?~~ DECIDED 2026-11-05 (florin):
   **decided anew**. Division by zero, integer overflow and an index out
   of range FAULT in every language; integer-to-TIME counts milliseconds,
   as CODESYS and Siemens do. Both editions call division by zero and
   overflow errors and leave the handling to the implementer, who
   documents it; iecf's compliance statement does.
4. ~~RuSTy first?~~ DECIDED 2026-11-05 (florin): **read it, then build**
   (§5 step 0). LGPL-3.0, so ideas, not code.
5. The IEC's policy on its name, read before a public release - or a
   name without "iec". OPEN. Searched 2026-11-05:
   - iec.ch refuses automated reading (HTTP 403 on every page), so the
     IEC's own policy text is still unread; it needs a browser, or a
     question to the IEC Central Office.
   - USPTO (via Justia's mirror) lists, owned by the IEC: the word mark
     "INTERNATIONAL ELECTROTECHNICAL COMMISSION"; "IEC" as a **design**
     mark (reg. 3643788, the logo with lines and a circle), not as a
     plain word; and IECEE, IECEx, IECQ, IECRE. No plain "IEC" word mark
     was found; EUIPO and WIPO were not searched.
   - Naming a standard to state conformity ("IEC 61131-3 compliant") is
     what every vendor does, and Ed.3 clause 5 asks for exactly that
     statement.
   Proposed: keep `iecf`, never use the IEC logo, never say "certified"
   or imply endorsement, and add to the README: "iecf is not affiliated
   with or endorsed by the International Electrotechnical Commission
   (IEC). IEC 61131-3 is a standard of the IEC." Ask the IEC before a
   1.0 release.
6. ~~The repository?~~ DECIDED 2026-11-05 (florin: "the IEC Forge will
   have its own repo"): its own repository, `raspihats/iecf`. Prokop
   consumes it as a pinned input, as it does matiec today (README.md),
   and `toolchain/build.py` finds the binary the same way.
7. ~~Check in the browser or on the server?~~ DECIDED 2026-11-05
   (florin: the in-browser check "would probably be extremely helpful in
   case of a hosted server with thousands of users", and the decision
   "should not involve how Prokop works today"): **in the browser**,
   the same code as the build, from one release (§2a); and therefore no
   edited text as iecf's input: standard IEC 61131-3 files only (§2b).
