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

The same holds for the project's own FUNCTIONs. A function block
called with EN or ENO, and `ENO` set inside a POU's own body, are not
compiled yet (E0009).
