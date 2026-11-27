# Performance

How fast iecf's C runs, beside matiec's (the reference compiler), and
how fast iecf itself checks and compiles; what made it faster, and what
was tried and not kept. Measured 2026-11-25 and 26 on one x86-64 Linux
machine with GCC at `-O2`. Numbers taken minutes apart moved up to 15 %
while the machine was busy: a comparison below is of runs made side by
side, and a difference under that is noise.

## How it is measured

The programs are in `tools/bench/`, plain IEC 61131-3 that both
compilers build (`oo.st` alone needs `--extension codesys`, and is
iecf's only). Each does its work many times a cycle, so that the cycle
is long next to the host's own cost, and writes a checksum of its
results to `%Q`. `tools/bench/run.sh` builds one with iecf and with the
reference compiler, compiles each C with `-O2` and the harness's host
(harness/README.md: `iecf_host.c`, `reference_host.c`), runs it the
cycles asked, the best of five timed, and compares the two traces, Q
and M bytes each cycle:

```
cargo build --release
REFERENCE=path/to/the/reference/compiler tools/bench/run.sh reals 2000
IECF_ARGS="--extension codesys" tools/bench/run.sh oo 500
```

Every program gives the same trace on both compilers but `sfc`, where
iecf runs an action once more as its step is left (its final scan,
[SFC](../behaviour/sfc.md)) and matiec does not: iecf does more work
there. A change to iecf's C kept its traces; where it could change a
flag, a harness case shows the flags it gave before (below).

The C an older iecf writes is measured the same way, `IECF=path/to/iecf`
beside.

## Results

### The programs, a cycle

Each row a run of the two side by side, iecf at abb5810:

| program | what it does | matiec | iecf |
|---|---|---|---|
| `arith` | DINT products and MOD, ROL, XOR, SQRT | 108 us | 84 us |
| `sort` | an insertion sort of 300 DINTs | 29 | 31 |
| `fbcalls` | 10 block instances and a FUNCTION, 50 times | 72 | 57 |
| `strings` | CONCAT, LEFT, RIGHT, LEN, FIND, conversions | 164 | 62 |
| `reals` | LREAL SIN, COS, EXP, LN, SQRT, a division | 171 | 186 |
| `structs` | nested structures copied, 8000 times | 85 | 89 |
| `sfc` | 40 charts of four steps, N actions | 2.75 | 1.86 |

What the changes below made of the last three, iecf before them (at
2c76441) and after, each pair measured together:

| program | iecf before | iecf after |
|---|---|---|
| `reals` | 250 to 270 us | 186 us |
| `structs` | 187 | 89 |
| `sfc` | 3.5 | 1.86 |

The `r_` programs take `reals` apart, one statement each:

| program | matiec | iecf before | iecf after |
|---|---|---|---|
| `r_explog`, EXP and LN | 60 us | 98 to 114 us | 64 us |
| `r_sincos`, SIN times COS | 99 | 115 | 100 |
| `r_sqrtdiv`, SQRT over a sum | 19 | 31 | 23 |
| `r_real`, a REAL product and conversion | 10 | 18 to 23 | 18 |
| `r_arr`, an array element updated | 11 | 23 to 32 | 18 |
| `r_conv`, two single operations | 6 | 9 to 10 | 9 |

### One construct each, 20000 times a cycle

| program | matiec | iecf |
|---|---|---|
| `m_iadd`, a DINT sum | 41 us | 32 us |
| `m_imul`, a DINT product and MOD | 114 | 86 |
| `m_iarr`, an array element updated | 36 | 26 |
| `m_bool`, BOOL logic and IF | 42 | 33 |
| `m_case`, CASE | 41 | 27 |
| `m_while`, WHILE | 89 | 65 |
| `m_lradd`, an LREAL sum | 22 | 25 |
| `m_fcall`, a FUNCTION | 99 | 66 |
| `m_fbcall`, a FUNCTION_BLOCK | 67 | 63 |
| `m_ton`, a TON | 89 | 46 |
| `m_struct`, a structure copied | 206 | 173 |
| `m_str`, CONCAT and LEN | 877 | 95 |

`m_empty`, a cycle of one statement: 0.5 us on both, most of it the
host writing its trace line. (These at bcd87a2, iecf's C the same at
abb5810.)

`oo` (iecf only): a method called on an instance, through an interface,
through a `REFERENCE TO` and through a `POINTER TO`, and a property read
through an interface: 3 to 5 ns each, the loop's arithmetic with it.

### A library: OSCAT

OSCAT BASIC (not in this repository; its licence) through a program
that calls 169 of its FUNCTIONs and FUNCTION_BLOCKs every cycle - those
with elementary inputs, that iecf checks with no extension, and whose
results the two compilers agreed on when the harness compared them
(2026-11-19) - each input on its own `%I` address, the same
pseudo-random bytes there each cycle for both compilers: 126 us a cycle
for iecf, 130 for matiec. 15 of the 169 are left out, as iecf faults in
them on such inputs and stops the program (docs/c-output.md §5) where
matiec's C goes on - a negative shift count, as in `BIT_LOAD_B2`,
undefined in C, or a REAL divided by zero, as in `COTH`, an infinity
in C, a fault in iecf: `BIT_LOAD_B`,
`BIT_LOAD_B2`, `BIT_LOAD_W`, `BIT_LOAD_W2`, `BIT_LOAD_DW`,
`BIT_LOAD_DW2`, `BIT_TOGGLE_B`, `BIT_TOGGLE_W`, `BIT_TOGGLE_DW`,
`BIT_OF_DWORD`, `SHL1`, `SHR1`, `COTH`, `COSH`, and `LANGEVIN`, which
calls COTH. The program is made by a script outside this repository
from OSCAT's files.

### Compiling

| | iecf | matiec |
|---|---|---|
| a program above to C | 5 to 8 ms | 210 to 320 ms |
| its C to an object (`cc -O2`) | about 0.3 s | about 0.45 s |
| the OSCAT program to C | 108 ms | 512 ms |
| its C to an object | 1.9 s | 2.2 s |

### Code size

The OSCAT program, its sections apart (`size -A`; measured again
2026-11-26, where 63 KB against 41 KB had counted iecf's symbol table's
strings as code):

| | iecf `-O2` | iecf `-Os` | matiec `-O2` | matiec `-Os` |
|---|---|---|---|---|
| machine code (`.text`) | 50.0 KB | 30.1 KB | 40.6 KB | 35.5 KB |
| the symbol table (§6), its entries | 31.0 KB | 31.0 KB | | |
| its names and paths (`.rodata`) | 11.8 KB | 11.8 KB | | |

At `-Os` iecf's code is the smaller by 15 %, with its tests - an
overflow's flag, an index's bounds, an infinite or NaN real, a fault
stopping the cycle (docs/c-output.md §1, §5) - that matiec's C does not
make. At `-O2` it is the larger by 23 %: GCC inlines more of the REAL
tests' code there, and without them the code is 22 KB smaller at `-O2`
and 7 KB at `-Os` (the tests stubbed out, to measure). Where size
counts more than speed, `-Os`.

The symbol table is data no code of the program reads: a platform that
does not look variables up at run time links it out with
`-ffunction-sections -fdata-sections -Wl,--gc-sections`, and the 42 KB
go (a `main` calling the start and the task alone: 30.4 KB of code,
0.4 KB of constants). matiec writes its list of variables to files
beside the C (`VARIABLES.csv`).

Tried to make the code smaller, its behaviour the same, and not kept:
the first flag stored without a branch (`-O2` 0.7 % smaller, `-Os`
0.2 % larger); every `if (c) iecf_flag(...)` of `iecf.h` branch-free
(0.5 % larger at `-O2`); a REAL result tested once, finite, before
which flag (1.2 % larger); the flag's choosing and recording out of
line, cold (6.8 % larger); the bookkeeping of `iecf_took` and
`iecf_stopped` out of line (15.5 % larger at `-O2`), each against the
`-O2` code's 50 KB. A call with its arguments takes more bytes than
the test and store it replaces. The REAL chains of bcd87a2, a fast path
beside the operator-by-operator one, cost 3.9 KB of the 50 at `-O2` and
1.7 KB at `-Os`.

### On an ESP32-S3

The programs above built as Prokop builds a program for its ESP32-S3
controller - the C to wasm32 by `zig cc` at `-O2`, then by `wamrc` to
Xtensa code the chip runs from flash (XIP) - and run on an
esp32s3_devkitc (240 MHz, no PSRAM) under Prokop's firmware, each with
its task's period made 1 s, as the work of a cycle is sized for a
desktop. The time is the last scan's, from the firmware's status line
every 5 s; three or more, which agreed to within 0.1 %. Measured
2026-11-26, iecf at 6c7690f, Prokop at 5bb209de, ms a scan:

| program | iecf `-O2` | iecf `-Os` | matiec `-O2` | matiec `-Os` |
|---|---|---|---|---|
| `arith` | 118.9 | 119.0 | 99.7 | 98.9 |
| `strings` | 14.4 | 13.2 | 174.3 | 124.2 |
| `fbcalls` | 10.9 | 10.9 | 3.0 | 6.9 |
| `sort` | 8.1 | 8.1 | 6.9 | 7.3 |
| `reals` | 329 | 329 | 315 | 315 |
| `realsf`, `reals` in REAL | 332 | 332 | 308 | 308 |
| `structs`, `sfc`, OSCAT | not loaded | not loaded | 16.5, 1.7, 6.2 | 16.2, 1.7, 6.2 |

The chip has instructions for a REAL but none for an LREAL, nor for a
REAL's SIN, EXP, ... computed in double: each such operation is a call
of a routine. iecf's three modules the firmware did not load carried
Prokop's table of their values in their memory, too large for the one
64 KB page the firmware gives a program (Prokop's glue, not iecf's C).

After this round - 1cffdd3, 793c3f2, 3530450 and ae4e3c2 in iecf, and in
Prokop its `wamrc` flags (64-bit multiplication and shifts inline), its
float functions with `IECF_REAL_FLOAT_MATH`, and its glue finding each
value by iecf's constants with a table of types rather than of values -
the same board, both compilers through Prokop's toolchain as it is now
(`-O2`), ms a scan:

| program | iecf before | iecf | matiec |
|---|---|---|---|
| `arith` | 118.9 | 104.6 | 99.7 |
| `strings` | 14.4 | 14.0 | 123.7 |
| `structs` | not loaded | 13.6 | 16.5 |
| `fbcalls` | 10.9 | 6.2 | 3.0 |
| `sort` | 8.1 | 8.1 | 6.9 |
| `reals` | 329 | 314 | 308 |
| `realsf` | 332 | 47.9 | 300 |
| `sfc` | not loaded | 2.8 | 1.7 |
| OSCAT, inputs constant | not loaded | 6.0 | 7.0 |

`sfc` needed a second page: its 40 charts' actions were iecf's own
context, 34 KB of it, each action holding the storage of every
ACTION_CONTROL qualifier though only N is used (856 bytes a chart).
An action now holds only its qualifiers' parts
([SFC](../behaviour/sfc.md)), and the context is 4 KB; the row is
iecf after that change. OSCAT's program reads its inputs from `%I`,
all 0 on the board: one of its functions then reaches `COTH(0)`, a
division by zero, which iecf faults on (docs/c-output.md §5) and
matiec's C does not; it was timed with its 451 inputs given as
constants of non-zero values (a copy).

Why `sfc` was slower than matiec's here though faster on x86: `wamrc`
built every load and store whose address is not a constant as single
bytes. WAMR's compiler gives such an access alignment 1, whatever the
wasm instruction states, and LLVM's Xtensa backend splits it - a 32-bit
read four byte loads, three shifts, three ORs. iecf reaches every
variable through `ctx` or `self`, so all of its code paid it; matiec's
program variables are at constant addresses, and only its function
blocks paid it. `sfc`'s program function was 22 KB of Xtensa code
against matiec's 6 KB. The ESP32-S3 loads and stores at any address in
hardware, so Prokop's `wamrc` now builds those accesses with the
alignment the instruction states, one instruction each (Prokop
21709636, its spec/aot-xtensa.md §2.2). The same programs and board,
ms a scan, with that `wamrc`:

| program | iecf | matiec |
|---|---|---|
| `arith` | 95.8 | 99.7 |
| `strings` | 14.1 | 119.4 |
| `structs` | 10.8 | 9.9 |
| `fbcalls` | 4.1 | 3.1 |
| `sort` | 4.7 | 3.5 |
| `reals` | 305 | 303 |
| `realsf` | 42.0 | 296 |
| `sfc` | 1.77 | 1.43 |
| OSCAT, inputs constant | 4.9 | 6.0 |

The outputs are the same as with the byte-wise code, read at every
scan. What is left of `sfc`'s gap is iecf's own work, each part
measured by taking it out of the C (before the `wamrc` change, 2.82
ms): a DINT `+` or `*` tested for overflow and the fault's place
recorded, about 0.55 ms; the test after each block call that the
program did not stop, 0.15; an action's final scan, 0.14 - all three
iecf's design, which matiec's C does not do. `reals` stays in
double, where the chip has no instructions.

Then the overflow tests themselves, cheaper on wasm (2026-11-27; the
study, each variant measured on the board, is in iecf-tools'
esp32/notes): a value that alone flags its statement ORs its overflow
into a local rather than reading `ctx->flag` each time (21666c4); a
DINT's and a LINT's `+` and `-` tested on their bits where the C
compiler targets wasm, as Xtensa makes a branch of each comparison the
compilers' test combines (9ea2d02); and a block body of 3 statements or
fewer always put into its calls, as those tests had tipped Clang
against inlining even `fbcalls`' 3-statement block (ec4d614). The same
board and programs, iecf at 2c46ac9 and at ec4d614, ms a scan:

| program | iecf before | iecf | matiec |
|---|---|---|---|
| `fbcalls` | 4.14 | **2.93** | 3.07 |
| `sfc` | 1.76 | 1.66 | 1.43 |
| `strings` | 14.05 | 13.84 | 119.4 |
| `arith` | 95.7 | 95.4 | 99.7 |
| OSCAT, inputs constant | 4.94 | 4.89 | 5.99 |
| `structs` | 10.77 | 10.78 | 9.94 |
| `sort` | 4.68 | 4.72 | 3.53 |
| `reals` | 304.7 | 304.6 | 303.3 |
| `realsf` | 42.0 | 42.0 | 295.9 |

Each program's outputs read the same from both builds at every scan
(11 to 13 each). On x86-64 under GCC `fbcalls` went 61 -> 44 us a cycle
(matiec's 75): GCC had kept `Filter` and `Mix` out of line, its stop
tests past GCC's inlining limit; the others within 1 us, and under
Clang all the same. What is left against matiec here: `sort`'s index
tests, which GCC proves away and Clang - the board's compiler - does
not; `structs`' copies; `sfc`'s final scan and stop tests.

Then, profiled on the board through its JTAG (iecf-tools'
esp32/scripts/profile.sh, sampling the scan core's program counter):
`structs`' copies were made byte by byte - `iecf_bytes`' `void *` lost
their alignment, and wasm said `align=1` - and are now the built-ins at
each call (dfb0d6f); `sort`'s inner loop kept its index tests, and a
WHILE loop's guarded variable now proves them (7da805d). Again, ms a
scan, iecf at ec4d614 and at 7da805d:

| program | iecf before | iecf | matiec |
|---|---|---|---|
| `structs` | 10.78 | **7.30** | 9.94 |
| `sort` | 4.72 | **3.24** | 3.53 |
| `sfc` | 1.66 | 1.64 | 1.43 |
| the others | | the same | |

The outputs read the same at every scan. On x86-64, `sort` under Clang
25 -> 21 us a cycle, GCC's the same (it proved those indexes itself), and
`structs` the same under both. iecf is now ahead of matiec on the board
on every program but `sfc`, and level on `reals`, in double. `sfc`'s
gap is not iecf's code, which runs faster than matiec's there (sampled,
about 0.95 ms of its 1.64 against 1.04 of 1.43): at the end of every scan
Prokop's glue copies every symbol's value to its live-values region, and
iecf's symbol table - each step's X and T, each action's Q and A, each
block's internals - is 3,848 bytes to matiec's 488.

Last, WAMR's own checks. It tests every memory access in software, the
address against the memory's size; with those tests off (a measurement
only - they are the sandbox) `sort` ran 3.24 -> 1.63 ms and `sfc` 1.64 ->
0.98. For a memory the module cannot grow Prokop's `wamrc` now tests
against constant bounds, which a large function no longer keeps on its
stack and reloads (Prokop 47e9dc65, its spec/aot-xtensa.md §2.3). The
same board and programs, both compilers through that `wamrc`, iecf at
ac1b8c6, ms a scan:

| program | iecf | matiec |
|---|---|---|
| `arith` | 93.7 | 99.8 |
| `strings` | 12.70 | 120.8 |
| `structs` | 7.17 | 9.31 |
| `fbcalls` | 2.00 | 3.06 |
| `sort` | 3.21 | 3.01 |
| `reals` | 303.4 | 302.9 |
| `realsf` | 41.2 | 295.5 |
| `sfc` | 1.08 | 1.04 |
| OSCAT, inputs constant | 4.26 | 5.54 |

iecf's outputs the same as before at every scan. iecf is ahead on six
programs, level on `reals` and `sfc` (4 %, where its larger symbol table
is copied each scan by Prokop's glue), and behind on `sort` by 7 %. What
is left of `sort` is the tests of its inner loop's addresses, which
change each pass: LLVM's IRCE recognises them as range checks but splits
no loop, and computing its `a[j + 1]` in 32 bits rather than INT's 16
gained 4 % - neither kept (iecf-tools' esp32/notes, 2026-11-27).

### The add-ons cost nothing

The C iecf wrote at 85d2c15 (2026-11-18), before the rounds of CODESYS
and TwinCAT extensions, and at 2c76441, with and without `--extension
codesys`, is the same, byte for byte, for every program above: a
standard program pays nothing for the extensions.

### Checking

| | before | after |
|---|---|---|
| TcOpen's 17 libraries, 590 files | 2.2 s | 0.61 s |
| TcOpen's 22 checks ([TcOpen](tcopen.md)) | 30.6 s | 7.6 s |
| OSCAT, 559 files | | 0.06 s |

The same diagnostics, byte for byte. Prokop's Workbench runs iecf's
checker as one types: a project the size of TcOpen is checked in well
under a second now.

## What made it faster

Each a commit of its own, its C's traces unchanged:

- **Structures and arrays copied by the compiler's block copy**
  (32d2b10). `iecf_bytes`, `iecf_move` and `iecf_zero` were byte
  loops, which GCC does not turn into a block copy at `-O2`; under GCC
  and Clang they are now `__builtin_memcpy`, `__builtin_memmove` and
  `__builtin_memset` (docs/c-output.md §1). `structs` 187 -> 89 us.
- **An SFC action's control worked out for the qualifiers it is
  given** (a475d91). Every action ran Ed.3's whole ACTION_CONTROL each
  scan, five timers, four flip-flops, two triggers; the parts no
  association of the action can drive are left out, an N action is its
  Q and final scan alone ([SFC](../behaviour/sfc.md)). `sfc` 3.5 ->
  1.86 us, matiec's 2.75 with less to do.
- **Standard functions, conversions and array elements in a REAL chain
  tested once** (bcd87a2). A REAL or LREAL result that is infinite or
  NaN is flagged (§5); a chain of `+ - *` was tested once at its end,
  and now so is one through `SQRT`, `LN`, `SIN`, ..., a conversion, an
  element, a divisor that is a result, its value worked out once
  (docs/c-output.md §5). `reals` 250 -> 186 us. The harness case
  `realchains` puts an infinity or a NaN through each kind of step, and
  its flags, codes and lines are those iecf gave before.
- **A real tested finite on its bits** (2026-11-26). `x - x == 0` was
  the test of every REAL and LREAL result; it is now the exponent's bits,
  not all ones, as an integer (docs/c-output.md §5). On x86-64 `reals`
  186 -> 172 us, `r_real` 17 -> 13, `m_lradd` 20 -> 17, `arith` 87 ->
  84, `r_arr` 17 -> 19 (each pair run side by side, the traces the
  same). On an ESP32-S3, which has no double instructions and so calls a
  routine for each floating point operation on an LREAL, `arith` 118.9
  -> 109.7 ms and `reals` 329 -> 322 (ESP32-S3, below). The OSCAT
  program's x86-64 code at `-O2` 55.2 -> 57.2 KB, at `-Os` the same
  (`gcc -c`, `.text`).
- **A REAL's functions in single precision, where the platform asks**
  (793c3f2). `-DIECF_REAL_FLOAT_MATH=1` makes a REAL's SIN, EXP, ...
  C99's `sinf`, ... (docs/c-output.md §1), which Prokop sets: on the
  ESP32-S3 `realsf` 333 -> 47.8 ms with Prokop's float functions. On
  x86-64, without it, the C is what it was.
- **A FUNCTION_BLOCK's body `static inline`** (docs/c-output.md, a
  hint the C compiler weighs by size). On the ESP32-S3, where a call
  between wasm functions run from flash costs most, `fbcalls` 9.7 -> 6.3
  ms and its code 24.5 -> 28.9 KB; `sfc`'s, `structs`' and OSCAT's code
  the same, their bodies too large or called once. On x86-64 the
  programs above run the same and OSCAT's code is the same: GCC took
  these calls inline already.
- **A large project checked four times as fast** (abb5810). Stacks
  sampled with gdb found the recursion check's call targets worked out
  for every call over every block, copying each type's bases each
  time; with `--extension recursion` a search from every node of the
  call graph (now strongly connected components, once); and the
  property accessors looked for over every method at every assignment
  to a member.

## Tried, not kept

- Testing a REAL result's finiteness with the flag's path marked
  unlikely (`__builtin_expect`): no change.
- Flags set without branches: 30 to 70 % slower, the tests then a chain
  of their own.
- One test a statement, the sum of `(t - t)` over every result, the
  flags then found again from the results kept: 10 % at best, the
  results kept across the calls of `exp` and `log` costing as much.
- An array index checked once for its target and its read: no change,
  GCC merges the two.
- A FOR loop of constant bounds as C's own `for`: 0 to 7 %.
- The rare paths of `iecf_took` and `iecf_stopped` out of line, cold:
  the code at `-O2` 15.5 % larger (Code size, above).

## What stays

A single REAL operation is tested on its own, as a chain needs two:
`r_real` and `r_conv` stay above matiec's, which tests nothing. Without
any REAL test `reals` ran 173 us where matiec's took 217 in the same
run; that difference is the price of a flagged infinity or NaN, which
iecf keeps (§5).

## The reference compiler, on the way

matiec segfaults on a FUNCTION_BLOCK declaration of several names, `f1,
f2 : Filter;` (stage 4, generate_var_list.cc), and writes C that does
not compile for an element of an array of structures given to a
FUNCTION's `VAR_IN_OUT`; the benchmark avoids both. It segfaulted on 4
of the harness's 131 standard cases too.
