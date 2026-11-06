# The harness: matiec's C and iecf's, run side by side

PROPOSAL.md §5 step 2: iecf's C reaches matiec's parity, compared in CI.
This directory is that comparison.

## A case

`cases/<name>.st` is a program and its CONFIGURATION, plain IEC 61131-3,
the same file for both compilers. `cases/<name>.cycles` says how many
ticks to run and what the inputs are:

```text
ticks 6                 # one tick is the CONFIGURATION's base interval
0 %IB0 16#03            # at tick 0, before it runs
3 %IX1.2 TRUE
4 %IW2 -5
```

The program shows its results by writing them to `%Q` and `%M`.
`cases/<name>.trace` is the trace: one line after initialisation and
one per tick, the Q and M bytes in hex, as far as the program addresses
them. Addresses count bytes and are little-endian (docs/c-output.md §4).

The trace is matiec's, written by the harness and checked by hand. A case
where iecf decides differently from matiec on purpose - overflow, the
counters' limits (PROPOSAL.md §1b) - has `cases/<name>.iecf.trace`
beside it, and the reason in the case's comment: `overflow` and
`narrow` (an integer overflow or conversion out of range flagged; the
bytes are matiec's), `edges` (the counters count over their type's
range), `infinity` (an infinite or NaN real flagged) and `unconverted`
(a real that does not fit an integer leaves its statement undone, where
matiec's C is undefined) and `subrange` (a value outside a subrange
clamped and flagged, where matiec stores it). A flag ends the line with
`flag CODE POU LINE`.
A fault ends the line with `fault CODE POU LINE` (docs/c-output.md §5).

The cases cover the Ed.2 core a slice at a time: `image` (each size of
address), `arith`, `bits`, `control` (every ST statement), `timers`,
`edges` (counters, edges, bistables), `pous` (user FUNCTIONs and
FUNCTION_BLOCKs, VAR_IN_OUT, globals), `types` (enumerations, subranges,
arrays, structures), `real`, `tasks` (two intervals), `temp` (VAR_TEMP),
`overflow` and
`narrow` (where iecf flags), `infinity`, `unconverted`, `subrange`. Apart
from those five they stay
inside what both compilers define: no overflow, no conversion out of
range.
A test runs `iecf check` on each: errors fail it, warnings (a deprecated
form both compilers read) do not.

## Running it

```text
cargo run -p iecf-harness -- matiec --matiec ~/matiec          # compare
cargo run -p iecf-harness -- matiec --matiec ~/matiec --bless  # write the traces
cargo run -p iecf-harness -- matiec --matiec ~/matiec timers   # one case
cargo run -p iecf-harness -- iecf                              # iecf's C
cargo run -p iecf-harness -- iecf --bless overflow             # write its .iecf.trace
```

`--matiec` (or `$IECF_MATIEC_DIR`) is a built `beremiz/matiec` checkout:
`iec2c` at its top, `lib/` beside it. CI builds commit 7949c0b, the one
the traces were written with. `$CC` is the C compiler, `cc` by default.

How matiec's C is run: `matiec_host.c` keeps matiec's located variables
in its own variables, copies the image into them before a tick and the
changed ones back after (matiec has no process image of its own), sets
`__CURRENT_TIME` to the tick times the base interval, and calls
`config_run__`. `matiec_fixes.h` covers two gaps between matiec's code
generator and its library, as Prokop does (PROPOSAL.md §1b).

The iecf side builds each case with iecf (the library, as `iecf build`
does), compiles the C as strict C99 with every warning an error
(docs/c-output.md §1), and runs it under `iecf_host.c`: the platform side
of the contract in a page - the image, the context, a scheduler that
runs each task on the ticks its interval divides, by priority. CI runs it
with the sanitizers on. No matiec is needed.

The generator is built in slices; `iecf-pending` lists the cases it does
not cover yet. Each must fail with E0009 alone, and the run fails when
one of them builds - it comes off the list then, and its trace must
match.
