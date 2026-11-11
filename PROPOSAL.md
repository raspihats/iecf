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
| CONCAT_DATE, CONCAT_TOD, CONCAT_DT, SPLIT_DATE, SPLIT_TOD, SPLIT_DT | Ed.3 Table 36; not in its library, so the parser refuses a call (found 2026-11-08, harness `parts`) |
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

Found by the harness (§5 step 2, `harness/`), 2026-11-06, matiec 7949c0b:

- **The counters stop early.** CTU and CTUD count up only while
  `CV < PV`, CTD down only while `CV > 0`; Ed.3 Table 45 has
  `CV < PVmax` and `CV > PVmin`. A CTU with PV 3 stops at 3 after four
  pulses (`harness/cases/edges.st`).
- **Program connections in a CONFIGURATION**: a literal given to a
  program input, `PROGRAM a WITH fast : counter(inc := 1);`, crashes
  iec2c (SIGSEGV); a global given to it, or an output connected with
  `=>`, gives C that does not compile (`B.INC = *TEN`). Ed.2 allows
  both.
- **An array in VAR_TEMP** gives C that does not compile
  (`__SET_VAR(data__->,ARR,,)`).
- **A number times a duration**, `n * T#1s` (Ed.2 Table 30 allows
  either order), gives C that does not compile: `__time_mul` is passed
  its arguments the wrong way round. Ed.3's MUL_TIME and DIV_TIME are
  unknown; matiec has Ed.2's MULTIME and DIVTIME.
