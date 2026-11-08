# Behaviour reference

IEC 61131-3 says what most operations give and leaves the rest to the
implementer: what happens at the edges of a type, how text looks, what
an error does. This part of the book says, for each operation iecf
compiles, what it gives — the ordinary cases and every edge — and what
the program is told when something is not as it should be.

Each choice is recorded with the vendors it was checked against in
[Implementer choices](../dependencies.md). This part is the reference
for the results themselves.

## Three outcomes

An operation in iecf either gives its result, or gives a result and
**flags** it, or **faults**.

| | The task | The program sees | The platform sees |
|---|---|---|---|
| **Result** | goes on | the value | nothing |
| **Flag** | goes on | a value, as the page for the operation says | `ctx->flag`: the first flag's code, POU and line, until the platform clears it |
| **Fault** | stops | nothing more: the task returns at once | `ctx->fault`: its code, POU and line; no task runs until a restart |

A flag is for a result that is defined but probably not what the
program wanted: an overflow that wrapped, a string cut to fit, a REAL
that became infinite. The program carries on, as Siemens and Rockwell
carry on with an error bit set; the platform can show the first flag
to the user. A fault is for an operation that has no result to go on
with: a division by zero, an index outside its array.

Some flagged conversions have no value to give: a REAL too large for an
INT has no low bits to keep. Then the **statement is left undone**: an
assignment keeps its target's old value, an IF or CASE runs no branch,
a FOR does not run, a WHILE or REPEAT ends ([Conversions](conversions.md)).

## The codes

`ctx->flag.code` and `ctx->fault.code` hold an `IECF_FAULT_*` of
`runtime/iecf.h` ([The C output](../c-output.md) §5):

| Code | Name | Kind | When |
|---|---|---|---|
| 1 | `DIVISION_BY_ZERO` | fault | an integer, real or duration divided by zero |
| 2 | `OVERFLOW` | flag | an integer, TIME, DATE or DT result outside its type, wrapped; an infinite real |
| 3 | `INDEX` | fault | an array index outside its bounds |
| 4 | `CONVERSION` | flag | a conversion to a type that cannot hold the value; text that is no literal of the type; a subrange store outside its bounds |
| 5 | `MUX` | fault | MUX's K outside 0 .. n-1 |
| 6 | `NEGATIVE` | fault | a negative shift or rotate count |
| 7 | `STRING_LENGTH` | flag | a string cut to fit the variable it is stored in |
| 8 | `IMAGE` | fault | the platform's process image smaller than the program addresses |
| 9 | `NAN` | flag | a real result that is not a number |
| 10 | `STRING_POSITION` | flag | a string function's length or position outside its string |
| 11 | `SFC` | flag | an SFC step entered while still active; an action's associations in conflict ([SFC](sfc.md)) |

## Reading the tables

The examples in this part are checked: CI compiles each row with iecf,
runs it, and fails if the result or the flag is not what the row says
(`cargo run -p iecf-harness -- book`). Each row is the statement
`r := Expression;` with `r` of the row's Type. A table that needs an
extension says so above it, and its rows are built with that extension
on ([extensions](../extensions.md)).

- **Result** is the value `r` holds afterwards, as an IEC literal;
  `NaN`, `+inf` and `-inf` for those reals; *unchanged* when the
  statement was left undone; — when the operation faulted.
- **Flag** is `flag NAME` or `fault NAME`, raised by that statement, or
  — for neither.

A typed literal (`INT#32767`) is used where a plain one would be
computed by the checker: `32767 + 1` is a constant 32768, which an INT
cannot hold, and is refused before the program runs (E0020).
