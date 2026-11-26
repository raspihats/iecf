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

The checker covers the standard: every declaration, type and ST
statement, SFC, classes, interfaces, namespaces and configurations, with
each diagnostic explained under [Diagnostics](errors.md). The C
generator covers programs over the integer, bit-string, real, time, date
and string types and the derived ones, references, the standard
functions and function blocks, the user's functions and function blocks,
classes and methods with inheritance and interfaces, SFC, and RETAIN;
what it does not cover yet is reported as E0009. Code written for
CODESYS and TwinCAT - TcOpen's libraries, OSCAT's - compiles with the
[extensions](extensions.md) it needs, each named.

The C is meant for controllers as small as an ESP32-S3: no allocation,
REAL functions in single precision where a platform asks for them, a
symbol table the platform can link out, and code smaller than matiec's
at `-Os` ([Performance](testing/performance.md)).

Every example in the Behaviour reference is checked: CI builds it with
iecf, runs it and compares the result and the flag with the page. The
generated C is checked against matiec's on the same programs
([Testing against matiec](testing/harness.md)), and against the programs
of Prokop, iecf's first user ([The Prokop survey](testing/survey.md)).

## Standards

iecf follows IEC 61131-3 Ed.3 (2013), and the clause and table numbers
in this book are Ed.3's unless they say otherwise. Ed.4 (2025) is to
come; what it removed iecf already leaves out or warns of: IL is not
read (E0008), an octal literal and TRUNC without a type are warned
(W0001, W0003). What iecf accepts beyond Ed.3 is an
[extension](extensions.md), off unless named. iecf is not affiliated
with the IEC.

## License

iecf is licensed under either of the Apache License, Version 2.0, or the
MIT license, at your option.