- **A negated output**, `NOT Q => x` or `NOT ENO => x` (Ed.3 6.6.1.4,
  a call's output inverted as it is taken), is "Internal compiler error
  in file function_call_param_iterator.cc at line 588", for a function
  block's output or a function's ENO alike (found 2026-11-08,
  `harness/cases/fbskip.st`).
- **A function block called with EN FALSE** still has its outputs taken:
  `fb(EN := FALSE, Q => x)` sets x from the instance
  (`harness/cases/fbenable.st`). Ed.3 leaves it to the implementer, but
  Example 4, the external form, takes nothing.

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
- **Semantics it defines**, not C's (§6.3): an index out of range and a
  division by zero FAULT, the same in every language; an integer
  overflow wraps and is flagged, as the vendors wrap; each checked by
  plain C comparisons before each operation (no
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
`dbfd009`+`7c1d847` as a wasm module are 189 KB, 71 KB gzipped; the
browser's module of step 1 complete (all of the checker, the standard
functions' tables, the JSON), built for size, 269 KB, 94 KB gzipped,
reported by CI on every push - a complete compiler several times that
is still smaller than the code editor a web IDE loads anyway.

Built: `crates/iecf-driver` is the one entry point, files in and
diagnostics out, that `iecf check` and the browser both call;
`crates/iecf-wasm` exports it with no JavaScript glue generated and no
dependency, and `iecf.js` wraps it in one call,
`iecf.check([{name, text}], {extensions})`, which returns what
`iecf check --format json` prints. CI runs that module in Node against
the same expectations.

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
address stands only in `VAR` of a PROGRAM or FUNCTION_BLOCK (Ed.3
Annex A, as CODESYS), or a structure field declared there, and every `*` of
every program instance gets its address, in the same area. CODESYS's
form - `VAR_CONFIG` in a list of its own, paths from the PROGRAM's name
- is read with `--extension var-config-list` (E0025 without it);
Beckhoff's, linking `%I*` in its I/O configuration, is outside the
source text and not read. The `VAR_EXTERNAL` extension is in, off by
default. `VAR_ACCESS` is still
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
  comparison (§4);
- the implementer's choices the standard asks for (clause 5 d), each
  made after checking what the big vendors do, with the evidence:
  `docs/dependencies.md`;
- the readings taken where the standard's own text disagrees. One so
  far: conversions between a string and a non-character type
  (`INT_TO_STRING`, `STRING_TO_LREAL`). Ed.2 defines them (Table 22,
  note e: the string holds the value as its literal is written); Ed.3
  Figure 11 marks them "-", none defined, yet Ed.3 states it is "fully
  compatible" with Ed.2 and does not list them among its deprecations
  (Annex B). iecf accepts them as Ed.2's, kept. Every other "-" of
  Figure 11 (`TIME_TO_DINT`) is an error. To be read again against
  Ed.4.

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

0. ~~**Read RuSTy**~~ DONE 2026-11-05: [RUSTY.md](https://github.com/raspihats/iecf/blob/main/RUSTY.md), what iecf
   takes as design and what it avoids. Read, not run: Prokop's examples
   were not put through it (no Rust toolchain or LLVM 21 here). No code
   taken.
1. **Front end only** - parser and checker, the Workbench's live errors.
   DONE 2026-11-05: `crates/iecf-syntax` (lexer, parser, syntax
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
   Then the standard functions in full (Ed.3 Tables 22-37): inputs by
   position or by their names (`IN1`, `G`, `MN`, `K`, ...), the type
   class of each, the common type of the overloaded ones, the typed
   forms (`ADD_INT`, `MUX_UINT_REAL`, 6.6.2.4), SPLIT's outputs, and
   conversions against Figure 11. Silent again on the 18 examples.
   Then the browser entry point (§2a): `iecf-driver`, shared with the
   command line, and `iecf-wasm` with `iecf.js`. Step 1 is complete.
   Useful at once, no risk to a running program. The TypeScript
   re-implementations of matiec's rules (§1d) retire as it covers them.
2. **C at matiec's parity, Ed.2**, compared against matiec in CI. The
   default once it agrees; matiec behind a flag for a while, then gone.
   STARTED 2026-11-05: the contract first, `docs/c-output.md` - entry
   points, time passed in, the process image read byte by byte, faults
   that return instead of jumping, a symbol table by `offsetof`, RETAIN
   in a region of its own - and `runtime/iecf.h`, tested in CI with GCC
   and run on wasm32. The representations decided after the vendors
   (`docs/dependencies.md`). The harness, 2026-11-06 (`harness/`):
   ten cases of the Ed.2 core, each run for a number of ticks with
   scripted inputs, its Q and M bytes traced; matiec's traces checked by
   hand and kept, CI building matiec 7949c0b to confirm them. It found
   two more matiec bugs (§1b). Then the generator in slices, each
   turning cases on. The first, 2026-11-06 (`crates/iecf-codegen`,
   `iecf build`): PROGRAM instances on tasks, variables of the integer,
   bit-string and BOOL types, located or not, every ST statement, the
   integer operators, conversions, shifts, ABS, MIN, MAX, LIMIT; every
   overflow, division by zero and conversion out of range a fault (since
   changed, §6.3). The
   checker now hands the generator what it worked out (`iecf_sema::analyze`).
   `image`, `arith`, `bits`, `control` and `tasks` match matiec's traces;
   `overflow` shows the fault where matiec wraps. The C is checked as
   strict C99 with every warning an error and run under the sanitizers.
   The second, the same day: the standard function blocks (bistables,
   edges, counters, timers) in `runtime/iecf.h`, TIME and LTIME, and
   the user's FUNCTIONs; the counters count over their type's range
   (decided, `docs/dependencies.md`). `timers` matches matiec's trace;
   `edges` differs where matiec stops its counters at PV and 0
   (`edges.iecf.trace`). The third: the user's FUNCTION_BLOCKs - inputs,
   outputs, VAR_IN_OUT bound by address, locals, instances of other
   blocks, VAR_EXTERNAL - and `pous` matches matiec's trace. The fourth:
   REAL and LREAL - arithmetic, division by zero a fault, conversions
   (to integers half to even, out of range a fault; DWORD_TO_REAL as bits),
   TRUNC, the Table 28 functions through `<math.h>` only when called -
   and `real` matches matiec's trace. Then an infinite or NaN real was
   decided flagged, not faulted (`infinity`), and an integer overflow and
   an integer conversion out of range the same: wrapped and flagged
   (§6.3), so `overflow` and the new `narrow` match matiec's bytes, with
   the flag; a real converted to an integer it does not fit is flagged
   and leaves its statement undone (`unconverted`). The fifth: derived
   types - enumerated, subrange (a value outside it clamped and flagged,
   decided; `subrange`), ARRAY (any dimensions, bounds not at 0, every
   index checked, an index out of bounds a fault) and STRUCT, their
   initial values, whole arrays and structures assigned - and `types`
   matches matiec's trace: every case passes. The sixth: VAR_TEMP in
   PROGRAMs and FUNCTION_BLOCKs, C locals set at each call, and derived
   types among a FUNCTION's locals (`temp`). The seventh: the symbol
   table - a symbol per top-level variable, each type described once
   (decided, florin: as ADS and CODESYS do), offsets by `offsetof` - which
   the harness checks holds together in every case. The eighth: RETAIN -
   one region, `ctx->retain`, that a warm start (`plant_warm`) leaves as
   it is - and `retain` shows it across a warm restart (the harness's
   scripts gained `TICK warm`). The ninth: program connections - a
   program's inputs and outputs, given and taken in the CONFIGURATION,
   checked as a function block call's - which matiec cannot compile, so
   `connections` has a hand-checked trace alone (`harness/reference-skip`).
   Step 2's slices are done. Then Prokop's own programs (2026-11-06):
   every source its toolchain tests hand matiec, 56, caught by a
   stand-in `iec2c`. matiec takes 52; iecf built 35 of them, and those
   that write `%Q` or `%M` matched matiec's trace in the harness, but for
   addresses that overlap (iecf's one image aliases them, matiec's host
   keeps a cell each) and a division by zero (a fault in iecf; matiec's C
   traps). What stopped the rest: STRING (5), the operators written as
   functions (3, and Prokop's LD, FBD and benchmark examples), RETAIN in
   a FUNCTION_BLOCK (2), IL (2, not iecf's), TOD, SFC, an array as a
   block's parameter (1 each); and iecf took a VAR block that mixes
   located and plain declarations, which §1e says to refuse. The tenth
   slice: the operators as functions - ADD, MUL, AND, OR, XOR with any
   number of inputs, SUB, DIV, MOD, NOT, the comparisons, MOVE, SEL
   (both inputs evaluated, as any function's) - and EXPT and `**`
   through `pow`; the mixed block is E0022. `functions` matches matiec's
   trace, and 39 of the 52 build. The eleventh: STRING - `char[n + 1]`,
   assignment, comparison, Table 34's functions into buffers sized for
   the most their inputs make; a string too long cut and flagged, a
   length or position out of its string flagged with Siemens' result
   (decided, florin, docs/dependencies.md) - and `strings` matches
   matiec's bytes; 41 of the 52 build. The twelfth: conversions to and
   from STRING, their text decided (florin, docs/dependencies.md) - %g
   for the reals from exact digits, decimal for the bit strings, the
   literal for a duration; text read as CODESYS reads it, none or out of
   range 0 and flagged - and `convert` matches matiec where their text
   agrees; 42 of the 52 build. The thirteenth: RETAIN and NON_RETAIN
   inside a FUNCTION_BLOCK, Ed.3 6.5.6 as written, an instance of a
   block that keeps anything kept whole as on CODESYS (decided, florin,
   docs/c-output.md §7) - and `fbretain` shows a warm restart. The
   fourteenth: DATE, TOD and DT - Table 30's arithmetic, DT_TO_TOD,
   DT_TO_DATE, CONCAT_DATE_TOD, DAY_OF_WEEK, their text both ways; a
   TOD wraps at midnight, a DATE, DT or difference out of range wraps
   and is flagged, a DT drops milliseconds (decided, florin,
   docs/dependencies.md) - and `dates` matches matiec but where its TOD
   runs past midnight. Prokop's programs again (survey/, 2026-11-07):
   46 of the 51 matiec builds build; TIME * and / by a number stop two,
   EN and ENO, an array as a block's parameter, SFC and IL one each
   (survey/results.md). The fifteenth: a duration times or divided by
   a number - by an integer exact, wrapped and flagged; by a real
   rounded to whole milliseconds half to even, as REAL_TO_DINT
   (decided, florin, docs/dependencies.md) - and `durations`; 47 of
   the 51 build. The sixteenth: EN and ENO on a function call - EN
   FALSE skips the call and leaves its statement undone, ENO FALSE on
   the function's own flag (decided, florin, docs/dependencies.md) - and
   arrays, structures and STRINGs as a FUNCTION_BLOCK's inputs and
   outputs, copied; MUX; `enable` and `blockdata`; 49 of the 51 build,
   SFC and IL left. The seventeenth: SFC in a PROGRAM or
   FUNCTION_BLOCK - actions then transitions each scan, with final scan,
   two passes, the first true transition by PRIORITY then source order,
   an unsafe entry flagged (decided, florin, docs/dependencies.md) - and
   `sfcdemo`, `sfc`; 50 of the 51 build, the IL one left, which iecf
   does not take. Then the harness compares every variable, not only
   the Q and M bytes (`iecf-harness compare`): 44 of the 50 run the same
   as matiec for 40 scans on random inputs, and the 6 others differ
   where iecf decided so; it found a FOR leaving its variable at the
   end, now one step past, as the standard's examples (decided, florin).
   Then the symbol table as JSON beside the C, which the harness reads
   to compare every array element, field and block member too: still 44
   of the 50. Then CONCAT_DATE, CONCAT_TOD, CONCAT_DT and the SPLITs
   (Table 36, which matiec lacks): a part out of range gives the type's
   default value, flagged, as CODESYS and PLCnext give it (decided,
   florin, docs/dependencies.md); a SPLIT's parts converted to its
   outputs' types; `parts`, a hand-checked trace. Then EN and ENO on a
   function block's call, as on a function's: EN FALSE calls nothing -
   inputs neither evaluated nor set, outputs not taken - and ENO is
   FALSE when a flag is raised while the block runs (decided, florin,
   docs/dependencies.md); `fbenable`, `fbskip`. Then a program's
   VAR_IN_OUT: a pointer the CONFIGURATION binds to a global, each
   in-out required connected (Ed.3 6.6.1.4 rule 2); `progio`. Then ENO
   in a POU's own body: there without a declaration in every FUNCTION
   and FUNCTION_BLOCK, the caller's ENO FALSE when the body set it so or
   a flag was raised (decided, florin, docs/dependencies.md);
   `enobody`. Then EN and ENO on a SPLIT: EN FALSE keeps every output,
   a part too big for its output clears ENO (decided, florin,
   docs/dependencies.md); `splitenable`. Then LDATE, LTOD and LDT, and
   TIME and LTIME converted: signed 64-bit nanoseconds over all their
   range, a long value made a short one rounding down, one the short
   type cannot hold its default value, flagged (decided, florin,
   docs/dependencies.md); `ldates`. Step 2 is done.
3. **Ed.3**: `STRING[n]`, the time and date types at an address, implicit
   widening, `REF_TO`, namespaces, then METHOD/INTERFACE/CLASS.
   `STRING[n]` came with step 2. The time and date types at an address,
   DONE 2026-11-08: a variable starts at its address and takes its own
   type's size, warned when the size prefix differs (W0004), a BOOL at
   a byte the whole byte (decided, florin, docs/dependencies.md, after
   Ed.3 6.5.5.3 and CODESYS); a program's time or date output connected
   to a longer type converted; `located`. Arrays, structures and STRINGs
   at an address, DONE 2026-11-08: their parts back to back from the
   address, a BOOL a byte, W0004 on the first part (decided, florin,
   docs/dependencies.md, after CODESYS's pack_mode); `locdata`.
   `REF_TO`, DONE 2026-11-08: a NULL reference dereferenced faults;
   REF() of a temporary (E0026, as Ed.3), of a CONSTANT or of a block's
   output from outside refused, of an input warned (W0005); references
   into the process image built; a RETAIN reference refused (E0027);
   = and <> between two (decided, florin, docs/dependencies.md); arrays
   of other bounds made other types; `refs` on both compilers, and
   `refimage`. CODESYS's REFERENCE TO as the extension `reference-to`,
   DONE 2026-11-09 (decided, florin): REF=, __ISVALIDREF, a block's
   REFERENCE TO input bound by a call; generated as the `REF_TO` it
   stands for; `refto`, iecf's only. Implicit widening audited
   2026-11-09: Figure 11 checked against the 2013 text; a block's output
   taken into a wider type (rule 5) and a short date with a long operand
   built, `DT + LTIME` typed LDT (it was DT, and let an LTIME narrow);
   `widen`, iecf's only. Two integers that both widen to a third,
   INT + UINT: that third, the narrowest signed one, as Siemens SCL
   (decided, florin, 2026-11-09, docs/dependencies.md). Typed
   functions (`ADD_INT`, Table 21) generated 2026-11-09, their table in
   the book; with Table 30's ADD_TIME, SUB_TIME, ADD_LTIME and
   SUB_LTIME. A FUNCTION's outputs generated 2026-11-09: each at its
   initial value per call, given back on return, nothing stored on a
   fault; `fnout` on both compilers and the compare list, `fnoutx`
   iecf's only. A FUNCTION's in-outs and externals 2026-11-09, with the
   standard's order of evaluation once calls have effects (7.3.2: the
   leftmost operand first), which also fixed outputs stored before a read
   left of the call; `fninout` on both compilers, `fnorder` iecf's only.
   CHAR 2026-11-09: literals (`CHAR#'X'`, `CHAR#16#41`, `'X'` where a
   CHAR is wanted), to STRING, Table 25 and 27's conversions, an empty
   string to CHAR `'$00'` flagged (docs/dependencies.md); `chars`. A
   FUNCTION without a result 2026-11-09, `void`, a statement only;
   in `fnoutx`. The implicit-externals extension compiled in every POU
   2026-11-09 (the checker allowed it, the generator gave E0009);
   `implicit`. Namespaces, DONE 2026-11-09 (NAMESPACES.md, its eight
   decisions florin's): NAMESPACE, INTERNAL and USING; a short name
   found from inside out as C#'s, whose wording Ed.3 6.9.4 follows; a
   namespace's element named in the C by its path joined by `__`;
   `_.` for the global namespace as the extension `global-namespace`,
   as Siemens and PLCnext write it; `namespaces` over two files, and
   `globalns`, both iecf's only. WSTRING and WCHAR, DONE 2026-11-10: a
   WCHAR one UTF-16 unit, a character above U+FFFF a surrogate pair that
   LEN counts as two; WSTRING_TO_STRING and WCHAR_TO_CHAR skip a
   character a STRING cannot hold, flagged, as Beckhoff skips;
   WSTRING_TO_WCHAR as Ed.3 Table 27 has it, though Figure 11 has none
   (all three decided, florin, docs/dependencies.md); the runtime's
   string functions made for both widths by one macro; `wstrings`,
   iecf's only. Then SEL, MUX, MOVE, MIN, MAX and LIMIT of strings, and
   a WSTRING at an address, its units little-endian at any byte
   (`wlocated`), and REF() of one there (`wrefs`), with `k := r^` of a
   string and `cp := rp^` of a structure, 2026-11-10. METHOD, CLASS
   and INTERFACE PROPOSED 2026-11-10 in OO.md, in three steps
   (methods; inheritance and dynamic binding; interfaces), its twelve
   decisions florin's, as recommended, the same day. Step A, methods and
   classes bound statically, BUILT 2026-11-10: `methods`, iecf's only.
   Step B, inheritance and dynamic binding (EXTENDS, OVERRIDE, FINAL,
   ABSTRACT, SUPER, a call through THIS, an in-out or a reference run
   from the instance's table), BUILT 2026-11-10: `inherit`, iecf's
   only. Step C, interfaces (INTERFACE, IMPLEMENTS, an interface's
   variable and its methods' calls, `=`, `?=`), BUILT 2026-11-10:
   `interfaces`, iecf's only. Function block and class instances
   besides a POU's own variable, DONE 2026-11-10: an instance's initial
   value (its inputs, outputs and PUBLIC variables, Ed.3 6.6.3.3), an
   instance as an array's element or a structure's field (Table 11 rows
   5a-7b) called and read there (Figure 8 d, e), the index checked
   before the call, and VAR_GLOBAL instances; each kept across a warm
   start as an instance in its section is; OO.md D11 applied to what
   holds an object; `instances`, iecf's only. A FUNCTION's and a
   method's inputs, outputs, in-outs and results of a STRING, array or
   structure type, DONE 2026-11-10: passed by pointer, an input copied
   in (a STRING cut and flagged at its declaration), a result into a
   buffer of the caller's statement, a STRING one a string expression;
   through interfaces and overrides; `fnparams`, iecf's only (a method,
   an interface and WSTRING in it). An interface wherever a variable
   can be (Ed.3 6.6.6.5.1), DONE 2026-11-11: an array's element, a
   structure's field, a VAR_GLOBAL, a block's output, a FUNCTION's and
   a method's output and result, an initial value that is an instance;
   `itfplaces`, iecf's only. A PROGRAM's input or output of one,
   connected in the CONFIGURATION, and a RETAIN one refused (E0027) as a
   RETAIN reference is (decided, florin, docs/dependencies.md);
   `itfprog`, iecf's only. Array bounds, STRING lengths, subrange limits
   and initial values naming a CONSTANT (Ed.3 6.6.1.2.4; E0032 for a
   bound that is not constant), `ARRAY[*]` in-outs of a FUNCTION, a
   method and a FUNCTION_BLOCK with LOWER_BOUND and UPPER_BOUND (Ed.3
   6.5.3; E0033 elsewhere), and a FUNCTION_BLOCK's in-outs of an array,
   a structure or a STRING, DONE 2026-11-11 (docs/dependencies.md);
   `flexarrays`, iecf's only. A CASE's labels constant expressions
   naming a CONSTANT (Ed.3 7.3.3.3.3), compared by value, a variable
   there E0032, DONE 2026-11-11; in `flexarrays`. Part of an array
   given to an `ARRAY[*]`: a row of an array of arrays, `aa[2]`, not
   Example 2's `A2[2]` of an `ARRAY [a, b]` (docs/dependencies.md,
   2026-11-11, florin to confirm); in `flexarrays`. Types with named
   values used as their base type (Ed.3 6.4.4.3), a value not given one
   more than the one before, bit string base types, INT when none is
   named (decided, florin: as CODESYS and TwinCAT); an enumeration as a
   FUNCTION's input, output and result and a block's in-out; one
   declared in place, DONE 2026-11-11 (docs/dependencies.md, florin to
   confirm the choices); `enums`, iecf's only. An `ARRAY[*]`
   input or output of a FUNCTION or a method, the caller's array and
   not a copy, as Siemens passes an FC's (decided, florin,
   docs/dependencies.md), DONE 2026-11-11; `flexio`, iecf's only.
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
   **decided anew**. Division by zero and an index out of range FAULT in
   every language; integer-to-TIME counts milliseconds, as CODESYS and
   Siemens do. Both editions call division by zero and overflow errors
   and leave the handling to the implementer, who documents it; iecf's
   compliance statement does. Revised 2026-11-06 (florin), after the
   vendors (docs/dependencies.md): an integer overflow and an integer
   conversion out of range wrap, two's complement, and are flagged in
   `ctx->flag`, as an infinite or NaN real is; the task goes on. A real
   converted to an integer it does not fit is flagged and leaves its
   statement undone (docs/dependencies.md).
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
8. ~~**Bit string to BOOL?**~~ DECIDED 2026-11-08 (florin): **as the
   vendors, an extension.** Ed.3 contradicts itself: Figure 11 defines
   no conversion from BYTE, WORD, DWORD or LWORD to BOOL, and Table 25
   lists `BYTE_TO_BOOL` and the others as "Binary transfer of the
   rightmost bit into the target". CODESYS and TwinCAT accept them and
   give TRUE for any value but 0; iecf gives the same with
   `--extension bit-string-to-bool`, and E0013 without it, as Figure 11
   (docs/extensions.md).
