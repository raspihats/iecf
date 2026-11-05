# Reading RuSTy - what iecf takes, and what it avoids

Step 0 of PROPOSAL.md §5, decided in §6.4. Read 2026-11-05 at
PLC-lang/rusty `6356681` (2026-11-04), files only: nothing was built or
run (no Rust toolchain and no LLVM 21 here). RuSTy is **LGPL-3.0**;
iecf is Apache-2.0 OR MIT, so what follows is design in our own words,
never code. `path:line` is RuSTy's. Where a note says *inferred*, it was
reasoned, not read.

RuSTy's own mdBook (`book/`) is frank about its deviations and its
regrets, and is the best single source.

## 1. What it is

An ST compiler in Rust over LLVM 21 (inkwell), about 90k lines:
parse, index, resolve (annotate), validate, then a chain of lowering
passes, then LLVM codegen and link. A stdlib (`libs/stdlib`) declares
its functions in ST and implements most of them in Rust behind a C ABI.

It cannot be iecf's dependency, license aside:

- **The parser is in the root crate, and the root crate links LLVM.**
  No front end builds for wasm32 without LLVM; only the lexer, AST,
  source and diagnostics crates are free of it.
- **Not built for a long-running process.** File names are leaked to
  `'static` (`compiler/plc_source/src/lib.rs:55`) and node ids come from
  one global counter, so every re-parse in an editor leaks and
  renumbers.
- **A parse error in one file stops the project** before indexing
  (`src/parser.rs:48-63` and the driver): the editor would lose every
  semantic error in the other files.
- **No wasm, no LSP, no JSON diagnostics.**

## 2. Coverage against IEC 61131-3

| Feature | RuSTy |
| --- | --- |
| ST, ACTIONS, JMP/labels, CONTINUE | yes |
| IL, SFC | **no** (no tokens at all) |
| LD / FBD | no; CFC only, from PLCopen XML, transpiled to ST |
| CONFIGURATION / RESOURCE / TASK, VAR_ACCESS | **no**; programs are `*_instance` globals |
| Namespaces / USING | **no** (`src/parser.rs:325`: "TODO: Parse USING directives") |
| Located `AT %IX...`, `VAR_CONFIG`, `AT %I*` | yes |
| RETAIN | partly: moved to a `.retain` section, but the initial value is written at every start |
| STRING[n], WSTRING | yes (UTF-8 and UTF-16), assignment truncates silently |
| USTRING / UCHAR, mutexes and semaphores (Ed.4) | no |
| LTIME, LDATE, LTOD, LDT | yes, 64-bit signed ns |
| TIME, DATE, TOD, DT | 32-bit **unsigned**: TIME ms (`T#500us` is 0, about 49 days, negative wraps), DATE/DT s since 1970 (`book/user/language/time.md`) |
| Integer-to-TIME | milliseconds (as iecf §6.3) |
| REF_TO, REF, `^`, NULL, REFERENCE TO | yes, plus non-standard POINTER TO, ADR |
| VLA `ARRAY[*]` | yes, fat struct {pointer, bounds} |
| CLASS, METHOD, INTERFACE, EXTENDS, IMPLEMENTS | yes, vtables and itables |
| THIS in a CLASS | **rejected**, contrary to Ed.3 |
| PUBLIC/PRIVATE/PROTECTED/INTERNAL, ABSTRACT, FINAL | **parsed and ignored** (`book/user/language/inheritance.md:155-161`) |
| Properties | yes, lowered to `__get_x`/`__set_x` methods |
| EN/ENO | in CFC only |
| R_EDGE / F_EDGE declarations | no |
| Implicit conversion per Ed.3 Fig. 11/12 | **no**: C-like promotion to DINT; every numeric conversion allowed, narrowing only warns |
| Division by zero, overflow, index, NULL | **no checks**: plain `sdiv`/`srem` (`src/codegen/generators/expression_generator.rs:2275-2290`), wrapping arithmetic, `inbounds` GEPs; subranges only checked if the user writes CODESYS-style `CheckRange*` functions |
| Timers | read the host wall clock (`libs/stdlib/src/timers.rs:48`), state in an opaque byte buffer in the FB |
| Generic ANY_* functions | name mangling only (`MAX__DINT`); a missing instance fails at link time |

So RuSTy aims at the CODESYS dialect, not at the standard as written.
Little of iecf's Ed.4 compliance work is done there.

## 3. Take for iecf

**Front end**

1. **A hand-written recursive-descent parser with region-stack error
   recovery.** Each construct with a known end pushes its closing
   tokens (`END_VAR`, `)`, `;`); on an error the parser skips to the
   nearest token that closes this or an outer region, reports the
   skipped text once, and puts a placeholder node where an operand was
   missing (`compiler/plc_lexer/src/lexer.rs:178-232`). Several errors
   in one run, no cascade.
2. **Dialect spellings mapped to the standard token with a warning**
   (`ENDVAR` is `END_VAR`, warning E013). iecf can accept what CODESYS
   users type and still say what the standard spells.
3. **Declaration and body apart.** A POU's declaration holds its
   variables, base, interfaces and properties; bodies are separate, and
   actions, methods and property accessors are all just bodies named
   `Parent.member`.
4. **Node ids with side tables.** Types and resolutions are recorded off
   the tree, keyed by node id; the tree is never changed to record a type.
5. **A location that can name a diagram element**
   (`CodeSpan::Block { diagram, order, pin }`,
   `compiler/plc_source/src/source_location.rs:81-90`). Prokop's LD, FBD
   and Blockly generate ST; iecf's errors on that ST should point at the
   element, not at generated text.

