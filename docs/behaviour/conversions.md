# Conversions

`<FROM>_TO_<TO>(x)` and `TO_<TO>(x)` convert a value; `TRUNC` and its
named forms cut a real to an integer. Ed.3 6.6.2.5 gives three kinds,
and iecf keeps them apart:

- **between numbers** — integers and reals — the value is kept if the
  target holds it;
- **between bit strings**, or a bit string and a number, the bits are
  kept: a *binary transfer* (Ed.3 Table 25);
- **to and from STRING**, the value is written or read as text
  ([Text conversions](string-conversions.md)).

Implicit conversions (Ed.3 Figure 11) only widen: an INT goes where a
DINT or a REAL is wanted without a word, and keeps its value.

## Integer to integer

A value the target holds is kept. One it does not hold keeps its low
bits — what a two's complement machine gives — and is flagged
`CONVERSION`: Siemens's 2011 manual keeps "the least-significant bytes
of IN", and TwinCAT's `INT_TO_SINT(4223)` is 127.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `INT_TO_DINT(INT#-5)` | `DINT` | `-5` | — | |
| `INT_TO_SINT(INT#100)` | `SINT` | `100` | — | |
| `INT_TO_SINT(INT#200)` | `SINT` | `-56` | `flag CONVERSION` | 16#C8 |
| `INT_TO_SINT(INT#4223)` | `SINT` | `127` | `flag CONVERSION` | 16#107F: its low byte |
| `DINT_TO_UINT(DINT#-1)` | `UINT` | `65535` | `flag CONVERSION` | |
| `UINT_TO_INT(UINT#40000)` | `INT` | `-25536` | `flag CONVERSION` | |
| `ULINT_TO_LINT(ULINT#18446744073709551615)` | `LINT` | `-1` | `flag CONVERSION` | |

## Real to integer: rounded half to even

`REAL_TO_INT` and the others round to the nearest integer, a half to the
even one (Ed.3 6.6.2.5.3: "rounding ... to the nearest integer"; IEEE
754's default). `TRUNC_INT`, `REAL_TRUNC_INT` and the old `TRUNC` (which
gives DINT and warns, W0003) drop the fraction, toward zero.

A real the target cannot hold — out of its range, an infinity, a NaN —
has no low bits to keep, and iecf invents no value: the conversion is
flagged `CONVERSION` and its **statement is left undone**. An assignment
keeps its target's old value; an IF or CASE runs no branch; a FOR does
not run; a WHILE or REPEAT ends; a function block's input keeps its
value, and the block is called. Only the first flag of a scan is kept,
so a NaN made by SQRT shows as `NAN`, not as the conversion after it.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `REAL_TO_INT(REAL#2.6)` | `INT` | `3` | — | |
| `REAL_TO_INT(REAL#2.5)` | `INT` | `2` | — | half to even |
| `REAL_TO_INT(REAL#3.5)` | `INT` | `4` | — | half to even |
| `REAL_TO_INT(REAL#-2.5)` | `INT` | `-2` | — | |
| `REAL_TO_UINT(REAL#-0.4)` | `UINT` | `0` | — | rounds to 0, which UINT holds |
| `REAL_TO_INT(REAL#40000.0)` | `INT` | unchanged | `flag CONVERSION` | statement left undone |
| `REAL_TO_UINT(REAL#-1.0)` | `UINT` | unchanged | `flag CONVERSION` | |
| `LREAL_TO_DINT(SQRT(LREAL#-1.0))` | `DINT` | unchanged | `flag NAN` | SQRT's flag came first |
| `TRUNC_INT(REAL#-2.7)` | `INT` | `-2` | — | toward zero |
| `REAL_TRUNC_DINT(REAL#2.7)` | `DINT` | `2` | — | |

## Integer to real

The nearest real, half to even. Past 2²⁴ for a REAL, 2⁵³ for an LREAL,
not every integer is a real; the nearest is no error and is not flagged.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `DINT_TO_REAL(DINT#16777216)` | `REAL` | `16777216.0` | — | |
| `DINT_TO_REAL(DINT#16777217)` | `REAL` | `16777216.0` | — | the nearest REAL, half to even |
| `INT_TO_LREAL(INT#-3)` | `LREAL` | `-3.0` | — | |

## Bit strings: a binary transfer

To or from a bit string, the bits move as they are (Ed.3 Table 25): to
a narrower type its rightmost (low) bits, to a wider one zeros on the
left. A signed integer's bits are its two's complement. Nothing is
flagged: a bit string has no range to leave.

From BOOL, 1 or 0. **To BOOL only with an extension.** Ed.3
contradicts itself here: Figure 11, which the checker follows, defines
no conversion from BYTE, WORD, DWORD or LWORD to BOOL, while Table 25
lists `BYTE_TO_BOOL` and the others as "Binary transfer of the rightmost
bit into the target" — so `BYTE_TO_BOOL(2)` would be FALSE. CODESYS and
TwinCAT accept them and give TRUE for any value other than 0. iecf
refuses them (E0013) unless `--extension bit-string-to-bool` is on, and
then gives what CODESYS and TwinCAT give (decided, florin,
[extensions](../extensions.md)). Portable either way: `b.0` for the
rightmost bit, `b <> 0` for any.

<!-- checked extension=bit-string-to-bool -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `BYTE_TO_BOOL(BYTE#16#02)` | `BOOL` | `TRUE` | — | not the rightmost bit: any value but 0 |
| `BYTE_TO_BOOL(BYTE#0)` | `BOOL` | `FALSE` | — | |
| `WORD_TO_BOOL(WORD#16#8000)` | `BOOL` | `TRUE` | — | |
| `DWORD_TO_BOOL(DWORD#1)` | `BOOL` | `TRUE` | — | |
| `LWORD_TO_BOOL(LWORD#16#8000000000000000)` | `BOOL` | `TRUE` | — | |
| `TO_BOOL(BYTE#16#10)` | `BOOL` | `TRUE` | — | the overloaded form |

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `INT_TO_WORD(INT#-1)` | `WORD` | `16#FFFF` | — | |
| `WORD_TO_INT(WORD#16#8000)` | `INT` | `-32768` | — | |
| `DWORD_TO_BYTE(DWORD#16#12345678)` | `BYTE` | `16#78` | — | the rightmost byte |
| `BYTE_TO_DWORD(BYTE#16#FF)` | `DWORD` | `16#000000FF` | — | zeros on the left |
| `BOOL_TO_WORD(TRUE)` | `WORD` | `16#0001` | — | |
| `BYTE#2 <> 0` | `BOOL` | `TRUE` | — | any bit set |

Between a 32 or 64-bit string and REAL or LREAL the bits are the IEEE
754 encoding:

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `DWORD_TO_REAL(DWORD#16#3F800000)` | `REAL` | `1.0` | — | |
| `REAL_TO_DWORD(REAL#-2.0)` | `DWORD` | `16#C0000000` | — | |
| `LREAL_TO_LWORD(LREAL#1.0)` | `LWORD` | `16#3FF0000000000000` | — | |
