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

## Implicit conversions: they widen

Ed.3 6.6.1.6 applies Figure 11's implicit conversions in an assignment,
to a function's or block's input, to a block's output taken with `=>`,
and to the operands of an operator or overloaded function, so that both
are one type: the narrower one is widened to the other's. Never to an
in-out (rule 6): a VAR_IN_OUT takes a variable of its own type.

An operation is done in the type of its operands, and its result is
widened only where it is used (Ed.3 6.6.1.6, Example 3): `INT * INT`
is an INT, and overflows as one, even into a DINT. CODESYS and TwinCAT
compute in their registers' width instead, at least 32 bits, so the
same product there is 60000; iecf follows the standard, and flags it.
Make one operand wide, `INT_TO_DINT(i) * j`, for the product in DINT.

An unsigned type widens to the signed type twice its width, keeping its
value: UINT#40000 is 40000 as a DINT, not -25536. Two integers neither
of which widens to the other are both widened to the narrowest signed
type that holds them both, as Siemens SCL does: SINT and USINT to INT,
INT and UINT to DINT, DINT and UDINT to LINT. ULINT and LINT have none,
and are refused (E0014): convert one.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `DINT#-3 + UINT#40000` | `DINT` | `39997` | — | the UINT widened |
| `UINT#40000 > DINT#-3` | `BOOL` | `TRUE` | — | |
| `SINT#-1` | `LINT` | `-1` | — | the sign kept |
| `USINT#255` | `INT` | `255` | — | |
| `BYTE#16#FF OR WORD#16#0100` | `WORD` | `16#01FF` | — | a bit string: no sign |
| `INT#3 / REAL#2.0` | `REAL` | `1.5` | — | Ed.3 Example 6 |
| `MAX(SINT#-1, DINT#-3)` | `DINT` | `-1` | — | |
| `INT#30000 * DINT#2` | `DINT` | `60000` | — | |
| `INT#30000 * INT#2` | `DINT` | `-5536` | `flag OVERFLOW` | multiplied as INT, then widened |
| `INT#30000 + UINT#40000` | `DINT` | `70000` | — | both widened to DINT |
| `SINT#-1 + USINT#255` | `INT` | `254` | — | both to INT |
| `DINT#-1 + UDINT#4000000000` | `LINT` | `3999999999` | — | both to LINT |
| `INT#-1 < UINT#40000` | `BOOL` | `TRUE` | — | compared as DINTs |
| `MAX(INT#-1, UINT#40000)` | `DINT` | `40000` | — | |

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

Undone, the statement **faults on nothing it computes after**: where the
conversion's place in it would divide by zero or index outside an array
(`10 / REAL_TO_INT(x)`, `a[REAL_TO_INT(x)] := 1`), the statement is
flagged and left undone, and the task carries on. A fault inside a
FUNCTION the statement calls still stops it, as does a fault in a
statement whose conversions all succeeded. An assignment's target may
be indexed by a conversion, or by a FUNCTION with effects, computed
before the value (Ed.3 7.3.2); the harness case `targetidx` runs these.

A **call after the conversion is not made**, and nothing the statement
would store is stored - not into the element the placeholder would
find. A FUNCTION with effects (an output taken, an in-out, a global
written) is called only if every conversion before it succeeded: one in
an operand before it, in one of its inputs, in the index of its in-out
(`Inc(a[REAL_TO_INT(x)])`). A FUNCTION's output goes where it is taken
after the call, so with `F(o => a[REAL_TO_INT(x)])` F is called and `o`
not stored; a call made before the conversion has been made, its
outputs stored. SPLIT_DATE and its kin store no part.

A function block's places are found before it runs - its instance
(`fs[REAL_TO_INT(x)](...)`), its in-outs, where its outputs and ENO go.
A conversion that fails in one of them leaves the whole call undone: no
input is set, the block is not called, no output and no ENO is stored.
(A conversion that fails in an *input's value* leaves that input alone,
and the block is called, as above.) The same holds where a whole array,
structure or STRING is read through such an index
(`row := aa[REAL_TO_INT(x)]`), where an instance found by one has its
input set (`fs[REAL_TO_INT(x)].d := 1`), and in `REF(a[REAL_TO_INT(x)])`:
the store is left undone. The harness case `undonecalls` runs these.

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
| `INT#10 / REAL_TO_INT(REAL#40000.0)` | `INT` | unchanged | `flag CONVERSION` | no division by zero |
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
