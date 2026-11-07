# iecf — the IEC Forge

iecf is a compiler for IEC 61131-3, the language standard of
programmable logic controllers. It reads Structured Text and SFC, checks
them against the standard as written, and writes portable C99 that any
C compiler can build for any controller, with a symbol table beside it.
It is written in Rust, runs on the command line and, compiled to
WebAssembly, in a browser. It is the compiler behind
[Prokop](https://github.com/raspihats/prokop).

This book is for three readers:

- **who writes PLC programs**: [Getting started](getting-started.md),
  and the [Behaviour reference](behaviour/overview.md) — what each
  operation gives, its edge cases included: an overflow, a division by
  zero, a conversion that does not fit, a string too long, a date past
  its range;
- **who puts iecf's C on a controller**: [The C output](c-output.md) —
  the entry points, the process image, faults and flags, the symbol
  table, RETAIN;
- **who wants to know why**: [Implementer choices](dependencies.md),
  where each choice the standard leaves open is made after checking
  what CODESYS, TwinCAT and Siemens do, and the [design](design/proposal.md).

## Where it stands

The checker covers the standard's core: every declaration, type and ST
statement, SFC, configurations, with each diagnostic explained under
[Diagnostics](errors.md). The C generator covers programs over the
integer, bit-string, real, time, date and string types and the derived
ones, the standard functions and function blocks, the user's functions
and function blocks, and RETAIN. What it does not cover yet is reported
as E0009.

Every example in the Behaviour reference is checked: CI builds it with
iecf, runs it and compares the result and the flag with the page. The
generated C is checked against matiec's on the same programs
([Testing against matiec](testing/harness.md)), and against the programs
of Prokop, iecf's first user ([The Prokop survey](testing/survey.md)).

## Standards

iecf follows IEC 61131-3 Ed.4 (2025) without IL, which Ed.4 removed; the
clause and table numbers in this book are Ed.3's (2013) unless they say
otherwise. iecf is not affiliated with the IEC.

## License

iecf is licensed under either of the Apache License, Version 2.0, or the
MIT license, at your option.
