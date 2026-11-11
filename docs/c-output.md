# The C iecf emits, and what a platform supplies

PROPOSAL.md §2: iecf's output is portable C, as generic as C can be. This
is the contract between that C and the platform that compiles and runs
it. Step 2 (§5) builds to it. Where the standard leaves a choice, the
choice is made after checking CODESYS, Beckhoff TwinCAT and Siemens,
and recorded in `docs/dependencies.md`.

Status: DECIDED 2026-11-05 (florin), the address numbering and byte
order included. Built in slices from 2026-11-06 (PROPOSAL.md §5 step
2): what is built is said in each section.

## 1. The language of the output

- **ISO C99, nothing else.** No compiler extensions, no built-ins
  (`__builtin_*`), no attributes, no `#pragma`. A build with
  `-std=c99 -pedantic -Werror` and every warning on is clean.
- **Freestanding.** The output and `iecf.h` include only the headers a
  freestanding C99 implementation must have: `<stdint.h>`, `<stddef.h>`,
  `<float.h>`, `<limits.h>`. No libc is taken for granted: strings,
  conversions and time arithmetic are iecf's own C. The one exception is
  the mathematics of Table 28 (`SQRT`, `SIN`, `LN`, ...) and `EXPT`,
  which come from `<math.h>` unless the platform defines `IECF_SQRT`,
  `IECF_EXPT(x, y)` and the others first; a program that calls none of them includes no `<math.h>`.
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
| `plant.symbols.json` | the symbol table (§6) as JSON, for tools |

A C identifier is the IEC name upper-cased (IEC names ignore case) with a
prefix that keeps it clear of C keywords and of other kinds of names:
`P_` for a POU, `V_` for a variable, `T_` for a type. An enumerated
value is written as its number (§8).

An element of a namespace (Ed.3 6.9) is named by its whole path, its
parts joined by two underscores: `Standard.Timers.TON` is
`B_STANDARD__TIMERS__TON_t`. No IEC name has two underscores together
(Ed.3 6.1.2), so no other name can be the same. Where a name is text
for the platform - `<p>_pou_names`, the type names of the symbol table
(§6) - it is the path with dots, `STANDARD.TIMERS.TON`.

- A PROGRAM is a structure `P_<NAME>_t`, its instances fields of the
  context, and two functions, `P_<NAME>_init` and `P_<NAME>` (built).
  Its inputs and outputs are fields too: a connection in the
  CONFIGURATION (`PROGRAM a WITH fast : counter(inc := 1, total => sum)`,
  Ed.3 6.8.2) sets each input from a literal, a global or an address
  before the instance runs, and writes each output to a global or an
  address after it, in the task's function. A VAR_IN_OUT is a pointer
  field, `counter(n := g)` setting it to the global `g` before each run,
  the body reading and writing `g` through it; every in-out must be
  connected (Ed.3 6.6.1.4, E0016), and only to a global that is not at
  an address: the process image is read byte by byte (§4), so no C
  pointer of the variable's type may point into it (built).
- A FUNCTION is `static <result> F_<NAME>(plant_t *ctx, <inputs>...)`,
  `void` for one without a result (Ed.3 6.6.2.1, built):
  every input passed, the caller giving an input's initial value where
  the call does not name it; its variables are C locals, initialised at
  each call, as a function has no memory (Ed.3 6.6.2.2). One whose body
  uses ENO without declaring it takes `IEC_BOOL *eno` after `ctx`, sets
  it to 1 as it starts and writes the body's ENO through it (built).
  Its outputs follow the inputs as pointers, `IEC_INT *O_Q`: the body
  works on a local of each, its initial value at each call, and writes
  it through the pointer as it returns - at its end or a RETURN, not on
  a fault; the caller passes NULL for an output it does not take. A call
  that takes outputs is written on a line of its own before its
  statement, into a temporary, its outputs stored after the check that
  stops the statement if it faulted (built). An in-out is a pointer
  among the inputs, in their declared order, `IEC_INT *V_V`, the body
  reading and writing `(*V_V)`; the caller passes the variable's
  address, an element by a computed index found on a line of its own
  first (built). A variable at an address, which no C pointer of its
  type may point at (§4), or of a subrange, which the function could
  write outside it, is passed as the address of a copy, `IEC_INT q3 =
  ...;`, stored back after the call as an output is: clamped into the
  subrange, and not on a fault (built). A VAR_EXTERNAL is the global itself, `ctx->V_G`, as in
  any POU (built).
