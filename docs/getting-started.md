# Getting started

## Building iecf

iecf is a Rust workspace with no dependencies beyond the standard
library.

```sh
git clone https://github.com/raspihats/iecf
cd iecf
cargo build --release
```

The compiler is `target/release/iecf`.

## Checking a program

`iecf check` reads IEC 61131-3 files — any number, a project's POUs,
types and configuration across them — and reports every diagnostic:

```text
$ iecf check main.st
main.st:12:9: error[E0013]: Ed.3 Figure 11 defines no conversion from BYTE to BOOL
```

Each code has its explanation, also on the command line:

```sh
iecf explain E0013
```

`--format json` gives the diagnostics as JSON, for an editor.

## Building C

`iecf build` writes the C for each CONFIGURATION of a project:

```sh
iecf build -o out *.st
```

`out/` then holds `<configuration>.c`, `<configuration>.h` and
`<configuration>.symbols.json`, the symbol table for tools ([The C
output](c-output.md) §6), for each, and `iecf.h`, the runtime header,
once. The C is portable C99, no compiler extensions, no allocation.
[The C output](c-output.md) says what the platform has to give it — a
process image and a clock — and what it gives back.

## A first program

```iec
PROGRAM main
VAR
    start AT %IX0.0 : BOOL;
    motor AT %QX0.0 : BOOL;
    run : TON;
END_VAR
run(IN := start, PT := T#2s);
motor := run.Q;
END_PROGRAM

CONFIGURATION plant
    RESOURCE cpu ON PLC
        TASK fast(INTERVAL := T#10ms, PRIORITY := 0);
        PROGRAM p WITH fast : main;
    END_RESOURCE
END_CONFIGURATION
```

## Extensions

By default iecf checks the standard as written. A few things vendors
accept that the standard does not — a global used without
`VAR_EXTERNAL`, TIME converted to an integer — are
[extensions](extensions.md), each switched on by name:

```sh
iecf check --extension implicit-externals *.st
```

## In a browser

`crates/iecf-wasm` builds the checker as a WebAssembly module of about
95 KB gzipped, and `iecf.js` wraps it:

```js
import { load } from "./iecf.js";
const iecf = await load(fetch("iecf.wasm"));
iecf.check([{ name: "main.st", text }]);
// -> [{ code, severity, message, file, line, column, ... }]
```

The diagnostics are those `iecf check --format json` gives.
