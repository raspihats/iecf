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

## Initial values are constant expressions

An initial value is a literal or a constant expression (Ed.3 6.4.4.1,
Figure 6; 6.6.1.2.4): literals, the names of constants, enumerated and
named values, and operators. The compiler works it out, so nothing of
it is left to the run - no flag and no fault as the program starts:

```
VAR CONSTANT
    N  : INT := 3;
    N2 : INT := N + 1;                    (* 4: a constant of constants *)
    KR : REAL := 1.5;
    KT : TIME := T#2s;
    KM : MODE := run;
END_VAR
VAR
    a   : INT := N * 2 + 1;               (* 7 *)
    r   : REAL := KR * 2.0;               (* 3.0 *)
    t   : TIME := KT + T#500ms;           (* T#2s500ms *)
    b   : BOOL := NOT (N > 2);            (* FALSE *)
    w   : BYTE := 16#0F AND 16#3C;        (* 16#0C *)
    m   : MODE := KM;                     (* run *)
    arr : ARRAY [1..N] OF INT := [N, 2(N2)];
    p   : PT := (x := N2, y := KR);
END_VAR
```

- The constants are those a bound names (above). A method also names
  its type's constants, a derived type its bases'; a `TYPE`'s initial
  value and a `VAR_GLOBAL`'s name the `VAR_GLOBAL CONSTANT`s directly.
  A constant may be declared below where it is named.
- A string, an array or a structure given a constant's name starts as
  that constant's own initial value.
- An integer expression is worked out exactly, not in its operands'
  type, as a bound is: `d : DINT := N2 * 10000` is 40000 though `N2`
  is an INT. A value its variable's type does not hold is
  [E0020](../errors/E0020.md). REAL is worked out in single precision,
  LREAL in double, as the run would; an infinite result or one that is
  no number is E0020 too.
- A division by zero, and a constant whose initial value names itself,
  have no value: [E0032](../errors/E0032.md). `MOD` by zero is 0, as
  when the program runs ([Integers](integers.md)).
- An input, a block's or a FUNCTION's, not given at a call takes its
  initial value the same way, and so does a FUNCTION's own variable at
  each call.

An initial value that is not a constant expression - a function's
result, `x : REAL := INT_TO_REAL(N)` - is computed as the variable is
set up, when it can neither fault nor be flagged; one that can is
[E0009](../errors/E0009.md) for now, as is the name of a variable. The
harness case `constinit` runs each of these.

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
`ARRAY[*]` is only its in-out. Anywhere else an `ARRAY[*]` is
[E0033](../errors/E0033.md).

A whole `ARRAY[*]` is assigned from another, or from an array of fixed
bounds, of as many dimensions and the same element type: `dst := src;`.
The two must have as many elements in each dimension, which only the
run knows: when they differ it is a fault `INDEX` and nothing is
copied. The bounds themselves need not match, `1..3` copies onto
`-1..1`. An `ARRAY[*]` is not assigned to an array of fixed bounds, nor
given to a parameter of fixed bounds (E0013), as Siemens has it (the
harness case `flexcopy`).

An index into one is checked against the bounds it was given, a fault
`INDEX` outside them, as for any array. So is `LOWER_BOUND(a, d)` with
a `d` outside the array's dimensions, which Ed.3 calls an error: a
fault `INDEX` when `d` is computed, refused when it is a constant.
LOWER_BOUND and UPPER_BOUND take an array of fixed bounds too, as Ed.3's
Example 1 does.

Ed.3 Example 2 also gives `SUM(A2[2])`, one row of a two-dimensional
`ARRAY [1..20, -2..2]`. An array indexed in its first dimensions only
is the array of the others; so is an element of an array of arrays, by
its declaration:

```
VAR
    a2 : ARRAY [1..2, 0..2] OF INT := [1, 2, 3, 4, 5, 6];
    aa : ARRAY [1..2] OF ARRAY [0..2] OF INT := [[1, 2, 3], [4, 5, 6]];
    row : ARRAY [0..2] OF INT;
END_VAR
total := SUM(a2[2]);        (* 15: the row 4, 5, 6 *)
total := SUM(aa[2]);        (* 15 *)
total := SUM(a2[i]);        (* the row i; i checked, a fault INDEX outside 1..2 *)
row := a2[1];               (* a row copied: 1, 2, 3 *)
x := a2[2][0];              (* 4, as a2[2, 0] *)
x := aa[2][0];              (* 4 *)
```

A row is an array like any other: given to an `ARRAY[*]` or to a
parameter of its bounds, or assigned. An `ARRAY[*, *]` indexed
once is an `ARRAY[*]`, to pass on. More indices than dimensions is
E0014.

The harness case `flexarrays` runs these.

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
in their order, in an INT (as CODESYS and TwinCAT hold one), so it has
at most 32768 of them (E0020 at the next). Only the type's own values can be stored in
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

The base type is an integer or a bit string type (but BOOL), INT when
the declaration names none, as in CODESYS and TwinCAT: `(slow := 2,
fast := 4)` is added to an INT as it is, and a value an INT cannot hold
needs the type named, `DINT (far := 100000)`. A value is a constant expression the base
type holds (E0020 otherwise); one not given is one more than the value
before it, from 0, as CODESYS and TwinCAT count. A variable of the base
type is given to an in-out of the type. A variable's own declaration
may be one, `x : INT (lo := 1, hi := 5) := hi;`: its names are known in
the POU, its numbers may name the POU's constants, and where two such
declarations share a name it is used without a type's name as long as
they give it the same number and base type (`fbargs`). The harness case `enums` runs
all of this.

## Arrays and structures given to a FUNCTION

An array or a structure is also a FUNCTION's or method's input, output,
in-out or result: an input copied in as the function starts, so the
function never writes the caller's; a result returned whole, `xs :=
RAMP(start := 1)`, or given straight to another's input (the harness
case `fnparams`). A part of what a FUNCTION gives is read where it is
called, `Shift(p).x`, `Ramp(3)[i]`, `Make(2).n` for an instance (the
harness case `callparts`): the call is made once, into a buffer of the
statement's, and the part is read from it; an index is checked as any
other. An in-out of one is given any variable of its type: one
at an address (`AT %MW0`), a part of it, or one reached through a
reference, is given as a copy, read before the call and stored back as
it returns, since the image holds no C object to point at; so is an
array there given to an `ARRAY[*]`. Two in-outs of one call given the
same variable at an address are two copies: the one stored back last
is what stays (the harness case `inoutdata`). A FUNCTION_BLOCK's in-out of an
array, a structure or a STRING is too, set by each call: the block
works on the caller's variable (`flexarrays`).
