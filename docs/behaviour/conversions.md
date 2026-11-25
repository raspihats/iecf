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

Two more change how a value is written, not its type: **BCD**, an
integer as decimal digits in a bit string's nibbles, and **byte
order**, `TO_BIG_ENDIAN` and its kin ([below](#bcd)).

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

### Narrowing implicitly: only with an extension

Anything Figure 11 does not widen takes a conversion function
([E0013](../errors/E0013.md)). With `--extension implicit-narrowing`,
as CODESYS and TwinCAT have it, an integer or a bit string (not BOOL)
goes where another integer or bit string is wanted all the same,
converted as its `*_TO_*` would be, and [W0010](../errors/W0010.md)
says where: a number its target does not hold keeps its low bits,
flagged; to or from a bit string the bits move, unflagged. An integer
goes where a real is wanted, and an LREAL where a REAL is, rounded as
DINT_TO_REAL and LREAL_TO_REAL. A real to an integer, BOOL, durations
and dates keep Figure 11's rules
([extensions](../extensions.md#implicit-narrowing-an-integer-a-bit-string-or-an-lreal-where-another-number-is-wanted-converted-as-its-_to_-would-with-a-warning);
the harness case `narrowing`).

<!-- checked extension=implicit-narrowing -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `DINT#100000` | `INT` | `-31072` | `flag CONVERSION` | its low 16 bits, as DINT_TO_INT |
| `DINT#-5` | `INT` | `-5` | — | held: kept |
| `INT#-2` | `UINT` | `65534` | `flag CONVERSION` | |
| `INT#-2` | `WORD` | `16#FFFE` | — | the bits, unflagged |
| `DWORD#16#12345678` | `INT` | `22136` | — | the low 16 bits, 16#5678 |
| `ULINT#300` | `USINT` | `44` | `flag CONVERSION` | |
| `DINT#16777217` | `REAL` | `16777216.0` | — | the nearest REAL, as DINT_TO_REAL |
| `LREAL#1.5` | `REAL` | `1.5` | — | as LREAL_TO_REAL |

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
(`fs[REAL_TO_INT(x)](...)`), its in-outs. A conversion that fails in
one of them leaves the whole call undone: no input is set, the block is
not called, no output and no ENO is stored. Where its outputs and ENO
go is found after it runs, as a FUNCTION's are: with
`f(q => a[REAL_TO_INT(x)])` f is called and `q` not stored, nor any
other output, an in-out's copy or ENO.
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

### Between bit strings and reals by value: only with an extension

Figure 11 converts only DWORD and REAL, and LWORD and LREAL, as above:
by their bits. It defines no conversion between BYTE or WORD and a real,
nor between DWORD and LREAL or LWORD and REAL; those are
[E0013](../errors/E0013.md). With `--extension bit-string-arithmetic`
they convert **by value**, the bit string as the unsigned integer of its
width, as CODESYS and TwinCAT convert them: to a real exactly (or the
nearest, for a DWORD or LWORD past 2^24 or 2^53), from a real rounded
half to even, one outside the bit string flagged and its statement left
undone, as `REAL_TO_UDINT`.

**Careful with code written for CODESYS:** there `DWORD_TO_REAL` and
`REAL_TO_DWORD` convert by value too, where Ed.3 Table 25 moves the
bits; this extension leaves them Ed.3's. `--extension
codesys-real-conversions`, in the `codesys` pack, converts them as
CODESYS does, warning each ([W0011](../errors/W0011.md)):

<!-- checked extension=codesys-real-conversions -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `DWORD_TO_REAL(DWORD#100)` | `REAL` | `100.0` | — | by value, as UDINT_TO_REAL |
| `REAL_TO_DWORD(REAL#2.5)` | `DWORD` | `16#00000002` | — | half to even |
| `REAL_TO_DWORD(REAL#-1.0)` | `DWORD` | unchanged | `flag CONVERSION` | below a DWORD: left undone |
| `LWORD_TO_LREAL(LWORD#1000)` | `LREAL` | `1000.0` | — | the number, not the bits |
| `LREAL_TO_LWORD(LREAL#1.0)` | `LWORD` | `16#0000000000000001` | — | |

A program meant for every tool writes `UDINT_TO_REAL(DWORD_TO_UDINT(d))`
and `UDINT_TO_DWORD(REAL_TO_UDINT(r))`
([extensions](../extensions.md#codesys-real-conversions-dword_to_real-and-its-kin-by-value-as-codesys-converts-them)).

<!-- checked extension=bit-string-arithmetic -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `BYTE_TO_REAL(BYTE#200)` | `REAL` | `200.0` | — | by value |
| `WORD_TO_LREAL(WORD#16#FFFF)` | `LREAL` | `65535.0` | — | |
| `DWORD_TO_LREAL(DWORD#100)` | `LREAL` | `100.0` | — | by value: Figure 11 has none |
| `DWORD_TO_REAL(DWORD#100)` | `REAL` | `1.4E-43` | — | Ed.3's binary transfer, kept |
| `REAL_TO_BYTE(REAL#51.5)` | `BYTE` | `16#34` | — | 52, half to even |
| `LREAL_TO_WORD(LREAL#2.5)` | `WORD` | `16#0002` | — | |
| `REAL_TO_BYTE(REAL#300.0)` | `BYTE` | unchanged | `flag CONVERSION` | beyond BYTE: left undone |

## BCD

`<BITS>_BCD_TO_<INT>` reads a bit string's nibbles as decimal digits,
the lowest the units; `<INT>_TO_BCD_<BITS>` writes them (Ed.3 Table 22,
3a-4b). The bit string is a BYTE (two digits), WORD (four), DWORD
(eight) or LWORD (sixteen), the integer any; `BCD_TO_<INT>` and
`TO_BCD_<BITS>` are the overloaded forms. `IS_VALID_BCD` tells whether a
bit string is BCD (Ed.3 Table 39).

What Ed.3 leaves open is a conversion that does not fit, and is flagged
`CONVERSION` as one is ([Implementer choices](../dependencies.md#bcd-what-is-not-bcd-or-does-not-fit-ed3-table-22-d)):
a bit string that is not BCD, a nibble above 9, gives 0; a number with
more digits than the bit string holds, and a negative number (BCD has
no sign), give every nibble F (16#FF for a BYTE, 16#FFFF for a WORD),
which is not BCD, so `IS_VALID_BCD` tells it later and converting it
back gives 0, flagged. Ed.2's `INT_TO_BCD`, which names no bit string,
is E0010, naming Ed.3's `INT_TO_BCD_WORD`. A value beyond the integer type
keeps its low bits, as [any conversion](#integer-to-integer) does. With
`ENO =>`, each makes ENO FALSE and leaves the statement undone
([EN and ENO](en-eno.md)).

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `WORD_BCD_TO_UINT(WORD#16#0369)` | `UINT` | `369` | — | Ed.3's own example |
| `USINT_TO_BCD_BYTE(USINT#25)` | `BYTE` | `16#25` | — | Ed.3's own example |
| `BCD_TO_INT(WORD#16#9999)` | `INT` | `9999` | — | the overloaded form |
| `INT_TO_BCD_WORD(INT#1234)` | `WORD` | `16#1234` | — | |
| `LWORD_BCD_TO_ULINT(LWORD#16#9999999999999999)` | `ULINT` | `9999999999999999` | — | sixteen digits |
| `WORD_BCD_TO_UINT(WORD#16#12A4)` | `UINT` | `0` | `flag CONVERSION` | not BCD |
| `WORD_BCD_TO_UINT(IN := WORD#16#12A4, ENO => eno)` | `UINT` | `unchanged, ENO FALSE` | `flag CONVERSION` | |
| `UINT_TO_BCD_WORD(UINT#12345)` | `WORD` | `16#FFFF` | `flag CONVERSION` | five digits: every nibble F |
| `USINT_TO_BCD_BYTE(USINT#100)` | `BYTE` | `16#FF` | `flag CONVERSION` | three digits |
| `ULINT_TO_BCD_LWORD(ULINT#10000000000000000)` | `LWORD` | `16#FFFFFFFFFFFFFFFF` | `flag CONVERSION` | seventeen digits |
| `INT_TO_BCD_WORD(INT#-5)` | `WORD` | `16#FFFF` | `flag CONVERSION` | BCD has no sign |
| `WORD_BCD_TO_USINT(WORD#16#0300)` | `USINT` | `44` | `flag CONVERSION` | 300 is beyond USINT: its low bits |
| `IS_VALID_BCD(WORD#16#1234)` | `BOOL` | `TRUE` | — | |
| `IS_VALID_BCD(BYTE#16#9A)` | `BOOL` | `FALSE` | — | a nibble A |

## Byte order: TO_BIG_ENDIAN and its kin

`TO_BIG_ENDIAN(x)` gives the value whose bytes in memory are x's in
big-endian order, the leftmost byte first; `TO_LITTLE_ENDIAN` the same
in little-endian order; `FROM_BIG_ENDIAN` and `FROM_LITTLE_ENDIAN` read
such a value back (Ed.3 6.6.2.5.13, Table 37). The input and the result
are one type. The PLC's own order is the target's: little-endian on
every target iecf runs on now (x86, ARM, wasm), so there the BIG forms
reverse the bytes and the LITTLE forms give x as it is. The rows below
are for a little-endian target.

The types converted are Ed.3's list - ANY_INT and ANY_BIT of 16 bits or
more, REAL and LREAL, WCHAR, TIME. **Any other is given back as it is**
("Other data types are not converted"): BOOL, a BYTE, a CHAR, a STRING,
an enumerated value, a reference, and LTIME and the dates and times of
day. With `--extension endian-dates` those are converted too, as PLCnext
converts them: their bytes as iecf holds them, so a DT's reversed
seconds are another DT ([extensions](../extensions.md#endian-dates-to_big_endian-and-its-kin-reverse-ltime-and-the-dates-too),
[Implementer choices](../dependencies.md#byte-order-the-plcs-own-and-the-types-converted-ed3-662513)).
An array or a structure is converted element by element and field by
field, the rest of it as it is. A structure with relative locations
(Ed.3 6.4.4.7) keeps its fields little-endian on every host, so there
the BIG forms reverse each field's bytes whatever the host, and the
LITTLE forms leave them. A function block instance in a structure is
copied as it is, as an assignment copies it; of an instance itself the
functions are refused (E0014), an instance not being a data type's
value. A component at a `*` address gives its value, converted, and the
input's address is not written: `b := TO_BIG_ENDIAN(a)` puts the
converted values at b's addresses, a's I/O as it was.

An `ARRAY[*]` is converted where it is stored: `b := TO_BIG_ENDIAN(a)`,
b an `ARRAY[*]` too, copies a into b (a fault, INDEX, and nothing
copied when their sizes differ, as for any [whole
ARRAY[*]](../dependencies.md#array-bounds-and-array-ed3-6445-653)) and
converts b's elements there. Given on as a value, to a FUNCTION's
`ARRAY[*]` input, it is not compiled (E0009): that would need a copy of
a size only the run knows, and iecf keeps no storage the run sizes.
Store it into an `ARRAY[*]` first. An array of fixed bounds converts
anywhere, an `ARRAY[*]` input included.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `TO_BIG_ENDIAN(DWORD#16#12345678)` | `DWORD` | `16#78563412` | — | in memory: 12 34 56 78 |
| `TO_LITTLE_ENDIAN(DWORD#16#12345678)` | `DWORD` | `16#12345678` | — | already |
| `FROM_BIG_ENDIAN(DWORD#16#78563412)` | `DWORD` | `16#12345678` | — | |
| `TO_BIG_ENDIAN(LWORD#16#123456789ABCDEF0)` | `LWORD` | `16#F0DEBC9A78563412` | — | |
| `TO_BIG_ENDIAN(INT#1)` | `INT` | `256` | — | |
| `REAL_TO_DWORD(TO_BIG_ENDIAN(REAL#1.5))` | `DWORD` | `16#0000C03F` | — | 1.5 is 16#3FC00000 |
| `TO_BIG_ENDIAN(T#1ms)` | `TIME` | `T#4h39m37s216ms` | — | 16#01000000 ms |
| `TO_BIG_ENDIAN(BYTE#16#12)` | `BYTE` | `16#12` | — | one byte: as it is |
| `TO_BIG_ENDIAN(LTIME#1ns)` | `LTIME` | `LTIME#1ns` | — | not in Ed.3's list: as it is |
| `TO_BIG_ENDIAN(DT#1970-01-02-00:00:00)` | `DT` | `DT#1970-01-02-00:00:00` | — | as it is |

With `--extension endian-dates`:

<!-- checked extension=endian-dates -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `TO_BIG_ENDIAN(LTIME#1ns)` | `LTIME` | `LTIME#833d23h59m54s37ms927us936ns` | — | 2⁵⁶ ns |
| `TO_BIG_ENDIAN(DT#1970-01-02-00:00:00)` | `DT` | `DT#2038-03-21-13:52:00` | — | 86400 s, 16#00015180, reversed |
| `FROM_BIG_ENDIAN(TO_BIG_ENDIAN(LTOD#12:00:00))` | `LTOD` | `LTOD#12:00:00` | — | |
