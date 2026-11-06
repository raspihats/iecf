# The C iecf emits, and what a platform supplies

PROPOSAL.md §2: iecf's output is portable C, as generic as C can be. This
is the contract between that C and the platform that compiles and runs
it. Step 2 (§5) builds to it. Where the standard leaves a choice, the
choice is made after checking CODESYS, Beckhoff TwinCAT and Siemens,
and recorded in `docs/dependencies.md`.

Status: DECIDED 2026-11-05 (florin), the address numbering and byte
order included. Built in slices from 2026-11-06 (PROPOSAL.md §5 step
2): what is built is said in each section; the symbol table (§6) and
RETAIN (§7) are not yet.

## 1. The language of the output

- **ISO C99, nothing else.** No compiler extensions, no built-ins
  (`__builtin_*`), no attributes, no `#pragma`. A build with
  `-std=c99 -pedantic -Werror` and every warning on is clean.
- **Freestanding.** The output and `iecf.h` include only the headers a
  freestanding C99 implementation must have: `<stdint.h>`, `<stddef.h>`,
  `<float.h>`, `<limits.h>`. No libc is taken for granted: strings,
  conversions and time arithmetic are iecf's own C. The one exception is
  the mathematics of Table 28 (`SQRT`, `SIN`, `LN`, ...), which come from
  `<math.h>` unless the platform defines `IECF_SQRT` and the others
  first; a program that calls none of them includes no `<math.h>`.
- **No assumption about the target**: not the word size, not the
  endianness, not the alignment, not that `int` is 32 bits. The types are
  `<stdint.h>`'s exact widths; byte order matters only at the process
  image, which is read and written byte by byte (§4).
- **No undefined behaviour**, on any compiler. Signed overflow, a shift
  by the width or more, a division by zero, an index out of range, a
  float-to-integer conversion out of range: each is checked with plain C
  comparisons before the operation and, instead of happening, faults or
  is flagged (§5). A signed overflow wraps in unsigned arithmetic, where
  C defines it.
- **No global state.** Everything a program has lives in one context
  structure the platform allocates; every generated function takes a
  pointer to it. Two instances of one program can run in one process (a
  simulator beside a live view, a test harness comparing two compilers).
- **Tested on two very different targets** in CI: GCC on a 64-bit host,
  and a 32-bit freestanding wasm32 target with no libc.

## 2. What iecf emits

