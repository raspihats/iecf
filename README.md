# iecf - the IEC Forge

An IEC 61131-3 (4th edition, 2025) compiler, written in Rust, that
translates Structured Text and SFC into portable C, with a symbol table, a published compliance statement and
diagnostics you can read. The compiler behind
[Prokop](https://github.com/raspihats/prokop).

## State

The front end has begun (PROPOSAL.md §5 step 1): a lexer and parser for
the Ed.2 core - POUs, declarations, types, every ST statement, SFC,
CONFIGURATION - with error recovery, and `iecf check`. It reads every
ST program in Prokop's repository without a diagnostic. No type checking
and no C yet.

```
cargo build --release
target/release/iecf check program.st          # file:line:col: error[E0005]: ...
target/release/iecf check --format json *.st  # for the Workbench
target/release/iecf explain E0006             # docs/errors/E0006.md
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
