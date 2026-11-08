# iecf - the IEC Forge

An IEC 61131-3 (4th edition, 2025) compiler, written in Rust, that
translates Structured Text and SFC into portable C, with a symbol table, a published compliance statement and
diagnostics you can read. The compiler behind
[Prokop](https://github.com/raspihats/prokop).

**The book: <https://raspihats.github.io/iecf/>** — getting started, the
behaviour reference (what every operation gives, edge cases included,
each example checked by CI), the C output, and the implementer choices.
It is built from [docs/](docs/) with [mdBook](https://rust-lang.github.io/mdBook/):
`mdbook serve` shows it locally.

## State

The front end is under way (PROPOSAL.md §5 step 1):

- a lexer and parser for the Ed.2 core - POUs, declarations, types,
  every ST statement, SFC, CONFIGURATION - with error recovery;
- a checker across a project's files: names, VAR_EXTERNAL against
  VAR_GLOBAL, types and the implicit conversions of Ed.3 Figure 11,
  calls (the standard functions by their tables), conditions, literal ranges, reserved names, SFC steps and
  actions, tasks, `AT %I*` completed by `VAR_CONFIG`;
- the same check in a browser: `crates/iecf-wasm` and `iecf.js`, a
  WebAssembly module of 94 KB gzipped, no dependencies;
- `iecf check`, by the standard as written; extensions only when asked
  for by name, each described in [docs/extensions.md](docs/extensions.md).

It checks every example program in Prokop's repository, as Prokop's
build hands it to matiec, without a diagnostic.

The C generator is started (§5 step 2): `iecf build` writes portable C99
([docs/c-output.md](docs/c-output.md)) for PROGRAMs over the integer,
bit-string, BOOL, REAL, TIME, DATE, TOD, DT and STRING types and the derived ones (enumerations,
subranges, arrays, structures), located or not, the standard function
blocks, the user's FUNCTIONs and FUNCTION_BLOCKs, every ST statement, SFC, and the integer
operators, conversions and shifts; an overflow wraps and is flagged, a
division by zero faults.
[harness/](harness/) runs that C beside matiec's on the same programs;
all thirty-four of its cases pass, twenty-one of them on a trace of iecf's own,
where iecf differs from matiec on purpose or matiec cannot compile them. The rest is E0009 for now.

```
cargo build --release
target/release/iecf check program.st          # file:line:col: error[E0005]: ...
target/release/iecf build -o out *.st         # out/plant.c, plant.h, iecf.h, plant.symbols.json
target/release/iecf check --format json *.st  # for the Workbench
target/release/iecf explain E0006             # docs/errors/E0006.md
target/release/iecf check --extension implicit-externals *.st
```

Every diagnostic code has its explanation in [docs/errors/](docs/errors/).

[PROPOSAL.md](PROPOSAL.md) has:

- the limitations of matiec, the compiler Prokop uses today, as Prokop met them;
- what iecf would be: C out, its own symbol table, semantics it defines;
- how it claims IEC 61131-3 compliance - against the standard's own text,
  in a compliance statement it publishes;
- what iecf takes from reading RuSTy, another ST compiler in Rust
  ([RUSTY.md](RUSTY.md));
- an order of work, and the decisions: Ed.4 without IL, Rust, one
  defined answer for division by zero and overflow, RuSTy read first.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
