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

## CHAR

A CHAR holds one Latin-1 character, its code a byte (Ed.3 Table 10);
it starts as `'$00'`. `CHAR#'X'` and `CHAR#16#58` are CHAR literals; an
untyped `'X'` of one character is a CHAR where a CHAR is wanted - given
to a CHAR, or compared with one (Ed.3 Table 6, 1b) - and a STRING
elsewhere. A CHAR goes where a STRING goes, as a string of that one
character (Figure 11: implicit), and `'$00'` there is the empty string,
as `'$00'` ends any string.

`CHAR_TO_BYTE`, `BYTE_TO_CHAR` and `CHAR_TO_WORD` and wider are binary
transfers (Table 25), `CHAR_TO_USINT` the code (Figure 11).
`STRING_TO_CHAR` takes the first character (Table 27); of an empty
string, for which Ed.3 leaves the result undefined, it gives `'$00'`
and is flagged `CONVERSION`, as any value its target cannot hold
([dependencies](../dependencies.md)).

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `CHAR#'a'` | `CHAR` | `'a'` | — | |
| `CHAR#16#41` | `CHAR` | `'A'` | — | |
| `BYTE_TO_CHAR(BYTE#16#42)` | `CHAR` | `'B'` | — | |
| `CHAR_TO_BYTE(CHAR#'a')` | `BYTE` | `16#61` | — | |
| `CHAR_TO_USINT('a')` | `USINT` | `97` | — | `'a'` a CHAR here |
| `CHAR_TO_WORD(CHAR#'$FF')` | `WORD` | `16#00FF` | — | |
| `CHAR#'a' < CHAR#'b'` | `BOOL` | `TRUE` | — | by code |
| `CHAR#'a' = 'a'` | `BOOL` | `TRUE` | — | |
| `MAX(CHAR#'a', CHAR#'b')` | `CHAR` | `'b'` | — | |
| `STRING_TO_CHAR('xyz')` | `CHAR` | `'x'` | — | the first character |
| `STRING_TO_CHAR('')` | `CHAR` | `'$00'` | `flag CONVERSION` | undefined in Ed.3 |
| `CHAR_TO_STRING(CHAR#'q')` | `STRING` | `'q'` | — | |
| `CONCAT('ab', CHAR#'c')` | `STRING` | `'abc'` | — | |
| `CHAR_TO_STRING(CHAR#'$00')` | `STRING` | `''` | — | the empty string |

## One character: `s[i]`

`s[i]` is the i-th character of a STRING, counted from 1, a CHAR (Ed.3
6.4.2); it reads and it is written: `c := s[2];`, `s[3] := 'B';`,
`a[1][2]` in an array of strings. Ed.3 gives the functions it is
equivalent to, `STRING_TO_CHAR(MID(s, 1, i))` and `REPLACE(s, c, 1, i)`,
and leaves a position outside the string open:

- a read outside the string gives `'$00'`, flagged `STRING_POSITION`,
  as that MID and STRING_TO_CHAR give;
- a write changes only a character the string has: below 1 or past its
  end, `s` is left as it is, flagged `STRING_POSITION` - it neither
  grows nor empties. `c` = `'$00'` takes the character out, as
  `REPLACE(s, '', 1, i)` does.

A position that is a constant outside `1..n` of a `STRING[n]` is an
error (E0014). `s[i]` is not a variable: no `REF()` of it (E0026), and it
is no in-out's (E0015). A STRING is not stored in a CHAR, so `s[2] :=
t` for a STRING `t` is E0013 (Ed.3's error cases); a CHAR goes into a
STRING. CODESYS and TwinCAT count from 0 and give a BYTE, not Ed.3's;
Siemens counts from 1 and gives a CHAR, as iecf. None documents a
position outside the string.

In these rows `s` is a `STRING[10]` holding `'ABCD'`:

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `s[2]` | `CHAR` | `'B'` | — | from 1 |
| `s[4]` | `CHAR` | `'D'` | — | the last |
| `s[INT#2 * 2 - 1]` | `CHAR` | `'C'` | — | computed |
| `s[5]` | `CHAR` | `'$00'` | `flag STRING_POSITION` | past the end, inside `STRING[10]` |
| `s[LEN(s) - 4]` | `CHAR` | `'$00'` | `flag STRING_POSITION` | 0 |
| `s[1] = 'A'` | `BOOL` | `TRUE` | — | |
| `CONCAT(s[4], s[1])` | `STRING` | `'DA'` | — | a CHAR as a string |

The harness case `stringchars` writes characters, past the end and
below 1 too (`s` unchanged, flagged), with values worked by hand.