For a project named `plant` (the CONFIGURATION's name):

| File | What |
| --- | --- |
| `plant.h` | the context type `plant_t`, the entry points (§3), the image sizes |
| `plant.c` | every POU, the tasks, initialisation, the symbol table (§6) |
| `iecf.h` | the runtime header, the same for every project: types, faults, image access, string and time helpers. Shipped with iecf, copied next to the output |

A C identifier is the IEC name upper-cased (IEC names ignore case) with a
prefix that keeps it clear of C keywords and of other kinds of names:
`P_` for a POU, `V_` for a variable, `T_` for a type. An enumerated
value is written as its number (§8).

- A PROGRAM is a structure `P_<NAME>_t`, its instances fields of the
  context, and two functions, `P_<NAME>_init` and `P_<NAME>` (built).
  Its inputs and outputs are fields too: a connection in the
  CONFIGURATION (`PROGRAM a WITH fast : counter(inc := 1, total => sum)`,
  Ed.3 6.8.2) sets each input from a literal, a global or an address
  before the instance runs, and writes each output to a global or an
  address after it, in the task's function (built; not yet a VAR_IN_OUT).
- A FUNCTION is `static <result> F_<NAME>(plant_t *ctx, <inputs>...)`:
  every input passed, the caller giving an input's initial value where
  the call does not name it; its variables are C locals, initialised at
  each call, as a function has no memory (Ed.3 6.6.2.2) (built).
- A FUNCTION_BLOCK is a structure `B_<NAME>_t` - its inputs, outputs and
  variables as fields `V_<NAME>`, a VAR_IN_OUT a pointer set at each call
  to the variable passed - and `B_<NAME>_init` and `B_<NAME>` (built).
- VAR_TEMP, in a PROGRAM or FUNCTION_BLOCK, is C locals of its body
  function, set to their initial values at each call, not fields of the
  instance (built).
- The standard function blocks are `iecf.h`'s: `iecf_TON_t`,
  `iecf_TON_init`, `iecf_TON(&fb, now_ns)`, their inputs and outputs by
  the standard's names; `iecf_CTU_DINT_t` for the typed forms (built).
- A STRUCT is a structure `T_<NAME>` with a field `V_<FIELD>` for each of
  its own; an ARRAY is a C array of as many dimensions, each counted from
  its lower bound (`ARRAY [-1..1] OF INT` is `IEC_INT V_A[3]`, `a[-1]` is
  `V_A[0]`). Every index that is not a constant is checked
  (`iecf_index`): out of its bounds it is a fault (§5), and an index of
  the variable assigned to is checked before anything is stored. A whole
  array or structure is assigned by copying its bytes (built).

## 3. The entry points

```c
void plant_init(plant_t *ctx);            /* cold start: every variable to its initial value */
void plant_warm(plant_t *ctx);            /* warm start: RETAIN kept, the rest initial (Ed.3 6.5.6) */
void plant_task_FAST(plant_t *ctx, int64_t now_ns);   /* one per TASK */
```

- A **task function** runs the program instances associated with that
  TASK, in the order the configuration lists them, once. The platform
  calls it: at the TASK's interval, or on its SINGLE trigger's rising
  edge. Interval, priority and trigger are in the symbol table (§6) for
  the platform's scheduler; iecf schedules nothing.
- **Time comes in as an argument**: `now_ns`, nanoseconds of a monotonic
  clock with any origin. Timers (TON, TOF, TP) measure against it. Nothing
  in the output reads a clock, so a test harness or a simulator decides
  what time it is.
- A program instance with no TASK runs in a task function of its own,
  `plant_task__default`, which the platform calls cyclically.
- A TASK of a named RESOURCE is `plant_task_<RESOURCE>__<TASK>`, upper
  case: `plant_task_R__FAST`.
- The tasks are also a table, for a scheduler that reads it (built):
  ```c
  typedef struct { const char *name; void (*run)(plant_t *, int64_t);
                   int64_t interval_ns; int64_t priority; } plant_task_t;
  extern const plant_task_t plant_tasks[];   /* interval 0: cyclic */
  extern const uint32_t plant_task_count;
  ```
  The time of the running task is `ctx->now_ns`.
- Until RETAIN is built, `plant_warm` is `plant_init`.
- Tasks of different priorities may preempt one another only if the
  platform says so: by default the output assumes one task runs at a
  time. Preemption and the §6.9 mutexes and semaphores of Ed.4 come with
  step 4.

## 4. The process image (directly represented variables)

A located variable (`start AT %IX0.3 : BOOL`) is read and written in the
platform's process image at each use - there is no copy of it in the
context. The image is three byte arrays, **I**, **Q** and **M**, which the
platform owns and points the context at:

```c
typedef struct { uint8_t *i, *q, *m; uint32_t i_size, q_size, m_size; } iecf_image_t;
```

`plant.h` says how many bytes of each the program addresses
(`PLANT_I_BYTES`, ...). `plant_init` faults (§5) if the platform gives
fewer.

- **The number is a byte offset.** `%IX3.5` is bit 5 of
  byte 3; `%IB3` is byte 3; `%IW3` is bytes 3 and 4; `%ID3` bytes 3 to 6;
  `%IL3` bytes 3 to 10. So `%IW0` and `%IW1` overlap, and `%IX1.0` is in
  both. Beckhoff ("addressing is always bytewise") and Siemens count this
  way, and so does Prokop's process image; CODESYS counts in units of the
  size (`%MD48` is bytes 192 to 195). Ed.3 6.5.5.2 leaves it to the
  implementer.
- **Little-endian.** A word's first byte is its low byte,
  as CODESYS and TwinCAT have it on their usual targets, as EtherCAT
  carries it, and as Prokop's image is; Siemens is big-endian. It is
  assembled byte by byte, so it is the same on a big-endian host.
- **One-level addresses only**, for now: `%IX1.2.3` (hierarchical) is
  refused at build time, as Prokop's image does.
- **An input's initial value** (`x AT %IX0.0 : BOOL := TRUE`) is not
  written by `plant_init`: the input image is the platform's. Those of
  `%Q` and `%M` variables are.
- **When the image changes** is the platform's business: the standard's
  model (IEC 61131-1) is that inputs are read before a task runs and
  outputs written after, and a platform that copies its I/O around each
  task function call gives exactly that.

## 5. Faults

A fault is an error the program meets while running: Ed.3 calls division
by zero and overflow errors and leaves their handling to the implementer
(PROPOSAL.md §6.3). iecf's handling: division by zero, and the errors no
value can carry on from, stop the task where it is; an overflow carries
on, wrapped, and is flagged.

- The generated code checks before every operation that could go wrong,
  and on a fault records it in the context and returns, from every
  function up to the task function. No `setjmp`, no `abort`, no signal.
- After a task function returns, the platform reads `ctx->fault`:
  ```c
  typedef struct { uint16_t code; uint16_t pou; uint32_t line; } iecf_fault_t;
  ```
  `code` says what (division by zero, index out of range, MUX's K out
  of range, a negative shift or string position, a string too long, an
  image too small), `pou` which
  POU (an index into `plant_pou_names`), `line` the source line of the
  statement.
- Not a fault, as Ed.3 defines it: `x MOD 0` is 0 (Table 29 e); SHL and
  SHR by the width or more give 0, ROL and ROR go round (Table 30).
- REAL and LREAL: division by zero is a fault.
- **Flagged, not faulted** (decided 2026-11-06, docs/dependencies.md):
  the value carries on, the task goes on, and the context's second
  record, `ctx->flag` (an `iecf_fault_t`), takes the first, its POU and
  line:
  - an integer result beyond its type, from + - * / NEG ABS: it wraps,
    two's complement (`32767 + 1` is `-32768`), `IECF_FAULT_OVERFLOW`;
  - an integer converted to one that cannot hold it: its low bits are
    kept (`INT_TO_SINT(4223)` is 127), `IECF_FAULT_CONVERSION`;
  - an infinite or NaN real, from + - * /, Table 28 (`SQRT(-1.0)`) or
    an LREAL beyond REAL's range made a REAL (the infinity of its sign):
    it carries on as IEEE 754 has it, `IECF_FAULT_OVERFLOW` for an
    infinity, `IECF_FAULT_NAN` for a NaN. The program can test a value
    with IS_VALID (Ed.3 6.6.2.5.15);
  - a value stored into a subrange outside its bounds: it becomes the
    bound it passed, `IECF_FAULT_CONVERSION` (a FOR over a subrange
    stops at the bound);
  - a real converted to an integer it does not fit, an infinity or a
    NaN among them, `IECF_FAULT_CONVERSION`: there is no integer to
    carry on with, so the statement is left undone (docs/dependencies.md).
    An assignment keeps its target's value; IF
    and CASE run no branch; FOR does not run; WHILE and REPEAT end; a
    function block's input keeps its value and the block is called.

  The flag stays until the platform clears it (`code` to 0, `line` to
  0); `plant_init` clears it.
- A faulted statement changes nothing: its value is computed and checked
  before it is stored. While it is set, every task function returns at once: the
  program is stopped until `plant_init` or `plant_warm`.
- What the platform does then - stop the controller, show the error,
  restart - is the platform's.

## 6. The symbol table

The platform needs to find variables by name: to watch them, write
them, keep the RETAIN ones. The output carries its own table, in C,
described in `iecf.h` (built):

```c
extern const iecf_symbol_t plant_symbols[];   /* plant_symbol_count */
extern const iecf_type_t plant_types[];       /* plant_type_count */
```

- **A symbol for each top-level variable**: a global (`G`; `R.G` for
  resource R's), and each variable of each program instance (`R.P.X` for
  X of instance P on resource R) - the paths VAR_CONFIG uses (Ed.3
  6.5.6), upper-cased. Each gives its type, its offset in the context,
  and flags: `IECF_SYM_CONSTANT`. A located variable gives its area
  (`'I'`, `'Q'`, `'M'`), byte and bit instead.
- **Each type once**: an elementary type by its `IECF_T_*`; an
  enumeration by its base and its values; a structure or function block
  - the user's and the standard ones - by its members, each with its
  type, offset and flags (`IECF_SYM_INPUT`, `IECF_SYM_OUTPUT`); an array
  by its element type and each dimension's bounds. A VAR_IN_OUT, a
  pointer set at each call, is not a member.

A platform finds `R.P.PTS[2].X` by walking them: the symbol `R.P.PTS`,
the array's element type and its bounds, the structure's member `X`.
Beckhoff's ADS symbols and CODESYS's symbol configuration are built the
same way, and for the same reason: an array of 10000 is one entry. The
offsets and sizes are the C compiler's own, through `offsetof` and
`sizeof`, so the table is right for whatever compiler and target built
it - which a table iecf computed itself could not promise for portable
C. VAR_TEMP and a FUNCTION's variables live only during a call, and are
not in it. Tasks (name, interval, priority) and POU names are tables
beside it.

A platform that wants the table outside the binary - in a file, in a
custom section - reads it from there; `iecf` is to write it as JSON too,
for the tools that only need the names and types (not yet).

## 7. RETAIN

`VAR RETAIN` and `VAR_GLOBAL RETAIN` variables are gathered into one
region of the context, `ctx->retain`, a structure of its own,
`plant_retain_t` (built). The platform keeps those bytes across a
restart (a file, FRAM, flash): `offsetof(plant_t, retain)` and
`sizeof(plant_retain_t)` are where and how many.

- `plant_init` is a cold start: every variable to its initial value,
  the RETAIN ones too.
- `plant_warm` is a warm start (Ed.3 6.5.6): every variable to its
  initial value but the RETAIN ones, which keep what the region holds.
  A variable that is neither RETAIN nor NON_RETAIN is set anew, as on
  CODESYS, TwinCAT and Siemens.
- A program's RETAIN variables are a structure `P_<NAME>_retain_t` in
  the region, one per instance, which the instance reaches by its
  `retain` pointer; a function block instance declared RETAIN is kept
  whole.
- The symbol table marks each `IECF_SYM_RETAIN` and gives its offset in
  the context, so a platform can also move them by name across a change
  of program.

Not yet: RETAIN inside a FUNCTION_BLOCK's own declarations (each
instance would need its part of the region), a RETAIN variable at an
address, and RETAIN on a program instance in the CONFIGURATION.

## 8. Representations

As `docs/dependencies.md`: BOOL is one byte, 0 or 1 (a packed bit only in
the process image); the integers and bit strings their exact widths;
REAL and LREAL IEEE 754 binary32 and binary64 (checked when `iecf.h` is
compiled); TIME signed 32-bit milliseconds, LTIME signed 64-bit
nanoseconds; DATE and DT unsigned 32-bit seconds since 1970, TOD
unsigned 32-bit milliseconds since midnight, the long ones signed 64-bit
nanoseconds; STRING(n) n+1 bytes, 0-terminated, Latin-1; WSTRING(n) n+1
UTF-16 units. An enumerated type is held as its base integer, DINT when
it names none, its values numbered from 0 in their order unless the
declaration gives them (Ed.3 6.4.4.3); a subrange as its base type,
clamped where it is stored (§5).

## 9. Not yet

Hierarchical addresses, preemptive tasks (Ed.4 §6.9), object orientation
(Ed.3 classes, methods, interfaces), USTRING (Ed.4), online change (a new
program taking over a running one's state by the symbol table) - each in
its step of PROPOSAL.md §5.
