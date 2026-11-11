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

It is an in-out of a FUNCTION, a method or a FUNCTION_BLOCK, as in
CODESYS and TwinCAT, and is passed on to another's (the harness case
`flexarrays`). It is also a FUNCTION's or method's input or output, as
Ed.3 allows and Siemens has it for an FC (the harness case `flexio`):

```
FUNCTION TOTAL : DINT
    VAR_INPUT a : ARRAY [*] OF INT; END_VAR
    VAR i : DINT; END_VAR
    FOR i := LOWER_BOUND(a, 1) TO UPPER_BOUND(a, 1) DO
        TOTAL := TOTAL + a[i];
    END_FOR;
END_FUNCTION

FUNCTION CLEAR_TO
    VAR_INPUT v : INT; END_VAR
    VAR_OUTPUT a : ARRAY [*] OF INT; END_VAR
    VAR i : DINT; END_VAR
    FOR i := LOWER_BOUND(a, 1) TO UPPER_BOUND(a, 1) DO
        a[i] := v;
    END_FOR;
END_FUNCTION
```

called `TOTAL(x)`, `TOTAL(a := x)`, `CLEAR_TO(v := 7, a => x)` or
`CLEAR_TO(7, x)`. Such an input or output is the caller's array itself,
not a copy of it - the function cannot hold an array whose size it does
not know - and what follows from that is:

- the input is read, never written: an assignment to it or to an
  element, or giving it to an in-out or an output, is E0015. It is
  given on to another `ARRAY[*]` input;
- the output's elements are set to their type's initial value as the
  call starts, as any output of a FUNCTION is; what the function then
  writes is in the caller's array at once, so a call that faults has
  already changed it, where an output of fixed bounds is only stored
  when the call returns;
- the call gives each one an array: there is no initial value to stand
  in for an input left out, and no place for an output not taken
  (E0016);
- one array given as the input and as the output of the same call is
  set to its initial value before it is read.

A FUNCTION_BLOCK's input or output is kept in the instance, so an
`ARRAY[*]` is only its in-out. A whole `ARRAY[*]` assigned is not
compiled yet (E0009). Anywhere else an `ARRAY[*]` is
[E0033](../errors/E0033.md). It is not given to a parameter of fixed
bounds: its size is only known when it runs.

An index into one is checked against the bounds it was given, a fault
`INDEX` outside them, as for any array. So is `LOWER_BOUND(a, d)` with
a `d` outside the array's dimensions, which Ed.3 calls an error: a
fault `INDEX` when `d` is computed, refused when it is a constant.
LOWER_BOUND and UPPER_BOUND take an array of fixed bounds too, as Ed.3's
Example 1 does.

Ed.3 Example 2 also gives `SUM(A2[2])`, one row of a two-dimensional
`ARRAY [1..20, -2..2]`. iecf takes a row of an array of arrays, whose
element is an array by its declaration; an `ARRAY [a, b]` is indexed in
every dimension (E0014 otherwise):

```
VAR
    aa : ARRAY [1..2] OF ARRAY [0..2] OF INT := [[1, 2, 3], [4, 5, 6]];
END_VAR
total := SUM(aa[2]);        (* 15: the row 4, 5, 6 *)
total := SUM(aa[i]);        (* the row i; i checked, a fault INDEX outside 1..2 *)
x := aa[2][0];              (* 4 *)
```

The harness case `flexarrays` runs both.

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
in their order, in a DINT. Only the type's own values can be stored in
it (Ed.3 6.4.4.2); there is nothing to check at run time. A variable
starts as the type's initial value, the first of its values unless the
declaration gives one. A value two enumerations have is written
`Type#value` (E0010 otherwise).

```
TYPE
    mode : (idle, run, stop) := run;
END_TYPE
VAR
    m  : mode;                      (* run *)
    mi : (low, mid, top) := mid;    (* declared in place *)
END_VAR
```

An enumeration is also a FUNCTION's or method's input, output, in-out
and result and a block's input, output and in-out; an input not given
and a result not set are the type's initial value.

## Types with named values

A declaration that gives its values numbers, or names a base type, is
a type with named values (Ed.3 6.4.4.3). Its variables are used as the
base type is: any value of it assigned, arithmetic, comparison with a
number; the names are constants of that type, a CASE's labels among
them.

```
TYPE
    col : INT (red := 1, green := 5, blue) := green;   (* blue is 6 *)
    lv  : WORD (lo := 16#10, hi := 16#20);
END_TYPE
VAR c : col; x : INT; END_VAR                          (* c is green, 5 *)
c := 27;
c := red + 1;
x := c + col#blue;
CASE c OF
    red:              x := 1;
    green - 3, blue:  x := 2;
END_CASE;
```

The base type is an integer or a bit string type (but BOOL), DINT when
the declaration names none. A value is a constant expression the base
type holds (E0020 otherwise); one not given is one more than the value
before it, from 0, as CODESYS and TwinCAT count. A variable of the base
type is given to an in-out of the type. The harness case `enums` runs
all of this.

## Arrays and structures given to a FUNCTION

An array or a structure is also a FUNCTION's or method's input, output,
in-out or result: an input copied in as the function starts, so the
function never writes the caller's; a result returned whole, `xs :=
RAMP(start := 1)`, or given straight to another's input (the harness
case `fnparams`). An in-out of one is given a variable in memory, not
one at an address (E0009 for now). A FUNCTION_BLOCK's in-out of an
array, a structure or a STRING is too, set by each call: the block
works on the caller's variable (`flexarrays`).
