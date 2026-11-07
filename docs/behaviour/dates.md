# DATE, TOD and DT

| Type | Held as | Range |
|---|---|---|
| DATE | seconds since 1970-01-01, at midnight; unsigned 32-bit | `D#1970-01-01` .. `D#2106-02-07` |
| TOD | milliseconds since midnight; unsigned 32-bit | `TOD#00:00:00` .. `TOD#23:59:59.999` |
| DT | seconds since 1970-01-01 00:00; unsigned 32-bit | `DT#1970-01-01-00:00:00` .. `DT#2106-02-07-06:28:15` |

CODESYS and TwinCAT hold them the same way. The default initial value of
a DATE and a DT is 1970-01-01: Ed.3 Table 10 leaves it to an implementer
whose calendar does not start at 0001-01-01. A literal that names no day
or time, or one iecf cannot hold — before 1970, a TOD finer than a
millisecond, a DT with a fraction of a second — is refused (E0020).

## At the edges

([Implementer choices](../dependencies.md#date-tod-and-dt-at-their-edges))

- **A TOD wraps at midnight**, unflagged: a time of day is a clock.
- **A DATE or DT outside its range wraps and is flagged** `OVERFLOW`, as
  an integer does; so does a difference outside TIME. A TIME holds
  ±24 days and 20 hours: the difference of two dates 25 days apart is
  already too much for it.
- **A DT holds whole seconds**: milliseconds that come to one are
  dropped, rounding down, unflagged.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `TOD#23:00:00 + T#2h` | `TOD` | `TOD#01:00:00` | — | wraps at midnight |
| `TOD#01:00:00 - T#2h` | `TOD` | `TOD#23:00:00` | — | |
| `TOD#01:00:00 - TOD#23:00:00` | `TIME` | `T#-22h` | — | |
| `DT#2026-11-07-22:00:00 + T#7h` | `DT` | `DT#2026-11-08-05:00:00` | — | |
| `DT#2026-11-07-12:00:00 + T#1500ms` | `DT` | `DT#2026-11-07-12:00:01` | — | ms dropped, rounding down |
| `DT#2026-11-07-12:00:00 - T#1500ms` | `DT` | `DT#2026-11-07-11:59:58` | — | down, not toward zero |
| `DT#1970-01-01-00:00:00 - T#1s` | `DT` | `DT#2106-02-07-06:28:15` | `flag OVERFLOW` | wrapped |
| `D#2026-12-01 - D#2026-11-07` | `TIME` | `T#24d` | — | |
| `D#2026-12-07 - D#2026-11-07` | `TIME` | `T#-19d17h2m47s296ms` | `flag OVERFLOW` | 30 days: beyond TIME |
| `SUB_DT_DT(DT#2026-11-07-12:00:00, DT#2026-11-07-11:00:00)` | `TIME` | `T#1h` | — | |
| `DT_TO_TOD(DT#2026-11-07-12:34:56)` | `TOD` | `TOD#12:34:56` | — | |
| `DT_TO_DATE(DT#2026-11-07-12:34:56)` | `DATE` | `D#2026-11-07` | — | |
| `CONCAT_DATE_TOD(D#2026-11-07, TOD#12:34:56.789)` | `DT` | `DT#2026-11-07-12:34:56` | — | ms dropped |
| `DAY_OF_WEEK(D#2026-11-07)` | `USINT` | `6` | — | 0 is Sunday (Ed.3 Table 36) |
| `DAY_OF_WEEK(D#1970-01-01)` | `USINT` | `4` | — | a Thursday |
| `MAX(D#2026-11-07, D#2025-01-01)` | `DATE` | `D#2026-11-07` | — | |
