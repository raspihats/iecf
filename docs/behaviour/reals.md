# REAL and LREAL

REAL is IEEE 754 binary32 (about 7 significant digits) and LREAL
binary64 (about 15), as Ed.3 Table 10 requires. Each operation rounds
its result to its own type, half to even, and keeps no extra precision
between operations.

## Infinity and NaN are flagged, division by zero faults

A result too large for its type becomes an infinity and is flagged
`OVERFLOW`; one that is not a number is flagged `NAN`. Both carry on as
IEEE 754 has them: a later operation on them gives what IEEE 754 says.
Division by zero — 0.0 or -0.0 — is a **fault**, as an integer's is.
([Implementer choices](../dependencies.md#an-infinite-or-nan-real-is-flagged-not-a-fault-ed3-table-10-e-f))

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `REAL#1.0E38 * REAL#10.0` | `REAL` | `+inf` | `flag OVERFLOW` | |
| `REAL#-1.0E38 * REAL#10.0` | `REAL` | `-inf` | `flag OVERFLOW` | |
| `REAL#1.0 / REAL#0.0` | `REAL` | — | `fault DIVISION_BY_ZERO` | not +inf |
| `SQRT(REAL#-1.0)` | `REAL` | `NaN` | `flag NAN` | |
| `LN(REAL#0.0)` | `REAL` | `-inf` | `flag OVERFLOW` | |
| `LREAL_TO_REAL(LREAL#1.0E300)` | `REAL` | `+inf` | `flag OVERFLOW` | beyond REAL |
| `EXPT(REAL#2.0, 10)` | `REAL` | `1024.0` | — | |
| `REAL#2.0 ** REAL#0.5` | `REAL` | `1.4142135` | — | |

## Precision

A REAL holds every integer up to 2²⁴ = 16777216 exactly, an LREAL up to
2⁵³; past that, a sum rounds to the nearest the type holds. Neither is
flagged: it is how floating point is, not an error.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `REAL#0.1 + REAL#0.2` | `REAL` | `0.3` | — | the nearest REAL to each is the same |
| `LREAL#0.1 + LREAL#0.2` | `LREAL` | `0.30000000000000004` | — | not LREAL's 0.3 |
| `REAL#16777216.0 + REAL#1.0` | `REAL` | `16777216.0` | — | 16777217 is no REAL |
| `REAL#1.0 / REAL#3.0` | `REAL` | `0.33333334` | — | |

The functions of Ed.3 Table 28 (SQRT, LN, LOG, EXP, SIN, COS, TAN,
ASIN, ACOS, ATAN, EXPT) come from the C library's `<math.h>`, or from
the platform's own when it gives one ([The C output](../c-output.md) §1):
their last digit is the library's.
