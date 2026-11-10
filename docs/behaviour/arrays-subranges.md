# Arrays, subranges and enumerations

## An index outside its array faults

Every index that is not a constant is checked where it is used. One
outside the array's bounds is a **fault** `INDEX`: carrying on would read
or write memory that is not the array's. A constant index outside the
bounds is refused when the program is built.

The C reads element 0 on its way out, so no memory outside the array is
ever touched, even for the one statement that faults. Rockwell's Logix
also makes an out-of-range subscript a major fault; CODESYS and TwinCAT
leave it unchecked unless the project adds CheckBounds, whose template
clamps the index instead.

## A value outside a subrange is clamped, and flagged

A value stored into a subrange variable becomes the bound it passed
(`INT (0..100)` given 150 holds 100) and is flagged `CONVERSION`. It
holds for every store: an assignment, an element or field of the type, a
function block's input, an output taken into one, and an in-out given
one: the function or block works on a copy, which is stored back -
clamped - when the call returns (the harness case `inoutcopy`). A FOR over a
subrange starts at its start clamped and stops at the bound it would
pass. A constant initial value outside the subrange is refused when the
program is built.

CODESYS and TwinCAT do the same when the project adds the
CheckRangeSigned and CheckRangeUnsigned functions, and nothing without
them; iecf always checks, and the platform sees it.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `INT#50` | `INT(0..100)` | `50` | — | |
| `INT#150` | `INT(0..100)` | `100` | `flag CONVERSION` | the bound it passed |
| `INT#-5` | `INT(0..100)` | `0` | `flag CONVERSION` | |
| `INT#100` | `INT(0..100)` | `100` | — | the bound itself is inside |

## Enumerations

An enumerated value is held as an integer, the values numbered from 0
in their order unless the declaration gives them (Ed.3 6.4.4.3), in a
DINT unless the declaration names a base type. Only the type's own
values can be stored in it; there is nothing to check at run time.

## Arrays and structures given to a FUNCTION

An array or a structure is also a FUNCTION's or method's input, output,
in-out or result: an input copied in as the function starts, so the
function never writes the caller's; a result returned whole, `xs :=
RAMP(start := 1)`, or given straight to another's input (the harness
case `fnparams`). An in-out of one is given a variable in memory, not
one at an address (E0009 for now).