- A FUNCTION's or method's input, output, in-out or result of a STRING,
  array or structure type is a pointer, which the function reaches
  through a cast to its exact C type, `(*(char (*)[7])R_)`: an input a
  `const void *P_<NAME>`, copied into the local `V_<NAME>` as the
  function starts (a STRING cut to its length and flagged, the flag
  noted at the input's line), NULL when the call does not give it, the
  local then its initial value; an in-out a `void *V_<NAME>`, the
  variable itself, in C memory and of its type exactly; an output a
  `void *O_<NAME>`, copied to whole as the function returns, then stored
  by the caller into its target, a STRING cut there. The result is a
  buffer of the calling statement's, `void *R_` after ENO, set to the
  type's initial value as the function starts; a STRING function also
  returns it, `static const char *F_<NAME>(...)`, so its call is a
  string expression, and an array's or structure's call is a line of
  its own, the buffer then copied (built).
- A statement whose expression calls a FUNCTION with effects - an
  output taken, an in-out, a VAR_EXTERNAL not CONSTANT, or a call of one
  that has - is written one operand to a line, `IEC_INT s5 = ...;`, left
  to right, each that can fault checked before the next runs: Ed.3 7.3.2
  evaluates the leftmost operand first, and C does not order a call's
  arguments or an operator's operands. Any other expression stays one C
  expression (built).
- A FUNCTION_BLOCK is a structure `B_<NAME>_t` - its inputs, outputs and
  variables as fields `V_<NAME>`, a VAR_IN_OUT a pointer set at each call
  to the variable passed, or to a copy of one at an address or of a
  subrange, stored back as a FUNCTION's is - and `B_<NAME>_init` and `B_<NAME>` (built).
  One whose body uses ENO without declaring it has a field `eno`, set to
  1 as each run starts (built). An input, or a PUBLIC variable, set from
  outside (`f.i := 3;`) is its field, which the next call reads (built).
- An instance's initial value, `t : TON := (PT := T#500ms)` (Ed.3
  6.6.3.3, 6.6.5.3), is its `_init` (`B_<NAME>_init`, or `iecf.h`'s
  `iecf_<TYPE>_init`) followed by a store to each input, output or
  PUBLIC variable it names, under the guard its type's own initial
  value of that variable has (§7). An instance is also an array's
  element, `B_<NAME>_t V_TS[50]`, or a structure's field (Table 11 rows
  5a to 7b); a structure that holds one is written after the block types
  (`T_<NAME>` follows the `B_` it holds). A call on one, `ts[i](...)`,
  or a method's, an interface or an in-out given one, takes its address
  first, `B_<NAME>_t *o5 = &(self->V_TS[...]);`, the index checked
  there, so one out of bounds stops the statement before anything runs.
  An element is an instance of exactly its type: no table is read. A
  VAR_GLOBAL instance is a field of the context, set at every start
  (built). Copying an array or a structure that holds an instance is
  not built, as an instance's copy is not.
- A CLASS (Ed.3 6.6.5) is a structure `B_<NAME>_t` and `B_<NAME>_init`,
  as a FUNCTION_BLOCK, with no body function: it has none. A method of
  a CLASS or FUNCTION_BLOCK is `M_<TYPE>__<METHOD>(ctx, B_<TYPE>_t
  *self, ...)`, its parameters as a FUNCTION's, its type's variables
  `self->V_<NAME>`; `inst.m(...)` and `THIS.m(...)` call it with
  `&inst` and `self` (OO.md step A; built). A method may write its
  instance, so its call is one of a statement's calls with effects: on a
  line of its own, checked for a fault before the next operand. The
  fault record names it `TYPE.METHOD`.
