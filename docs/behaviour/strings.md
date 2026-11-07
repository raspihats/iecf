# STRING

A `STRING` is `STRING[80]`; a `STRING[n]` holds up to n characters, one
byte each, read as Latin-1, followed by a 0 byte: `n + 1` bytes in all,
as on CODESYS and TwinCAT. n is at most 65535. A literal with a
character above U+00FF is refused (E0020): write it as a WSTRING. `$N`
is a line feed.

Strings compare by their characters' codes, left to right, a shorter
string as if extended with zeros (Ed.3 6.6.2.5.11): `'Z' > 'AZ'`, and
`'ab' < 'abc'`.

## Too long: cut, and flagged

A string stored where it does not fit is cut to fit and flagged
`STRING_LENGTH`. Every vendor cuts; iecf also says so. A function's
result is never cut on its way: its buffer holds the most its inputs can
make, so only the store at the end can cut.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `CONCAT('ab', 'cd')` | `STRING` | `'abcd'` | — | |
| `CONCAT('ab', 'cd')` | `STRING[3]` | `'abc'` | `flag STRING_LENGTH` | cut where it is stored |
| `CONCAT('ab', 'cd', 'ef')` | `STRING` | `'abcdef'` | — | CONCAT takes any number |
| `'Z' > 'AZ'` | `BOOL` | `TRUE` | — | |
| `'ab' < 'abc'` | `BOOL` | `TRUE` | — | |
| `LEN('')` | `INT` | `0` | — | |

## Lengths and positions outside the string

LEFT, RIGHT, MID, DELETE, INSERT and REPLACE take a length L and a
position P, counted from 1. The standard calls a negative L or P, or a
position past the end, an error and leaves what it does to the
implementer. iecf gives what Siemens documents for each case — the one
vendor that documents them all — and flags `STRING_POSITION`; the task
goes on. A length of 0 takes nothing and is no error; INSERT at P = 0
puts IN2 first, as CODESYS and Siemens do.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `LEFT('abc', 2)` | `STRING` | `'ab'` | — | |
| `LEFT('abc', 0)` | `STRING` | `''` | — | no error |
| `LEFT('abc', 5)` | `STRING` | `'abc'` | `flag STRING_POSITION` | L past the end: all of IN |
| `LEFT('abc', -1)` | `STRING` | `''` | `flag STRING_POSITION` | L < 0: empty |
| `RIGHT('abc', 2)` | `STRING` | `'bc'` | — | |
| `RIGHT('abc', 5)` | `STRING` | `'abc'` | `flag STRING_POSITION` | |
| `MID('abcde', 2, 2)` | `STRING` | `'bc'` | — | MID(IN, L, P) |
| `MID('abc', 5, 2)` | `STRING` | `'bc'` | `flag STRING_POSITION` | L past the end: from P to the end |
| `MID('abc', 2, 0)` | `STRING` | `''` | `flag STRING_POSITION` | P < 1: empty |
| `MID('abc', 1, 4)` | `STRING` | `''` | `flag STRING_POSITION` | P past the end: empty |
| `DELETE('abcde', 2, 2)` | `STRING` | `'ade'` | — | DELETE(IN, L, P) |
| `DELETE('abc', 2, 5)` | `STRING` | `'abc'` | `flag STRING_POSITION` | P past the end: IN unchanged |
| `DELETE('abcde', 9, 2)` | `STRING` | `'a'` | `flag STRING_POSITION` | L past the end: to the end |
| `INSERT('abc', 'XY', 2)` | `STRING` | `'abXYc'` | — | after the P-th character |
| `INSERT('abc', 'XY', 0)` | `STRING` | `'XYabc'` | — | P = 0: first |
| `INSERT('abc', 'XY', 5)` | `STRING` | `'abcXY'` | `flag STRING_POSITION` | P past the end: appended |
| `REPLACE('abcde', 'X', 2, 2)` | `STRING` | `'aXde'` | — | REPLACE(IN1, IN2, L, P) |
| `REPLACE('abc', 'X', 2, 9)` | `STRING` | `'abcX'` | `flag STRING_POSITION` | P past the end: appended |
| `FIND('abcbc', 'bc')` | `INT` | `2` | — | the first |
| `FIND('abc', 'x')` | `INT` | `0` | — | not there |
| `FIND('abc', '')` | `INT` | `0` | — | |

LEN and FIND give INT, as CODESYS, TwinCAT and Siemens do.
