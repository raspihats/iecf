# iecf - the IEC Forge

An IEC 61131-3 compiler, written in Rust, that translates Structured
Text and SFC into portable C, with a symbol table and diagnostics you
can read. It follows Ed.3 (2013); Ed.4 (2025) is to come. The compiler
behind [Prokop](https://github.com/raspihats/prokop).

**The book: <https://raspihats.github.io/iecf/>** — getting started, the
behaviour reference (what every operation gives, edge cases included,
each example checked by CI), the C output, and the implementer choices.
It is built from [docs/](docs/) with [mdBook](https://rust-lang.github.io/mdBook/):
`mdbook serve` shows it locally.

## State

- **The checker** reads ST and SFC - every declaration, type and
  statement, classes, interfaces, namespaces, CONFIGURATION - and checks
  them against Ed.3 as written; each diagnostic is explained in
  [docs/errors/](docs/errors/). The same check runs in a browser
  (`crates/iecf-wasm`, `iecf.js`).
- **Extensions** beyond Ed.3 are off unless named, each described with
  how to do without it in [docs/extensions.md](docs/extensions.md);
  `--extension codesys` names those code written for CODESYS and
  TwinCAT uses, at once;
  the choices Ed.3 leaves open are in
  [docs/dependencies.md](docs/dependencies.md).
- **The C generator** (`iecf build`) writes portable C99
  ([docs/c-output.md](docs/c-output.md)) for the whole of the above;
  an overflow wraps and is flagged, a division by zero faults. What it
  does not compile yet is E0009.
- **For small controllers**: the runtime header computes a REAL's
  functions in single precision where the platform defines
  `IECF_REAL_FLOAT_MATH`, for an FPU without double; `<p>.h` names
  every symbol's offset as a constant, so a platform can find its
  values without the symbol table and link the table out; a block's
  body is `static inline`, a hint the C compiler weighs
  ([docs/c-output.md](docs/c-output.md)).
- **Testing**: [harness/](harness/) runs iecf's C on 217 programs,
  beside matiec's where matiec compiles them, under gcc and zig cc, C99
  and C23, and with the runtime's variants (no `<stdckdint.h>`, no
  compiler built-ins, single-precision REAL functions); every example of
  the behaviour reference is built and run by CI.
- **Performance**: on standard programs iecf's C is faster than
  matiec's on integer arithmetic, strings, blocks, timers and SFC,
  level on an array sort, and within 10 % on REAL mathematics and
  structure copies, where it tests what matiec does not. At `-Os` its
  code is 15 % smaller than matiec's on OSCAT's library. On Prokop's
  ESP32-S3 it is ahead of matiec on integer arithmetic, strings, REAL
  mathematics (in single precision) and OSCAT, level on LREAL, behind
  on block calls, an array sort, structure copies and SFC. A project the size of TcOpen is checked in
  under a second. How it is measured, the numbers, and what was tried
  are in [docs/testing/performance.md](docs/testing/performance.md),
  the programs in [tools/bench/](tools/bench/).
- **Compliance**: the statement Ed.3 clause 5.3 asks for, each feature
  table's features by number, is [docs/compliance.md](docs/compliance.md).
  Not yet: Ed.4's additions.

```
cargo build --release
target/release/iecf check program.st          # file:line:col: error[E0005]: ...
target/release/iecf build -o out *.st         # out/plant.c, plant.h, iecf.h, plant.symbols.json
target/release/iecf check --format json *.st  # for the Workbench
target/release/iecf explain E0006             # docs/errors/E0006.md
target/release/iecf check --extension implicit-externals *.st
```

Every diagnostic code has its explanation in [docs/errors/](docs/errors/).

Every addition or fix to iecf is documented where a reader finds it:
this README, if it changes what iecf is or does, and the book
([docs/](docs/)), in the same commit - [CONTRIBUTING.md](CONTRIBUTING.md).

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