- A type that EXTENDS another (Ed.3 6.6.5.5, 6.6.7.2.7) starts its
  structure with its base's, `B_<BASE>_t base;`, so a pointer to it is
  a pointer to its base (C99 6.7.2.1); what it inherits is
  `self->base.V_X`, and its `_init` calls its base's first. The root of
  a hierarchy (a type with a base or a derived type) starts with `const
  iecf_otype_t *type`, which each `_init` sets to its own `TY_<NAME>`
  (`iecf.h`: its base's, its name, its table). `VT_<NAME>` is the table:
  a structure of function pointers, its base's first, then the body of
  a hierarchy's first FUNCTION_BLOCK, then each method a type declares
  that can be overridden (not PRIVATE, not FINAL), each taking the
  declaring type's `self`; a derived type's implementation is reached
  through a thunk `M_<TYPE>__<METHOD>__vt` (or `B_<TYPE>__vt` for a
  body) that converts it back (built).
- A call through THIS, a VAR_IN_OUT or a reference reads its slot from
  the instance's table, so it runs the method, or the block's body, of
  the instance's own type (Ed.3 6.6.5.6, 6.6.7.2.9 rule 4); `SUPER.m()`,
  `SUPER()`, a call on a named instance, and one of a method no derived
  type overrides call the function directly. A reference is checked
  before a method is called through it: NULL faults and the statement
  stops, the method not called (OO.md D8). A derived instance is given
  to a base-typed VAR_IN_OUT, of a block, FUNCTION or method, as the
  address of its base part (built).
- A variable of an INTERFACE (Ed.3 6.6.6) is an `iecf_itf_t` (`iecf.h`):
  `self`, the instance it references, and `table`, that instance's
  type's table for the interface; NULL when `self` is. The table's C
  type is `IT_<ITF>_t`, a function pointer `m_<METHOD>` for each method
  of the interface and of those it extends, each taking `void *self`. A
  type that implements an interface, itself or through a base, carries
  its type as a type in a hierarchy does, and has a table of its own
  for each, `I_<ITF>__<TYPE>`, of thunks `IM_<ITF>__<TYPE>__<METHOD>`
  to the method it has; its `TY_<NAME>` lists them (`itfs`, by the
  interface's `ITY_<ITF>`). A named instance given to an interface
  takes its table directly; THIS, an in-out, what a reference
  references and another interface find it in the instance's type
  (`iecf_itf_of`, `iecf_itf_as`). A method called through an interface
  is checked first, NULL faulting and stopping the statement (OO.md
  D8), then called through the slot. `a ?= b` is `iecf_itf_as` between
  interfaces, `iecf_itf_ref` into a `REF_TO` a type, and `iecf_ref_as`
  between references, each NULL when the instance is not of a type `a`
  holds (OO.md step C; built). An interface is an `iecf_itf_t` wherever
  it is: a VAR_GLOBAL, an array's element, a structure's field, a
  block's input or output; a FUNCTION's or method's output and result,
  and an input with an initial value, pass as a structure does (§2:
  `void *O_<NAME>`, the caller's `void *R_`, `const void *P_<NAME>`),
  a plain input by value. An initial value naming an instance is set
  in the `_init` once the POU's variables are all placed, as `REF()`'s.
  A PROGRAM's input or output of one is a field connected as an
  elementary one is: set before the run, stored after it (built).
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
- An `ARRAY[*]` in-out (Ed.3 6.5.3) is a pointer to an `iecf_flex_t`
  (iecf.h): `p` the caller's array, `b` each dimension's lower and
  upper bound in turn, `int64_t`. A call gives a compound literal of
  one, `&(iecf_flex_t){(void *)&(self->V_X), (const int64_t[]){INT64_C(0),
  INT64_C(4)}}`, which lives until the calling statement's block ends;
  one passed on is its own pointer. An element is `((IEC_INT
  (*))V_A->p)[flat]`, row-major, each index checked against `b`, a
  constant one too; `LOWER_BOUND(a, 1)` is `a->b[0]`, and with a
  computed dimension `iecf_bound`, a fault outside the array's
  dimensions. On an array of fixed bounds with a constant dimension it
  is a constant (built).
- A FUNCTION_BLOCK's in-out of an array, a structure or a STRING is a
  pointer field of the variable's own type, `IEC_INT (*V_F)[3]`, set by
  each call to the caller's variable; an `ARRAY[*]` one an
  `iecf_flex_t *` (built).

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
- **A variable takes its own type's width** from its address, whatever
  the size prefix (Ed.3 6.5.5.3; W0004 warns when they differ): `t AT
  %MD0 : TIME` is bytes 0 to 3, a signed count of milliseconds; an LTIME,
  LDATE, LTOD or LDT 8 bytes; `r AT %IB12 : REAL` bytes 12 to 15. A
  BOOL at a byte or wider takes that whole byte, 1 or 0, and reads TRUE
  for any value but 0 (docs/dependencies.md).
- **An array, structure or STRING at an address** lies from that byte,
  its parts back to back with no gap, each its own width, little-endian,
  a BOOL a byte, a STRING[n] n + 1 bytes: `p AT %MB0 : pt`, `pt` a
  BOOL, an INT and a TIME, is bytes 0 to 6, `p.t` bytes 3 to 6. It is
  read and written there part by part, never through a C object, so its
  layout does not depend on the C compiler.
- **A reference to a variable at an address** (`REF(x)`, Ed.3
  6.4.4.10) is its area, byte and bit, not a pointer: the platform may
  point the context at other image buffers between runs. Every
  reference is an `iecf_ref_t`, a C pointer for a variable in the
  context (`area` 0, NULL for NULL) or those three for one in the
  image; `r^` reads and writes through `iecf_rget_<T>` and
  `iecf_rset_<T>`, which look at where it is and treat the image as the
  variable itself does; a WSTRING through `iecf_wref_get`, its units in
  place in C memory or copied out of the image, and `iecf_wref_put`,
  which writes a stored one back. A reference to a type that cannot lie in the
  image (a function block, a structure holding a reference) is always a
  C pointer. Dereferencing NULL is a fault (§5); while its statement
  stops, the check gives `ctx->nowhere`, scratch memory as big as any
  type a reference reaches, so no C pointer is NULL when used.
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
  of range, a negative shift, an image too small, a NULL reference
  dereferenced: `IECF_FAULT_NULL`), `pou` which
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
  - an SFC step entered while still active, or an action's
    associations in conflict (Ed.3 6.7.5, 6.7.4.6 d): `IECF_FAULT_SFC`,
    the step staying active once (docs/behaviour/sfc.md);
  - a DATE or DT outside 1970-01-01 .. 2106-02-07-06:28:15, or a
    difference of two outside TIME: wrapped, `IECF_FAULT_OVERFLOW` (a
    TOD wraps at midnight, unflagged; docs/dependencies.md);
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
    function block's input keeps its value and the block is called;
  - a STRING too long for the variable it is stored in: cut to fit,
    `IECF_FAULT_STRING_LENGTH`. A function's result never is: its
    buffer holds the most its inputs can make;
  - text that is no literal of the type, or one the type cannot hold,
    converted from a STRING: 0, `IECF_FAULT_CONVERSION`;
  - a string function's length or position out of its string - negative,
    0 or past the end where the standard calls it an error (Ed.3
    6.6.2.5.11): what Siemens gives, an empty string or the part there
    is, `IECF_FAULT_STRING_POSITION` (docs/dependencies.md).

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
  and flags: `IECF_SYM_CONSTANT`, `IECF_SYM_RETAIN` when it is in the
  region (§7) - a function block instance kept whole there is too. A
  located variable gives its area
  (`'I'`, `'Q'`, `'M'`), byte and bit instead: the bit of a BOOL at a
  bit, `IECF_NO_BIT` for any other (a BOOL there takes the whole byte).
  An array, structure or STRING there has the flag `IECF_SYM_PACKED`:
  its parts are back to back from its byte (§4), not at the offsets of
  the type table, which are the C compiler's. A variable of an
  INTERFACE has no symbol: it is a reference to an instance, which has
  its own. Nor does an array of them, nor a structure's or block's
  field of one, which its type's members leave out.
- **Each type once**: an elementary type by its `IECF_T_*`, a
  `STRING[n]` as `IECF_T_STRING` of size n + 1, a `WSTRING[n]` as
  `IECF_T_WSTRING` of size 2(n + 1) (JSON kind `wstring`); an
  enumeration by its base and its values; a structure or function block
  - the user's and the standard ones - by its members, each with its
  type, offset and flags (`IECF_SYM_INPUT`, `IECF_SYM_OUTPUT`,
  `IECF_SYM_RETAIN` for a block's RETAIN variable); an array
  by its element type and each dimension's bounds; a reference,
  `IECF_KIND_REF`, by the type it references (`of`), its size an
  `iecf_ref_t`'s. A VAR_IN_OUT, a
  pointer set at each call, is not a member; nor is a program's a
  symbol: the global it is bound to is.

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

Beside the C, `iecf build` writes the same table as JSON,
`plant.symbols.json`, for the tools that need the names and types but
not the binary - an editor's watch list, a platform's online view:

```json
{
  "configuration": "plant",
  "symbols": [
    {"path": "R.P.PTS", "type": 0, "flags": []},
    {"path": "R.P.K", "type": 3, "flags": ["RETAIN"]},
    {"path": "R.P.LAMP", "type": 4, "flags": [], "area": "Q", "byte": 1, "bit": 2}
  ],
  "types": [
    {"name": "ARRAY [0..9] OF POINT", "kind": "array", "of": 1, "dims": [[0, 9]]},
    {"name": "POINT", "kind": "struct", "members": [{"name": "X", "type": 2, "flags": []}]},
    {"name": "INT", "kind": "elementary"},
    {"name": "DINT", "kind": "elementary"},
    {"name": "BOOL", "kind": "elementary"}
  ]
}
```

A symbol's and a member's `type` index `types`, in the order of
`plant_types`. A type's `kind` is `elementary` (its `name` the IEC
type), `string` (with its `length`), `enum` (its `base` type and its
`values`, name and number), `struct` and `block` (their `members`: name,
type, flags), `array` (`of` the element type, `dims` each dimension's
bounds), or `reference` (`of` the type it references). `flags` name `INPUT`, `OUTPUT`, `CONSTANT`, `RETAIN`, `PACKED`; a located
variable adds its `area` and `byte`, and a BOOL at a bit its `bit`. Offsets and sizes are not in
the JSON: they are the C compiler's, in the C table. The harness reads
this file to compare iecf's variables with matiec's (harness/README.md).

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
  `retain` pointer.
- A FUNCTION_BLOCK instance is kept whole, in the region, when it is
  declared RETAIN or its type keeps something - a RETAIN variable of
  its own, or an instance of a block that does. CODESYS does the same
  ("The entire instance of the function block with all of its data is
  located in the retain memory area. Only the declared retain variable
  is protected", CODESYS help, retain variables) (decided 2026-11-07,
  florin; a structure per instance with a pointer to it was the other
  way). `B_<NAME>_init(ctx, self, keep)` then says what a start sets
  anew, as Ed.3 6.5.6 does: `keep` 0 at a cold start, everything; 1 at
  a warm start, all but the block's RETAIN variables; 2 at a warm start
  of an instance declared RETAIN, only its NON_RETAIN ones. A block
  inside a block follows its own type and the section it is declared
  in, not the outer instance ("the retain declaration of the used
  function block type is applied", Ed.3 6.5.6.2).
- An array or a structure of instances is kept as its instances are:
  whole in the region when it is declared RETAIN or an instance in it
  keeps something; each instance's `_init` given the `keep` of its
  section, and, in a RETAIN one, its other parts set at a cold start
  only, a standard block's state with them. A VAR_GLOBAL instance, or
  an array or structure of them, is kept the same way.
- The symbol table marks each `IECF_SYM_RETAIN` and gives its offset in
  the context, so a platform can also move them by name across a change
  of program.

Not yet: a RETAIN variable at an address, and RETAIN on a program
instance in the CONFIGURATION.

## 8. Representations

As `docs/dependencies.md`: BOOL is one byte, 0 or 1 (a packed bit only in
the process image); the integers and bit strings their exact widths;
REAL and LREAL IEEE 754 binary32 and binary64 (checked when `iecf.h` is
compiled); TIME signed 32-bit milliseconds, LTIME signed 64-bit
nanoseconds; DATE and DT unsigned 32-bit seconds since 1970, TOD
unsigned 32-bit milliseconds since midnight, the long ones signed 64-bit
nanoseconds; STRING(n) n+1 bytes, 0-terminated, Latin-1; CHAR a byte,
a Latin-1 code (`IECF_T_CHAR` in the symbol table); WSTRING(n) n+1
UTF-16 units, `IEC_WCHAR[n + 1]`, its literals C99 compound literals
(`(const IEC_WCHAR[]){97u, 0u}`); WCHAR one unit (`IECF_T_WCHAR`). A
WSTRING at an address is its n + 1 units little-endian from its byte,
at any byte, read and written there byte by byte (`iecf_wimg_get`,
`iecf_wimg_put`), and through a reference by `iecf_wref_get` and
`iecf_wref_put`. A WCHAR at an
address is 16 bits, as a WORD. An enumerated type is held as its base integer, DINT when
it names none, its values numbered from 0 in their order unless the
declaration gives them (Ed.3 6.4.4.3); a subrange as its base type,
clamped where it is stored (§5). A reference is an `iecf_ref_t` (§4).

## 9. Not yet

EN and ENO on a function block's call, ENO set in a POU's own body, SFC
steps and actions in the symbol table, hierarchical addresses, preemptive tasks (Ed.4 §6.9),
USTRING (Ed.4), online change (a new
program taking over a running one's state by the symbol table) - each in
its step of PROPOSAL.md §5.
