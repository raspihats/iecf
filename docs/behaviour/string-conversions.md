# Text conversions

`<T>_TO_STRING`, `TO_STRING` and `STRING_TO_<T>` write a value as text
and read it back. The standard leaves the text to the implementer; iecf
writes what CODESYS, TwinCAT and Siemens write where they document it,
and reads what they read ([Implementer choices](../dependencies.md#conversions-to-and-from-string)).
Neither direction uses the C library: the digits come from iecf's own
arithmetic, the same on every target.

## To STRING

- **BOOL**: `TRUE` or `FALSE`.
- **Integers and bit strings**: decimal, a `-` for a negative number and
  no `+`.
- **REAL and LREAL**: C's `%g` with 7 and 15 significant digits,
  correctly rounded, trailing zeros dropped — `inf`, `-inf` and `nan`
  for those.
- **TIME and LTIME**: the literal, with the units that are not 0, in
  lower case.
- **DATE, TOD, DT**: the literal; a TOD shows milliseconds only when
  they are not 0.
- **LDATE, LTOD, LDT**: the literal, prefixed `LD#`, `LTOD#`, `LDT#`; the
  fraction of a second in groups of three digits, as many as are not 0
  (`.500`, `.000001`).

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `BOOL_TO_STRING(TRUE)` | `STRING` | `'TRUE'` | — | |
| `INT_TO_STRING(INT#-123)` | `STRING` | `'-123'` | — | |
| `UDINT_TO_STRING(UDINT#4294967295)` | `STRING` | `'4294967295'` | — | |
| `BYTE_TO_STRING(BYTE#16#FF)` | `STRING` | `'255'` | — | decimal, not 16#FF |
| `REAL_TO_STRING(REAL#2.5)` | `STRING` | `'2.5'` | — | |
| `REAL_TO_STRING(REAL#1.0)` | `STRING` | `'1'` | — | trailing zeros dropped |
| `REAL_TO_STRING(REAL#0.1)` | `STRING` | `'0.1'` | — | 7 digits: not 0.100000001 |
| `REAL_TO_STRING(REAL#16777216.0)` | `STRING` | `'1.677722e+07'` | — | |
| `REAL_TO_STRING(REAL#0.00001)` | `STRING` | `'1e-05'` | — | |
| `LREAL_TO_STRING(LREAL#0.1)` | `STRING` | `'0.1'` | — | 15 digits |
| `REAL_TO_STRING(REAL#1.0E38 * REAL#10.0)` | `STRING` | `'inf'` | `flag OVERFLOW` | the multiplication's flag |
| `TIME_TO_STRING(T#1d2h3m4s500ms)` | `STRING` | `'T#1d2h3m4s500ms'` | — | |
| `TIME_TO_STRING(T#0s)` | `STRING` | `'T#0ms'` | — | |
| `TIME_TO_STRING(T#-1.5s)` | `STRING` | `'T#-1s500ms'` | — | |
| `LTIME_TO_STRING(LTIME#1s5us)` | `STRING` | `'LTIME#1s5us'` | — | |
| `DATE_TO_STRING(D#2026-11-07)` | `STRING` | `'D#2026-11-07'` | — | |
| `TOD_TO_STRING(TOD#12:34:56)` | `STRING` | `'TOD#12:34:56'` | — | |
| `TOD_TO_STRING(TOD#12:34:56.007)` | `STRING` | `'TOD#12:34:56.007'` | — | |
| `DT_TO_STRING(DT#2026-11-07-12:34:56)` | `STRING` | `'DT#2026-11-07-12:34:56'` | — | |
| `LDATE_TO_STRING(LD#1969-12-31)` | `STRING` | `'LD#1969-12-31'` | — | |
| `LTOD_TO_STRING(LTOD#12:34:56.5)` | `STRING` | `'LTOD#12:34:56.500'` | — | |
| `LDT_TO_STRING(LDT#2026-11-08-12:34:56.000000001)` | `STRING` | `'LDT#2026-11-08-12:34:56.000000001'` | — | |

## From STRING

The text is read as CODESYS and TwinCAT read it: spaces before it are
skipped; then comes a literal of the type — a type prefix (`INT#5`), a
sign, a based integer (`16#FF`, `2#1111_0000`), underscores between
digits, a REAL's fraction and exponent, a duration's `T#` (any case),
`TRUE` or `FALSE` in any case, or `1` or `0`; **what follows is
ignored**: `'123abc'` is 123. A duration or date needs its prefix. A
TIME keeps whole milliseconds and a DT whole seconds, the rest dropped.
A REAL is read exactly and rounded once, half to even.

**No literal, or a value the type cannot hold, gives 0** (for a DATE or
DT, 1970-01-01) and is flagged `CONVERSION`, as Siemens gives 0 and
ENO = FALSE. The task goes on.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `STRING_TO_INT(' 42')` | `INT` | `42` | — | spaces before are skipped |
| `STRING_TO_INT('INT#-5')` | `INT` | `-5` | — | a type prefix |
| `STRING_TO_INT('16#FF')` | `INT` | `255` | — | |
| `STRING_TO_DWORD('2#1111_0000')` | `DWORD` | `16#F0` | — | |
| `STRING_TO_INT('123abc')` | `INT` | `123` | — | the rest is ignored |
| `STRING_TO_INT('abc')` | `INT` | `0` | `flag CONVERSION` | no literal |
| `STRING_TO_INT('')` | `INT` | `0` | `flag CONVERSION` | |
| `STRING_TO_LDT('LDT#1677-09-21-00:12:43.145224192')` | `LDT` | `LDT#1677-09-21-00:12:43.145224192` | — | the first LDT |
| `STRING_TO_LDATE('LD#1677-09-21')` | `LDATE` | `LD#1970-01-01` | `flag CONVERSION` | no midnight LDATE holds |
| `STRING_TO_SINT('200')` | `SINT` | `0` | `flag CONVERSION` | not a SINT |
| `STRING_TO_BOOL('true')` | `BOOL` | `TRUE` | — | any case |
| `STRING_TO_BOOL('1')` | `BOOL` | `TRUE` | — | |
| `STRING_TO_BOOL('yes')` | `BOOL` | `FALSE` | `flag CONVERSION` | |
| `STRING_TO_REAL('2.5e3')` | `REAL` | `2500.0` | — | |
| `STRING_TO_REAL('1e39')` | `REAL` | `0.0` | `flag CONVERSION` | beyond REAL |
| `STRING_TO_TIME('T#1m30s')` | `TIME` | `T#1m30s` | — | |
| `STRING_TO_TIME('t#1.5s')` | `TIME` | `T#1s500ms` | — | |
| `STRING_TO_TIME('T#1.0005s')` | `TIME` | `T#1s` | — | below a millisecond dropped |
| `STRING_TO_TIME('90')` | `TIME` | `T#0s` | `flag CONVERSION` | no `T#` |
| `STRING_TO_DATE('D#2024-02-29')` | `DATE` | `D#2024-02-29` | — | |
| `STRING_TO_DATE('D#2023-02-29')` | `DATE` | `D#1970-01-01` | `flag CONVERSION` | no such day |
| `STRING_TO_TOD('TOD#24:00:00')` | `TOD` | `TOD#00:00:00` | `flag CONVERSION` | |
| `STRING_TO_DT('DT#2026-11-07-12:34:56.9')` | `DT` | `DT#2026-11-07-12:34:56` | — | whole seconds |
