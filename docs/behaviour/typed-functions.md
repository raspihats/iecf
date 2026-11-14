# Typed functions

An overloaded standard function can be called *typed*: its name, `_`,
and a type, `ADD_INT` (Ed.3 6.6.2.4, Table 21). The typed function "is
performed using the type as data type for its inputs and outputs": each
input is made that type first, by an implicit conversion, which only
widens ([Conversions](conversions.md)), and the result is that type. So
`ADD_DINT(i, j)` of two INTs adds in DINT, where `i + j` adds in INT;
and `NOT_WORD(b)` of a BYTE widens it before it inverts. An input the
type does not take implicitly is E0013: `ADD_INT(d, i)` of a DINT.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `ADD_DINT(INT#30000, INT#30000)` | `DINT` | `60000` | — | added as DINTs |
| `ADD_INT(SINT#100, SINT#100, SINT#100)` | `INT` | `300` | — | |
| `MUL_INT(INT#300, INT#300)` | `INT` | `24464` | `flag OVERFLOW` | 90000 wrapped |
| `SUB_LINT(UDINT#0, UDINT#1)` | `LINT` | `-1` | — | |
| `DIV_REAL(INT#7, INT#2)` | `REAL` | `3.5` | — | |
| `MOD_DINT(DINT#7, INT#3)` | `DINT` | `1` | — | |
| `ABS_LINT(SINT#-128)` | `LINT` | `128` | — | |
| `NOT_WORD(BYTE#16#0F)` | `WORD` | `16#FFF0` | — | widened, then inverted |
| `SHL_WORD(BYTE#16#81, 1)` | `WORD` | `16#0102` | — | `SHL(BYTE#16#81, 1)` is `16#02` |
| `AND_DWORD(BYTE#16#FF, WORD#16#0F0F)` | `DWORD` | `16#0000000F` | — | |
| `MAX_LREAL(REAL#1.5, INT#2)` | `LREAL` | `2.0` | — | |
| `LIMIT_DINT(INT#0, DINT#-5, INT#10)` | `DINT` | `0` | — | |
| `SEL_LREAL(TRUE, INT#1, REAL#2.5)` | `LREAL` | `2.5` | — | |
| `MUX_UINT_DINT(UINT#1, INT#-5, DINT#7)` | `DINT` | `7` | — | K a UINT, the inputs DINT |
| `GT_DINT(INT#-1, DINT#0)` | `BOOL` | `FALSE` | — | |
| `EQ_LREAL(REAL#1.5, LREAL#1.5)` | `BOOL` | `TRUE` | — | |
| `SQRT_LREAL(REAL#2.25)` | `LREAL` | `1.5` | — | |
| `EXPT_REAL(REAL#2.0, 3)` | `REAL` | `8.0` | — | |
| `MOVE_LINT(SINT#-7)` | `LINT` | `-7` | — | |
| `CONCAT_STRING('ab', 'cd')` | `STRING` | `'abcd'` | — | |
| `LEFT_STRING('abc', 2)` | `STRING` | `'ab'` | — | |
| `FIND_STRING('abcd', 'cd')` | `INT` | `3` | — | |
| `MAX_CHAR(CHAR#'a', 'b')` | `CHAR` | `'b'` | — | `'b'` a CHAR here |

`ADD_TIME`, `SUB_TIME`, `ADD_LTIME` and `SUB_LTIME` are Table 30's
functions of durations, `+` and `-` of TIME or LTIME; `MUL_TIME` and
`DIV_TIME` are Table 30's too, a duration by a number
([TIME and LTIME](time.md)). `CONCAT_DATE` and the other Table 36 names
are those functions, not typed forms of CONCAT.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `ADD_TIME(T#1s, T#500ms)` | `TIME` | `T#1s500ms` | — | |
| `SUB_LTIME(LTIME#1s, T#1ms)` | `LTIME` | `LTIME#999ms` | — | the TIME widened |

## Which functions are typed

Table 21 asks the implementer for "an additional table showing which
functions are overloaded and which are typed". In iecf, every overloaded
standard function whose inputs share one type has a typed form, for
each type of its inputs' class:

| Function | Typed for |
|---|---|
| ABS | the integer and real types |
| SQRT, LN, LOG, EXP, SIN, COS, TAN, ASIN, ACOS, ATAN, ATAN2, EXPT | REAL, LREAL |
| ADD, SUB, MUL, DIV | the integer and real types, TIME, LTIME |
| MOD | the integer types |
| SHL, SHR, ROL, ROR, AND, OR, XOR, NOT | the bit strings and BOOL |
| MOVE, SEL, MAX, MIN, LIMIT, MUX, GT, GE, EQ, LE, LT, NE | every elementary type |
| LEFT, RIGHT, MID, INSERT, DELETE, REPLACE, FIND | STRING, WSTRING |
| CONCAT | STRING, WSTRING, CHAR, WCHAR |

`MUX_K_T` names K's type and the inputs' (Table 32, footnote c):
`MUX_UINT_REAL`. A function whose inputs are of no one type - LEN,
the conversions, the date and time functions - has none; a conversion
is typed by its own name already (`INT_TO_REAL`, `REAL_TRUNC_INT`).

## Inputs left out

A standard function called by name may leave inputs out; each is then
its type's default value, 0, FALSE, '', T#0s, D#1970-01-01 (Ed.3
6.6.1.4.2), the type the given inputs share. An extensible function has
as many inputs as the highest name given (6.6.2.5.1), at least as many
as in order; [E0016](../errors/E0016.md) where a call gives too few, or
gives a generic function nothing to take its type from, `ABS()`
([Implementer choices](../dependencies.md#a-standard-functions-inputs-left-out-ed3-66142-66251)).

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `LIMIT(IN := INT#7, MX := INT#5)` | `INT` | `5` | — | MN 0 (Table 19, NOTE 3) |
| `LIMIT(IN := INT#-2, MX := INT#5)` | `INT` | `0` | — | |
| `ADD(IN3 := INT#7)` | `INT` | `7` | — | IN1 and IN2 0 |
| `MUL(IN1 := INT#7, IN3 := INT#2)` | `INT` | `0` | — | IN2 0 |
| `MUX_INT(K := 3, IN0 := 1, IN2 := 2, IN4 := 3)` | `INT` | `0` | — | IN3 0 (6.6.2.5.1, EXAMPLE 2) |
| `SEL(G := TRUE, IN0 := INT#7)` | `INT` | `0` | — | IN1 0 |
| `GT(IN2 := INT#-2)` | `BOOL` | `TRUE` | — | 0 > -2 |
| `LEFT(IN := 'abc')` | `STRING` | `''` | — | L 0 |
| `DIV(IN1 := INT#7)` | `INT` | — | `fault DIVISION_BY_ZERO` | IN2 0 |
