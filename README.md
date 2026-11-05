# iecf - the IEC Forge

An IEC 61131-3 (4th edition, 2025) compiler, written in Rust, that
translates Structured Text and SFC into portable C, with a symbol table, a published compliance statement and
diagnostics you can read. The compiler behind
[Prokop](https://github.com/raspihats/prokop).

## State

A proposal, no code yet. [PROPOSAL.md](PROPOSAL.md) has:

- the limitations of matiec, the compiler Prokop uses today, as Prokop met them;
- what iecf would be: C out, its own symbol table, semantics it defines;
- how it claims IEC 61131-3 compliance - against the standard's own text,
  in a compliance statement it publishes;
- an order of work, and the decisions: Ed.4 without IL, Rust, one
  defined answer for division by zero and overflow, RuSTy read first.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