**Diagnostics**

6. **One markdown file per error code** (165 of them, each with the
   mistake and the fix) feeding the registry, a `plc explain E037`
   command and the published error book. That is the Ed.3 clause 5 e
   document ("all errors that are detectable and reported"), generated.
7. **User-remappable severities** per code (`--error-config`), and a
   clang-style one-line format beside the rich one.

**Middle**

8. **Fixpoint constant evaluation that keeps the reason** a constant
   failed (overflow, not constant), turned into a diagnostic later.
   Needed for `STRING[n]`, array bounds and subranges.
9. **Name every anonymous type early** (an inline array gets a
   generated type name). Done with names stable across files, that ends
   matiec's "anonymous ARRAY across VAR_EXTERNAL" mismatch.
10. **Built-in signatures declared in ST**, parsed like user code;
    implementations native.
11. **A resolved type and an expected type per expression** (RuSTy's
    annotation and hint), with the conversion decided between them.
12. **The lowering catalogue**, as a checklist: loops to WHILE with
    EXIT; ELSIF to nested IF; properties to get/set methods;
    inheritance as an embedded `__Base` first member, `SUPER` to it;
    interfaces as a fat pointer {data, itable}; a STRING, array or
    struct result as a hidden caller-owned out-parameter; per-type
    initializer functions; a function returning a reference with a
    hidden result. After them the program is structs, function pointers,
    plain functions, WHILE and IF - which is C.
13. **vtables and itables.** A hidden table pointer in each root FB or
    class, one table struct per type, one itable per (interface, POU)
    pair, fat pointers for interface variables; calls through `SUPER`
    direct. Plain C structs of function pointers, `call_indirect` in
    wasm.
14. **The calling convention.** FUNCTION: inputs by value, IN_OUT and
    OUTPUT by pointer. FB and PROGRAM: `void f(Instance *)`, the caller
    writes inputs into the instance and copies `=>` outputs after.
    Method: instance pointer first.

**Outputs**

15. **A map from instance path to symbol to address**
    (`--hwmap-file`, JSON or TOML, `book/technical/outputs/01-hardware-map.md`).
    iecf extends it to every variable, with type, offset, size and
    retain: the symbol table matiec lacks (PROPOSAL §1d).
16. **A C header of the instance layouts** (`compiler/plc_header_generator`),
    emitted with the C, as the ABI contract for Prokop's glue.
17. **Stop points and dumps**: `--check` after validation, the tree
    after parsing and after lowering, for snapshot tests.

**Runtime and tests**

18. **One `no_std` literal and conversion core** shared by the compiler
    and the runtime library (`compiler/plc_literals`), so a literal that
    compiles to a value converts to the same value at run time
    (`STRING_TO_TIME`). iecf: the same core, compiled into the runtime
    or generated as C.
19. **Three test tiers**: snapshot tests (insta) of trees and of
    rendered diagnostics; one-ST-file-per-test that compiles, runs and
    checks printed output (their LLVM `lit`, about 800 files); and the
    same suite at two optimisation levels to expose undefined behaviour.
    iecf adds UBSan on the emitted C.

## 4. Avoid

- **Lowering by rewriting the AST in place** and re-indexing and
  re-annotating after each pass (nine times a build). Their own book
  calls it technical debt and says a dedicated IR would have fitted
  better. iecf: AST, then a typed HIR (where validation runs), then a
  C-oriented IR.
- **Conversions kept only in a side table.** iecf makes every implicit
  conversion an explicit node after resolution - where the Ed.3
  Fig. 11/12 rules are enforced and where FAULT checks are emitted.
- **One fact computed twice.** Their validator and resolver bind mixed
  named and positional arguments differently, a documented bug
  (`book/technical/pipeline/04-validation.md`). Bind once, store it.
- **String-keyed types** lowercased at every lookup: intern names, use
  ids.
- **A parse error that stops the project**, leaked file names, a global
  id counter: iecf is per-file and incremental-ready from the start,
  because it runs in the editor.
- **Parse and ignore.** Access specifiers, ABSTRACT and FINAL accepted
  and meaningless is worse than refused.
- **Undefined behaviour at run time.** C has the same undefined
  behaviour as LLVM on division by zero and `INT_MIN / -1`; iecf emits
  the checks (§6.3).
- **TIME as 32-bit unsigned.** Keep milliseconds for integer-to-TIME;
  store TIME signed and 64-bit (matiec's is 12 bytes, spec/abi.md §6).
- **Semantics left to the host or the linker**: wall-clock timers
  (iecf's timers read the scan's time from the runtime, deterministic
  and testable), state in opaque byte buffers (breaks retain and the
  symbol table), missing generic instances found at link time (iecf
  instantiates or refuses at compile time), retain values overwritten at
  every start.
- **Comments and pragmas thrown away by the lexer.** iecf keeps them as
  trivia, for attribute pragmas and an editor-grade syntax tree.
- **Mangled addresses without the size letter**: their `%QW2.5` and
  `%QX2.5` collide (stated in their hardware-map doc).

## 5. What it changes in the proposal

- The pipeline in PROPOSAL §2 gains its stages: parse (per file,
  recovering) → index → resolve → typed HIR + validation → lowering
  (§3 item 12) → C IR → C, plus the symbol table, the layout header and
  the error document as outputs.
- A C backend for RuSTy instead of iecf (contributing, rejected in
  §6.4) would still leave IL, SFC, configurations, namespaces, the
  runtime checks, Ed.4 and the wasm front end to do, under LGPL, on an
  AST-rewriting pipeline its authors want to replace. Building iecf
  stands.
