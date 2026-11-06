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
  comparisons before the operation, and faults (§5) instead of
  happening.
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
`P_` for a POU, `V_` for a variable, `T_` for a type. Enumerated values
are `T_<type>__<value>`.

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
(PROPOSAL.md §6.3). iecf's handling: the task stops where it is.

- The generated code checks before every operation that could go wrong,
  and on a fault records it in the context and returns, from every
  function up to the task function. No `setjmp`, no `abort`, no signal.
- After a task function returns, the platform reads `ctx->fault`:
  ```c
  typedef struct { uint16_t code; uint16_t pou; uint32_t line; } iecf_fault_t;
  ```
  `code` says what (division by zero, overflow, index out of range, a
  conversion out of range, MUX's K out of range, a negative shift or
  string position, a string too long, an image too small), `pou` which
  POU (an index into `plant_pou_names`), `line` the source line of the
  statement.
- Not a fault, as Ed.3 defines it: `x MOD 0` is 0 (Table 29 e); SHL and
  SHR by the width or more give 0, ROL and ROR go round (Table 30).
- A faulted statement changes nothing: its value is computed and checked
  before it is stored. While it is set, every task function returns at once: the
  program is stopped until `plant_init` or `plant_warm`.
- What the platform does then - stop the controller, show the error,
  restart - is the platform's.

## 6. The symbol table

The platform needs to find variables by name: to watch them, write
them, keep the RETAIN ones. The output carries its own table, in C:

```c
extern const iecf_symbol_t plant_symbols[];
extern const uint32_t plant_symbol_count;
```

one entry per variable reachable by a path (`MAIN.PUMP.Q`, `G_SPEED`,
`MAIN.ARR[3]`): its path, its IEC type, its offset in the context
(`offsetof`), its size, and flags (RETAIN, CONSTANT, input, output). The
offsets are the C compiler's own, through `offsetof`, so the table is
right for whatever compiler and target built it - which a table iecf
computed itself could not promise for portable C. Tasks (name, interval,
priority, trigger) and POU names are tables beside it.

A platform that wants the table outside the binary - in a file, in a
custom section - reads it from there; `iecf` can also write it as JSON
for the tools that only need the names and types.

## 7. RETAIN

`VAR RETAIN` variables, wherever they are declared, are gathered into
one region of the context, `ctx->retain`, a structure of its own. The
platform keeps those bytes across a restart (a file, FRAM, flash) and
`plant_warm` leaves them as they are. The symbol table marks each, so a
platform can also move them by name across a change of program.

## 8. Representations

As `docs/dependencies.md`: BOOL is one byte, 0 or 1 (a packed bit only in
the process image); the integers and bit strings their exact widths;
REAL and LREAL IEEE 754 binary32 and binary64 (checked when `iecf.h` is
compiled); TIME signed 32-bit milliseconds, LTIME signed 64-bit
nanoseconds; DATE and DT unsigned 32-bit seconds since 1970, TOD
unsigned 32-bit milliseconds since midnight, the long ones signed 64-bit
nanoseconds; STRING(n) n+1 bytes, 0-terminated, Latin-1; WSTRING(n) n+1
UTF-16 units.

## 9. Not yet

Hierarchical addresses, preemptive tasks (Ed.4 §6.9), object orientation
(Ed.3 classes, methods, interfaces), USTRING (Ed.4), online change (a new
program taking over a running one's state by the symbol table) - each in
its step of PROPOSAL.md §5.
