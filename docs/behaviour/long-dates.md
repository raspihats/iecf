# LDATE, LTOD and LDT

| Type | Held as | Range |
|---|---|---|
| LDATE | nanoseconds since 1970-01-01, at midnight; signed 64-bit | `LD#1677-09-22` .. `LD#2262-04-11` |
| LTOD | nanoseconds since midnight; signed 64-bit | `LTOD#00:00:00` .. `LTOD#23:59:59.999999999` |
| LDT | nanoseconds since 1970-01-01 00:00; signed 64-bit | `LDT#1677-09-21-00:12:43.145224192` .. `LDT#2262-04-11-23:47:16.854775807` |

Ed.3 Table 10 makes each a signed 64-bit count of nanoseconds; the range
is the whole of it, as CODESYS, TwinCAT and PLCnext give it (decided
2026-11-08, florin; [implementer choices](../dependencies.md#ldate-ltod-and-ldt)).
A date before 1970 is a negative count. A literal takes up to nine
digits of a second; one that names no day or time, or that the type
cannot hold, is refused (E0020).

## At the edges

- **An LTOD wraps at midnight**, unflagged, as a TOD does.
- **An LDT outside its range wraps and is flagged** `OVERFLOW`, as a DT
  does; so does a difference outside LTIME.
- **A long value made a short one drops what the short type is too coarse
  for, rounding down**, unflagged: LDT_TO_DT the fraction of a second,
  LTOD_TO_TOD and LTIME_TO_TIME what is below a millisecond. Before 1970
  down is earlier: LDT_TO_LDATE of the last nanosecond of 1969-12-31 is
  1969-12-31.
- **A long value the short type cannot hold gives the short type's
  default value**, flagged `CONVERSION`: an LDT before 1970 or after
  2106-02-07 made a DT or DATE is 1970-01-01, an LTIME beyond
  ±24d20h31m23s647ms made a TIME is `T#0s`. The task goes on. Ed.3
  Table 22 leaves this result to the implementer.
- DT, DATE, TOD and TIME widen to the long types without a conversion
  function (Ed.3 Figure 11), exactly. With one long operand the other
  is widened (Ed.3 6.6.1.6, rule 7): `DT + LTIME` is an LDT, never a
  DT, which could not hold the LTIME's range.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `LTOD#23:00:00 + LTIME#2h` | `LTOD` | `LTOD#01:00:00` | — | wraps at midnight |
| `LTOD#01:00:00 - LTIME#2h1ns` | `LTOD` | `LTOD#22:59:59.999999999` | — | |
| `LTOD#01:00:00 - LTOD#23:00:00` | `LTIME` | `LTIME#-22h` | — | |
| `LDT#2026-11-08-22:00:00 + LTIME#7h` | `LDT` | `LDT#2026-11-09-05:00:00` | — | |
| `LDT#1970-01-01-00:00:00 - LTIME#1ns` | `LDT` | `LDT#1969-12-31-23:59:59.999999999` | — | before 1970 |
| `LDT#2262-04-11-23:47:16.854775807 + LTIME#1ns` | `LDT` | `LDT#1677-09-21-00:12:43.145224192` | `flag OVERFLOW` | wrapped |
| `LDT#2026-11-08-12:00:00 + T#1500ms` | `LDT` | `LDT#2026-11-08-12:00:01.5` | — | a TIME widened |
| `DT#2026-11-08-12:00:00 + LTIME#1500ms` | `LDT` | `LDT#2026-11-08-12:00:01.5` | — | the DT widened |
| `LDT#2026-11-08-12:00:01.5 - DT#2026-11-08-12:00:00` | `LTIME` | `LTIME#1500ms` | — | |
| `TOD#10:00:00 + LTIME#1500ms` | `LTOD` | `LTOD#10:00:01.5` | — | |
| `LD#2026-11-09 - D#2026-11-08` | `LTIME` | `LTIME#1d` | — | |
| `LD#2026-12-08 - LD#2026-11-08` | `LTIME` | `LTIME#30d` | — | |
| `SUB_LDT_LDT(LDT#2262-04-11-00:00:00, LDT#1677-09-22-00:00:00)` | `LTIME` | `LTIME#-1d23h34m33s709ms551us616ns` | `flag OVERFLOW` | beyond LTIME: wrapped |
| `DT_TO_LDT(DT#2106-02-07-06:28:15)` | `LDT` | `LDT#2106-02-07-06:28:15` | — | |
| `LDT_TO_DT(LDT#2026-11-08-12:00:00.999999999)` | `DT` | `DT#2026-11-08-12:00:00` | — | rounding down |
| `LDT_TO_DT(LDT#1969-12-31-23:59:59)` | `DT` | `DT#1970-01-01-00:00:00` | `flag CONVERSION` | before DT's range |
| `LDT_TO_DATE(LDT#2106-02-08-00:00:00)` | `DATE` | `D#1970-01-01` | `flag CONVERSION` | after it |
| `LDATE_TO_DATE(LD#2026-11-08)` | `DATE` | `D#2026-11-08` | — | |
| `LDT_TO_LDATE(LDT#1969-12-31-23:59:59.999999999)` | `LDATE` | `LD#1969-12-31` | — | down is earlier |
| `LDT_TO_LTOD(LDT#1969-12-31-23:59:59.999999999)` | `LTOD` | `LTOD#23:59:59.999999999` | — | |
| `LDT_TO_TOD(LDT#2026-11-08-12:34:56.789999999)` | `TOD` | `TOD#12:34:56.789` | — | |
| `LTOD_TO_TOD(LTOD#23:59:59.999999999)` | `TOD` | `TOD#23:59:59.999` | — | |
| `DT_TO_LTOD(DT#2026-11-08-12:34:56)` | `LTOD` | `LTOD#12:34:56` | — | |
| `CONCAT_DATE_LTOD(D#2026-11-08, LTOD#12:34:56.000000001)` | `LDT` | `LDT#2026-11-08-12:34:56.000000001` | — | |
| `CONCAT_LDT(1677, 9, 21, 0, 12, 43, 146)` | `LDT` | `LDT#1677-09-21-00:12:43.146` | — | |
| `CONCAT_LDT(1677, 9, 21, 0, 12, 43, 145)` | `LDT` | `LDT#1970-01-01-00:00:00` | `flag CONVERSION` | before the range |
| `CONCAT_LTOD(23, 59, 59, 999)` | `LTOD` | `LTOD#23:59:59.999` | — | |
| `MAX(LD#1969-12-31, LD#1677-09-22)` | `LDATE` | `LD#1969-12-31` | — | |

SPLIT_LTOD and SPLIT_LDT give milliseconds, the rest dropped rounding
down, as SPLIT_DT's do ([DATE, TOD and DT](dates.md)); the harness's
`ldates` case runs them, with the other conversions, tick by tick.
