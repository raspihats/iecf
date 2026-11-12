# Statements

The ST statements (Ed.3 7.3.3) do what the standard says; this page has
what it leaves to the implementer, and what iecf adds at the edges.

## A statement that faults changes nothing

Each expression that can fault is computed, and checked, before the
statement stores anything: an assignment whose value divides by zero
leaves its target as it was, and the task stops there ([Results, flags
and faults](overview.md)). A statement left undone — a REAL that does
not fit its integer, a call whose EN is FALSE — stores nothing either,
and the task goes on.

## FOR

`FOR i := a TO b BY s DO ... END_FOR` evaluates a, b and s once, before
the first iteration (Ed.3 7.3.3.4.2). The test is made before each
iteration, so the body does not run at all when a is already past b.

**After the loop, the variable is one step past its end**: after
`FOR i := 1 TO 10`, i is 11; after `FOR i := 3 TO 1 BY -1`, 0. The
standard calls the value Implementer specific, but its own examples
assume this ("terminates when the value of the variable I reaches 0";
Table 72's J "will have the value 101"), and CODESYS ("at the end, i
always will be one step outside the range") and matiec do the same.
An EXIT leaves the variable where it was. Decided 2026-11-07 (florin,
[Implementer choices](../dependencies.md#for-the-variable-after-the-loop-ed3-73342)).

**At the type's limit the loop still ends.** `FOR i := 0 TO 32767` on an
INT runs 32768 times and stops; its last step wraps i to -32768 and is
flagged `OVERFLOW`, as any overflow is. On CODESYS that loop never ends
("If the end value of the counter is equal to the upper limit of the
data type of the counter, then an infinite loop results").

A variable of a **subrange** type stays inside its subrange: the loop
ends at its last value, and an end beyond the subrange is the bound it
would pass, flagged `CONVERSION` ([Subranges](arrays-subranges.md)).

A step of 0 never ends, as written.

**A body that moves the variable outside the TO range ends the loop**
(Ed.3: "The iteration is terminated when the value of the control
variable is outside the range specified by the TO construct"; the
standard also says the body "shall not" alter it). `FOR k := 1 TO 10`
with `k := k + 2` in its body runs for k 1, 4, 7 and 10, and ends with
k 13. Until 2026-11-12 such a loop ran on through the type's range,
wrapping.

The variable may be **at an address**, `i AT %MW2 : INT`: the loop
counts in a copy, written to the address as each pass starts and read
back after it, so the body sees it and may change it there; after the
loop the address holds the value the variable would, an EXIT leaving
what the body left. The harness case `locmore` runs these.

## WHILE, REPEAT, EXIT, CONTINUE, RETURN

WHILE tests before each iteration, REPEAT after. EXIT leaves the
innermost loop; CONTINUE goes to its next iteration — in REPEAT, to the
UNTIL test. RETURN leaves the POU; in an SFC action, it ends the action
([SFC](sfc.md)).

## CASE

The first list a value matches runs; none matching runs the ELSE, if
there is one. A selector left undone runs no branch.

A label is a constant expression (Ed.3 7.3.3.3.3): integer literals,
`+ - * / MOD`, and the names of constants, as an array's bound is
([E0032](../errors/E0032.md) for a variable there). It is compared by
its value, whatever integer type its constants are; a value the
selector's type cannot hold is refused as a literal is (E0020). For an
enumeration, a label is one of its values. The harness case
`flexarrays` runs one.

```
VAR CONSTANT FIRST : INT := 1; LAST : INT := 8; END_VAR
CASE k OF
    FIRST:            x := 1;
    FIRST + 1..LAST:  x := 2;
ELSE
    x := 0;
END_CASE;
```
