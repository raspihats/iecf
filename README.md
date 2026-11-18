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
  the choices Ed.3 leaves open are in
  [docs/dependencies.md](docs/dependencies.md).
- **The C generator** (`iecf build`) writes portable C99
  ([docs/c-output.md](docs/c-output.md)) for the whole of the above;
  an overflow wraps and is flagged, a division by zero faults. What it
  does not compile yet is E0009.
- **Testing**: [harness/](harness/) runs iecf's C on 136 programs,
  beside matiec's where matiec compiles them, under gcc and zig cc, C99
  and C23; every example of the behaviour reference is built and run by
  CI.
- **Not yet**: the compliance statement Ed.3 clause 5.3 asks for (each
  feature table's features, by number), and Ed.4's additions.

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
  in a compliance statement it publishes (not written yet);
- what iecf takes from reading RuSTy, another ST compiler in Rust
  ([RUSTY.md](RUSTY.md));
- an order of work, and the decisions: Ed.4 without IL, Rust, one
  defined answer for division by zero and overflow, RuSTy read first.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
