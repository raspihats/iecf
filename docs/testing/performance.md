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
