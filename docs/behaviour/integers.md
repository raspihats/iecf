# Integers and bit strings

The integer types are two's complement (signed) or plain binary
(unsigned), of the widths Ed.3 Table 10 gives them:

| Type | Range | Type | Range |
|---|---|---|---|
| SINT | -128 .. 127 | USINT | 0 .. 255 |
| INT | -32768 .. 32767 | UINT | 0 .. 65535 |
| DINT | -2³¹ .. 2³¹-1 | UDINT | 0 .. 2³²-1 |
| LINT | -2⁶³ .. 2⁶³-1 | ULINT | 0 .. 2⁶⁴-1 |

BYTE, WORD, DWORD and LWORD are 8, 16, 32 and 64 bits; BOOL is one bit,
held in a byte as 0 or 1.

## Overflow wraps, and is flagged

A result outside its type wraps, two's complement for a signed type,
modulo 2ⁿ for an unsigned one, and is flagged `OVERFLOW`. The task goes
on. No vendor stops a task on an overflow; Siemens and Rockwell set an
error bit, as iecf flags ([Implementer choices](../dependencies.md#an-integer-overflow-wraps-and-is-flagged-ed3-732-table-23)).

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `INT#32767 + INT#1` | `INT` | `-32768` | `flag OVERFLOW` | |
| `INT#-32768 - INT#1` | `INT` | `32767` | `flag OVERFLOW` | |
| `SINT#100 * SINT#2` | `SINT` | `-56` | `flag OVERFLOW` | 200 - 256 |
| `UINT#0 - UINT#1` | `UINT` | `65535` | `flag OVERFLOW` | |
| `USINT#200 + USINT#100` | `USINT` | `44` | `flag OVERFLOW` | 300 - 256 |
| `LINT#9223372036854775807 + LINT#1` | `LINT` | `-9223372036854775808` | `flag OVERFLOW` | |
| `ULINT#18446744073709551615 + ULINT#1` | `ULINT` | `0` | `flag OVERFLOW` | |
| `ABS(INT#-32768)` | `INT` | `-32768` | `flag OVERFLOW` | 32768 is no INT |
| `ABS(INT#-5)` | `INT` | `5` | — | |
| `INT#30000 + INT#2767` | `INT` | `32767` | — | the edge itself is no overflow |

## Division

Integer division goes toward zero, as C's and every vendor's does. MOD
gives the remainder of that division, with the sign of the dividend
(Ed.3 Table 29: `IN1 - (IN1 / IN2) * IN2`). Division by zero is a
**fault**: no vendor agrees on a value to go on with. MOD by zero is 0,
as Ed.3 Table 29, note e, has it.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `DINT#7 / DINT#2` | `DINT` | `3` | — | |
| `DINT#-7 / DINT#2` | `DINT` | `-3` | — | toward zero |
| `DINT#7 / DINT#0` | `DINT` | — | `fault DIVISION_BY_ZERO` | |
| `UDINT#7 / UDINT#0` | `UDINT` | — | `fault DIVISION_BY_ZERO` | |
| `INT#-32768 / INT#-1` | `INT` | `-32768` | `flag OVERFLOW` | 32768 is no INT |
| `DINT#7 MOD DINT#3` | `DINT` | `1` | — | |
| `DINT#-7 MOD DINT#3` | `DINT` | `-1` | — | the dividend's sign |
| `DINT#7 MOD DINT#-3` | `DINT` | `1` | — | |
| `DINT#7 MOD DINT#0` | `DINT` | `0` | — | Ed.3 Table 29 e |

## Selection

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `MAX(INT#3, INT#-4, INT#2)` | `INT` | `3` | — | |
| `MIN(UINT#3, UINT#7)` | `UINT` | `3` | — | |
| `LIMIT(INT#0, INT#150, INT#100)` | `INT` | `100` | — | MN, IN, MX |
| `LIMIT(INT#0, INT#-5, INT#100)` | `INT` | `0` | — | |
| `SEL(FALSE, INT#1, INT#2)` | `INT` | `1` | — | G = FALSE takes IN0 |
| `MUX(INT#1, INT#10, INT#20, INT#30)` | `INT` | `20` | — | K counts from 0 |
| `MUX(INT#3, INT#10, INT#20, INT#30)` | `INT` | — | `fault MUX` | no IN3 |

SEL and MUX are functions: every input is evaluated, also the ones not
selected, as any function's inputs are.

**MOVE, SEL and MUX select arrays, structures, instances, interfaces
and references too** (Ed.3 Table 32, footnote d: "allowed, but not
required"): `SEL(g, a, b)[2]`, `MUX(k, p1, p2).x`, `i := SEL(g, o1,
o2)`, `SEL(g, r1, r2)^ := 30`. The result is the input selected, where
it is: nothing is copied until it is stored, and an element or field of
it is read in place. Every input is computed first, in order, its
indices checked; MUX's K outside its inputs faults as for numbers.
`NULL` goes beside an interface or a reference, and an instance beside
an interface its type implements: `MUX(k, o1, NULL, i)` is of the
interface. MIN, MAX and LIMIT stay ANY_ELEMENTARY, as Table 32 has
them. The harness case `selectdata` runs each.

## Partial access: a bit, byte, word of a bit string

A BYTE, WORD, DWORD or LWORD has parts (Ed.3 6.6.1.3, Table 17),
numbered from the least significant: `w.3` or `w.%X3` a bit, a BOOL;
`w.%B1` a byte, `dw.%W0` a word, `l.%D1` a double word. A part is read,
and written - into its whole, the rest of it kept - wherever the
variable is: a plain one, an element, a field, one at an address,
through a reference, an output taken into it. A part outside the
variable is E0014 (`b.%X8` of a BYTE, `w.%B2` of a WORD), and so is a
part of an INT, or of a direct variable (`%MB0.%X1`: name it, `m AT
%MB0`). The harness case `partaccess` runs each.

```
VAR w : WORD := 16#1234; b : BYTE; END_VAR
b := w.%B1;          (* 16#12 *)
w.%B0 := 16#FF;      (* w is 16#12FF *)
w.0 := FALSE;        (* w is 16#12FE *)
```

## Comparisons of more than two

GT, GE, EQ, LE and LT take any number of inputs (Ed.3 Table 33): the
result holds when each neighbouring pair does, `GT(a, b, c)` being
`a > b AND b > c`. NE takes two. Every input is computed once and in
order, a middle one too, though it is in two pairs: a FUNCTION called
there is called once (the harness case `extcompare`).

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `GT(INT#5, INT#2, INT#1)` | `BOOL` | `TRUE` | — | 5 > 2 > 1 |
| `GT(INT#5, INT#6, INT#1)` | `BOOL` | `FALSE` | — | |
| `GE(INT#5, INT#5, INT#5, INT#1)` | `BOOL` | `TRUE` | — | |
| `LT(INT#1, INT#2, INT#2)` | `BOOL` | `FALSE` | — | 2 < 2 does not hold |
| `LE(INT#1, INT#2, INT#2)` | `BOOL` | `TRUE` | — | |
| `EQ('ab', 'ab', 'ab')` | `BOOL` | `TRUE` | — | strings too |

## Bit strings: logic and shifts

AND, OR, XOR and NOT work bit by bit. SHL and SHR shift in zeros; a
count of the width or more leaves 0. ROL and ROR rotate by the count
modulo the width. A negative count is a **fault**: the standard has no
meaning for it. A BOOL is a bit string of one bit (Ed.3 Table 33's
ANY_BIT): shifted by 1 or more it is FALSE, rotated it is itself.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `WORD#16#F0F0 AND WORD#16#FF00` | `WORD` | `16#F000` | — | |
| `NOT BYTE#16#0F` | `BYTE` | `16#F0` | — | |
| `BYTE#16#0F XOR BYTE#16#FF` | `BYTE` | `16#F0` | — | |
| `SHL(BYTE#16#81, 1)` | `BYTE` | `16#02` | — | the top bit falls out |
| `SHL(BYTE#1, 8)` | `BYTE` | `16#00` | — | the width or more: 0 |
| `SHR(WORD#16#8000, 15)` | `WORD` | `16#0001` | — | zeros in, also on top |
| `ROL(BYTE#16#81, 1)` | `BYTE` | `16#03` | — | |
| `ROR(BYTE#16#01, 9)` | `BYTE` | `16#80` | — | 9 is 1, modulo 8 |
| `SHL(BYTE#1, -1)` | `BYTE` | — | `fault NEGATIVE` | |
| `SHL(TRUE, 1)` | `BOOL` | `FALSE` | — | one bit: it falls out |
| `ROL(TRUE, 5)` | `BOOL` | `TRUE` | — | |
