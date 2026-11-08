# TIME and LTIME

A TIME is a signed 32-bit count of milliseconds, ±24d20h31m23s647ms; an
LTIME a signed 64-bit count of nanoseconds, ±292 years. Both may be
negative (Ed.3 Table 8: `T#-14ms`). A TIME literal must be whole
milliseconds; an LTIME literal whole nanoseconds.

## Adding and subtracting

Exact; a result outside the type wraps and is flagged `OVERFLOW`, as an
integer's does.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `T#1s + T#500ms` | `TIME` | `T#1s500ms` | — | |
| `T#1s - T#1500ms` | `TIME` | `T#-500ms` | — | negative durations exist |
| `T#24d20h31m23s647ms + T#1ms` | `TIME` | `T#-24d20h31m23s648ms` | `flag OVERFLOW` | |
| `LTIME#1s + LTIME#1ns` | `LTIME` | `LTIME#1s1ns` | — | |

## TIME and LTIME, one made the other

A TIME becomes an LTIME without a conversion function (Ed.3 Figure 11),
exactly. LTIME_TO_TIME drops what is below a millisecond, rounding down,
unflagged; an LTIME beyond TIME gives `T#0s`, flagged `CONVERSION`
(decided 2026-11-08, florin; [LDATE, LTOD and LDT](long-dates.md)).

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `T#-1500ms + LTIME#0s` | `LTIME` | `LTIME#-1500ms` | — | widened |
| `LTIME_TO_TIME(LTIME#1ms999us)` | `TIME` | `T#1ms` | — | rounding down |
| `LTIME_TO_TIME(LTIME#-1500us)` | `TIME` | `T#-2ms` | — | down, not toward zero |
| `LTIME_TO_TIME(LTIME#25d)` | `TIME` | `T#0s` | `flag CONVERSION` | beyond TIME |

## Times and divided by a number

Ed.3 Table 30 multiplies and divides a duration by any number (MUL_TIME,
DIV_TIME, or `*` and `/`); a number may also stand first in `*`. How the
result becomes whole milliseconds is the implementer's
([Implementer choices](../dependencies.md#a-duration-times-or-divided-by-a-number-ed3-table-30)):

- **by an integer**, exact; an overflow wraps and is flagged; a division
  goes toward zero, as DINT's;
- **by a REAL or LREAL**, computed in LREAL and **rounded to whole
  milliseconds, half to even**, as REAL_TO_DINT rounds; a result outside
  TIME, or infinite, or NaN is flagged `CONVERSION` and the statement
  left undone, as any real made an integer;
- **by zero**, 0 or 0.0, a fault.

Panasonic and PLCnext round the factor to a whole number first, so on
them `T#1s * 2.5` is 2 or 3 seconds; in iecf it is 2.5.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `T#1s * 3` | `TIME` | `T#3s` | — | |
| `3 * T#1s` | `TIME` | `T#3s` | — | either order |
| `T#10s / 3` | `TIME` | `T#3s333ms` | — | toward zero |
| `T#-7ms / 2` | `TIME` | `T#-3ms` | — | toward zero |
| `T#1s * REAL#2.5` | `TIME` | `T#2s500ms` | — | |
| `T#3ms * REAL#1.5` | `TIME` | `T#4ms` | — | 4.5: half to even |
| `T#5ms * REAL#1.5` | `TIME` | `T#8ms` | — | 7.5: half to even |
| `T#1s / REAL#3.0` | `TIME` | `T#333ms` | — | 333.33 |
| `T#1s * REAL#1.0E10` | `TIME` | unchanged | `flag CONVERSION` | beyond TIME: left undone |
| `T#1s / 0` | `TIME` | — | `fault DIVISION_BY_ZERO` | |
| `T#1s / REAL#0.0` | `TIME` | — | `fault DIVISION_BY_ZERO` | |
| `T#24d * 2` | `TIME` | `T#-1d17h2m47s296ms` | `flag OVERFLOW` | wrapped |
| `MUL_TIME(T#2s, 2)` | `TIME` | `T#4s` | — | |
| `DIV_TIME(T#1m, 4)` | `TIME` | `T#15s` | — | |
| `LTIME#1us * 1500` | `LTIME` | `LTIME#1ms500us` | — | |

matiec holds nanoseconds in a TIME, so its `T#10s / 3` is
3.333333333 s; iecf's TIME counts milliseconds, as every vendor's does.
