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

## Bounds are constants

An array's bounds, a STRING's length and a subrange's limits are fixed
when the program is built (Ed.3 6.4.4.5.1): literals, `+ - * / MOD`, and
the names of constants (Ed.3 6.6.1.2.4), [E0032](../errors/E0032.md)
for anything else. A constant is a POU's own `VAR CONSTANT`, or a
`VAR_GLOBAL CONSTANT` through its `VAR_EXTERNAL CONSTANT`; a `TYPE`
sees the global constants directly, as a CODESYS GVL's `VAR_GLOBAL
CONSTANT` are (CODESYS refuses a plain `VAR_GLOBAL` there, C0161):

```
TYPE ROW : ARRAY [0..W] OF SINT; END_TYPE     (* W a VAR_GLOBAL CONSTANT *)
PROGRAM main
    VAR CONSTANT N : INT := 5; END_VAR
    VAR
        a : ARRAY [1..N] OF INT;
        s : STRING[N];
        lim : INT (0..N) := N;                (* an initial value too *)
    END_VAR
END_PROGRAM
```

The bounds are within DINT, as CODESYS has them, since LOWER_BOUND and
UPPER_BOUND give a DINT; the lower one is not above the upper one.

## Arrays of any size: ARRAY[*]

A parameter declared `ARRAY [*] OF INT`, or `ARRAY [*, *] OF INT`,
takes an array of any bounds with as many dimensions and the same
element type (Ed.3 6.5.3); `LOWER_BOUND(a, d)` and `UPPER_BOUND(a, d)`
give dimension `d`'s bounds, a DINT (Table 15). This is Ed.3's own
Example 2:

```
FUNCTION SUM : DINT
    VAR_IN_OUT A : ARRAY [*] OF INT; END_VAR
    VAR i, s : DINT; END_VAR
    FOR i := LOWER_BOUND(A, 1) TO UPPER_BOUND(A, 1) DO
        s := s + A[i];
    END_FOR;
    SUM := s;
END_FUNCTION
```

iecf compiles it as an in-out of a FUNCTION, a method or a
FUNCTION_BLOCK, where CODESYS and TwinCAT allow it, and passes it on to
another's (the harness case `flexarrays`). Ed.3 also allows it as a
FUNCTION's or method's input or output, which no vendor does; iecf does
not compile that yet (E0009), nor a whole `ARRAY[*]` assigned. Anywhere
else it is [E0033](../errors/E0033.md). An `ARRAY[*]` is not given to
an in-out of fixed bounds: its size is only known when it runs.

An index into one is checked against the bounds it was given, a fault
`INDEX` outside them, as for any array. So is `LOWER_BOUND(a, d)` with
a `d` outside the array's dimensions, which Ed.3 calls an error: a
fault `INDEX` when `d` is computed, refused when it is a constant.
LOWER_BOUND and UPPER_BOUND take an array of fixed bounds too, as Ed.3's
Example 1 does.

Ed.3 Example 2 also gives `SUM(A2[2])`, one row of a two-dimensional
array; iecf does not take part of an array yet: index every dimension.

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
one at an address (E0009 for now). A FUNCTION_BLOCK's in-out of an
array, a structure or a STRING is too, set by each call: the block
works on the caller's variable (`flexarrays`).
