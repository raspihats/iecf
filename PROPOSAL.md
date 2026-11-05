# iecf - the IEC Forge, an IEC 61131-3 compiler for Prokop

PROPOSED 2026-11-05, nothing decided but the working name. florin asked
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
purchased standard.

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
| Ed.3 object orientation | METHOD, INTERFACE, CLASS, EXTENDS, namespaces - none (check). No namespaces is why tags and globals.gvl share one flat namespace (io-tags §8) |
| Implicit widening conversions | `INT + DINT` and `r := 42` into a REAL are refused, so Blockly writes `42.0` and refuses mixed types (lblk-generate.ts). Ed.3 allows widening (check) |
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

Implementation-dependent in the standard, but inconsistent in matiec
(standard-functions §2b):

- **Integer division by zero**: ST `/` and IL `DIV` trap (FAULT); ST
  `MOD` gives 0; the `DIV()`/`MOD()` functions - and so FBD and LD - give
  0 with ENO FALSE; IL `MOD` traps.
- **Overflow**: `INT` and `SINT` wrap through C's promotion to `int`;
  `DINT` and `LINT` trap on the smallest value divided by -1. Signed
  overflow in the generated C is undefined behaviour, which clang may
  optimise on.
- **`DINT_TO_TIME(1500)` is 1500 seconds**; CODESYS and Siemens count
  milliseconds.
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

- An empty `VAR ... END_VAR` - the grammar wants at least one
  declaration (check).
- `DT`, `TOD` as names - keywords (check: and standard function names
  such as `LT`).
- Located and plain declarations in one block - separate productions in
  the Ed.2 grammar (check); CODESYS relaxes it.

## 2. What iecf is

- **Input**: ST, SFC's textual form, the declarations, CONFIGURATION /
  RESOURCE / TASK - written against the normative text of IEC 61131-3,
  Ed.3 (2013) at least (§6.1). IL by decision (§6.1).
- **Output: C**, compiled by the same zig/clang wasm32 step as today.
  The prokop-1 ABI, the process image, the retain and aux regions and
  every runtime stay as they are. `matiec_glue.c`'s role, the musl
  vendoring and the MicroPython link are unchanged.
- **A symbol table and layout it emits itself**, with arrays,
  structures, enumerations and function block members, replacing
  `VARIABLES.csv`, the positional pairing and most of the aux and retain
  bridges ("the compiler-emitted symbol table replaces it", abi §6).
- **Semantics it defines**, not C's: index checks that FAULT, overflow
  done with `__builtin_*_overflow` or unsigned arithmetic, one answer for
  division by zero in every language. Each is listed in the compliance
  statement.
- **Its own front end in the Workbench** (built to wasm): the same parser
  and checker give errors in the editor, on the right file and line,
  before a Build.
- **A statement-to-C line map**, so ST breakpoints are the tier-2 work
  Python already had (debugging §2b).
- **Apache-2.0 OR MIT**, like the rest of the repository; the GPLv3
  boundary goes away with matiec.

**Written in Rust**, proposed (§6.2): one native binary for the build
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
and its implementation-dependent parameters (check: the clause and
annex). iecf ships that statement as a document generated from its own
tables:

- the feature tables supported, by number;
- the parameters: STRING length, TIME range and resolution, division by
  zero, overflow, index out of range, identifier length;
- every deliberate difference from matiec, while matiec is still the
  comparison (§4).

The outside checks: PLCopen's conformance material and the TC6 XML
exchange format (check what PLCopen offers today). There is no formal
certification of a compiler to rely on.

The standard itself must be bought - Ed.3 (2013) and Ed.4 (2025) - and is
the source for every rule. Nothing is taken from CODESYS's documentation
as if it were the standard.

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
  were made with matiec as the compiler; with iecf they are decided again,
  from the standard (§6.3).
- **An existing compiler to read first**: RuSTy (PLC-lang/rusty), ST in
  Rust over LLVM, with some object orientation; could reach wasm32. Its
  license (believed LGPL) and coverage are unchecked (§6.4).

## 5. Order, if accepted

1. **Front end only** - parser and checker, the Workbench's live errors.
   Useful at once, no risk to a running program. The TypeScript
   re-implementations of matiec's rules (§1d) retire as it covers them.
2. **C at matiec's parity, Ed.2**, compared against matiec in CI. The
   default once it agrees; matiec behind a flag for a while, then gone.
3. **Ed.3**: `STRING[n]`, LTIME and the other L types, implicit
   widening, `REF_TO`, namespaces, then METHOD/INTERFACE/CLASS.
4. **ST breakpoints and stepping** from the line map.
5. **LD and FBD** compiled with their own network mapping, if the ST
   route (graphical-languages) is not enough by then.

## 6. Decisions for florin

1. Which edition: Ed.3 (2013) or Ed.4 (2025)? Ed.4 drops IL (check);
   Prokop supports IL today (`toolchain/examples/demo_il.il`).
2. Written in Rust (proposed), or TypeScript if the Workbench front end
   matters most?
3. The time types and division by zero: decided again from the standard,
   or matiec's kept?
4. Read RuSTy first, or build iecf from the start?
5. The IEC's policy on its name, read before a public release - or a
   name without "iec".
6. ~~The repository?~~ DECIDED 2026-11-05 (florin: "the IEC Forge will
   have its own repo"): its own repository, `raspihats/iecf`. Prokop
   consumes it as a pinned input, as it does matiec today (README.md),
   and `toolchain/build.py` finds the binary the same way.
