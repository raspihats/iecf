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

## Bit strings: logic and shifts

AND, OR, XOR and NOT work bit by bit. SHL and SHR shift in zeros; a
count of the width or more leaves 0. ROL and ROR rotate by the count
modulo the width. A negative count is a **fault**: the standard has no
meaning for it.

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
