# EN and ENO

Any function call may pass `EN := c`, an enable input, and
`ENO => x` (or `NOT ENO => x`), which tells whether the call went well
(Ed.3 6.6.1.5). The standard leaves what happens in detail to the
implementer; iecf decided, 2026-11-07
([Implementer choices](../dependencies.md#en-and-eno-ed3-6615)):

- **EN = FALSE: the call does not happen.** Its inputs are not evaluated
  — a division by zero among them does not fault — the function is not
  called, ENO is FALSE, and **the statement the call is in is left
  undone**: an assignment keeps its target's value, as when a REAL does
  not fit an integer. Nothing is flagged. This is Ed.3's "external
  implementation" (its Example 2), and what CODESYS does with a box whose
  EN is FALSE.
- **ENO is FALSE when the function's own operation raised a flag**: an
  overflow, a conversion that does not fit, a string length or position
  outside its string. A flag raised while computing an argument is the
  argument's, not the function's, and leaves ENO TRUE. The flag still
  reaches `ctx->flag` as any flag does. This is Ed.3's rule 3, and what
  Siemens does.
- Otherwise ENO is TRUE. A fault stops the task, as without ENO.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `DIV(EN := TRUE, IN1 := INT#7, IN2 := INT#2, ENO => eno)` | `INT` | `3, ENO TRUE` | — | |
| `DIV(EN := FALSE, IN1 := INT#7, IN2 := INT#2, ENO => eno)` | `INT` | `unchanged, ENO FALSE` | — | the assignment is left undone |
| `DIV(EN := FALSE, IN1 := INT#7, IN2 := INT#0, ENO => eno)` | `INT` | `unchanged, ENO FALSE` | — | not called: no fault |
| `DIV(EN := TRUE, IN1 := DINT#7, IN2 := DINT#0, ENO => eno)` | `DINT` | — | `fault DIVISION_BY_ZERO` | a fault still stops the task |
| `ADD(IN1 := INT#32767, IN2 := INT#1, ENO => eno)` | `INT` | `-32768, ENO FALSE` | `flag OVERFLOW` | its own overflow |
| `ADD(IN1 := INT#32767 + INT#1, IN2 := INT#1, ENO => eno)` | `INT` | `-32767, ENO TRUE` | `flag OVERFLOW` | the argument's overflow, not ADD's |
| `ADD(IN1 := INT#1, IN2 := INT#1, NOT ENO => eno)` | `INT` | `2, ENO FALSE` | — | ENO negated |
| `REAL_TO_INT(IN := REAL#40000.0, ENO => eno)` | `INT` | `unchanged, ENO FALSE` | `flag CONVERSION` | |
| `LEFT(IN := 'abc', L := 5, ENO => eno)` | `STRING` | `'abc', ENO FALSE` | `flag STRING_POSITION` | |
| `CONCAT(EN := FALSE, IN1 := 'a', IN2 := 'b')` | `STRING` | `unchanged` | — | |
| `MAX(EN := TRUE, IN1 := INT#1, IN2 := INT#2)` | `INT` | `2` | — | EN without ENO |

The same holds for the project's own FUNCTIONs, those with effects too
- an in-out, a VAR_EXTERNAL written, an output taken: with EN = FALSE
the function is not called, so its in-outs and globals stay as they
were and its outputs are not stored; ENO is not cleared by a flag from
storing an output (into a subrange, clamped), only by one the function
raised. The harness case `fneffen` runs each, with values worked by
hand.

## On a function block call

`t(EN := c, IN := x, PT := T#1s, Q => q, ENO => ok);` follows the same
rules (decided 2026-11-08):

- **EN = FALSE: the block is not called.** Its inputs are neither
  evaluated nor set in the instance, its body does not run — a timer or
  counter stays as it was — its outputs are not taken (`q` keeps its
  value), and `ok` is FALSE. Nothing is flagged.
- **ENO is FALSE when a flag is raised while the block runs**, in its
  body or anything it calls; not one from computing its inputs or from
  storing its outputs.
- A block that declares `EN` or `ENO` itself gets them as plain
  parameters, and its body decides.

The harness's `fbenable` and `fbskip` cases run these. ENO taken into
anything but a BOOL variable is not compiled yet (E0009).

## On a SPLIT

```iec
SPLIT_DATE(EN := run, IN := days[i], YEAR => y, MONTH => m, DAY => d, ENO => ok);
```

- **EN = FALSE: nothing is split.** IN is not evaluated - `days[i]`
  with `i` out of bounds does not fault - every output keeps its value,
  and `ok` is FALSE.
- **ENO is FALSE when a part does not fit its output** - the year
  into a USINT - besides being wrapped and flagged `CONVERSION`. Unlike a
  block's output, a SPLIT's output has no type of its own: the call picks
  it, so the narrowing is the call's own work (decided 2026-11-08,
  florin; [dependencies](../dependencies.md)).

The harness's `splitenable` case runs these.

## Set in a POU's own body

Every FUNCTION and FUNCTION_BLOCK has ENO without declaring it: TRUE as
each run starts, its body may read it and set it (decided 2026-11-08).
The caller's ENO is FALSE when **the body set it FALSE, or a flag was
raised while it ran** — `ENO := TRUE` cannot hide an overflow.

```iec
FUNCTION half : INT
VAR_INPUT x : INT; END_VAR
half := x / 2;
ENO := x >= 0;          (* half(x := -4, ENO => ok): ok is FALSE *)
END_FUNCTION
```

A PROGRAM has no ENO. The harness's `enobody` case runs these.
