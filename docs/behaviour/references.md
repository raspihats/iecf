# References

A reference (`REF_TO`, Ed.3 6.4.4.10) holds where a variable is, or
nothing: `NULL`, which every reference starts as unless its declaration
gives `REF(x)`. `REF(x)` takes a reference to `x`; `r^` is the variable
`r` references, read and written as `x` itself would be, and `r^.f`,
`r^[i]` its parts; `rf^(...)` calls the function block instance it
references. Two references to one type compare with `=` and `<>`, equal
when they reference the same variable, and either compares with `NULL`.

```
VAR
    level : INT;
    r : REF_TO INT := REF(level);
END_VAR
r^ := r^ + 1;          (* level + 1 *)
IF r <> NULL THEN ...
```

## A reference given, stored through, called through

A FUNCTION or a method may give a reference, `Pick(w) : REF_TO INT`,
NULL when it sets none; `Pick(TRUE)^` reads where it points,
`Pick(FALSE)^ := 5` writes there, `PickB()^(i := 4)` calls the instance
it gives, each call made once and the reference checked before
anything is stored. Through a reference to an instance, its input is
set from outside, `rt^.PT := T#3s`, and the whole of it assigned,
`rt^ := t3`, as at its own name. `REF(i)` of an interface variable, an
element or a field of one, gives a `REF_TO` the interface; `p^ := o`
sets the interface it references. The harness case `refmore` runs
each.

## A NULL reference dereferenced faults

`r^` with `r` NULL is a **fault** `NULL`, as an index outside its array
is: the statement stops before it stores anything, and the task stops
([Results, flags and faults](overview.md)). Ed.3 calls it an error and
leaves the check to the implementer ("at compile time, by the runtime
system, or by the application program"). TwinCAT stops its runtime on
an invalid pointer, unless the project's CheckPointer function says
otherwise; CODESYS offers the same hook and `__ISVALIDREF`. Test
`r <> NULL` first where a reference may be NULL.

<!-- checked -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `rz^` | `INT` | `0` | — | `z`, through `rz := REF(z)` |
| `rz^ + INT#5` | `INT` | `5` | — | |
| `rz^ OR TRUE` | `BOOL` | `TRUE` | — | |
| `nr^` | `INT` | — | `fault NULL` | `nr` is NULL |
| `nr^ + INT#1` | `INT` | — | `fault NULL` | nothing is stored |
| `CONCAT(rz^, 'ab')` | `STRING` | `'ab'` | — | `z` is `''` |
| `rz^` | `WSTRING` | `""` | — | |
| `CONCAT(rz^, "a$263A")` | `WSTRING` | `"a$263A"` | — | |
| `CONCAT(nr^, "a")` | `WSTRING` | — | `fault NULL` | nothing is stored |

## A reference into the process image

`REF(x)` of a variable at an address references it there: `r^` reads and
writes the bytes the variable has, as the variable does - little-endian,
its type's width, a BOOL at a bit that bit only, a BOOL in a whole byte
1 or 0 and any value but 0 TRUE, an array or structure its parts back to
back ([The C output](../c-output.md) §4). Where a reference points is
known only as the program runs, so each `r^` looks: in C memory, or in
the image. A WSTRING there is its units back to back, each
little-endian, at an odd byte too; through a reference it is copied out
to be read, and a store into it - whole, or `r^[i]` - is written back
whole, so units after its end stay as they were, as at its own name. A reference to a variable at an input address warns
([W0005](../errors/W0005.md)): what is written through it, the next
inputs overwrite.

## What REF() takes, and where a reference cannot be

`REF(x)` takes a variable, an element or a field that lives as long as
the program and could be assigned: not a VAR_TEMP or any variable of a
FUNCTION (gone when it returns, Ed.3), not EN or ENO, not a CONSTANT and
not a function block's output taken from outside (both could be written
through the reference) - [E0026](../errors/E0026.md).

A reference is not a VAR_IN_OUT (Ed.3), not RETAIN - after a restart it
would reference nothing - and not at an address
([E0027](../errors/E0027.md)).

A reference takes only its own type: `REF_TO ARRAY[0..9] OF INT` not an
array of other bounds, `REF_TO INT` not a subrange of INT (E0013).

## REFERENCE TO (an extension)

With the `reference-to` extension ([Implementer extensions](../extensions.md)),
iecf takes CODESYS's `REFERENCE TO` too: a reference used without `^`,
bound with `REF=`, tested with `__ISVALIDREF`.

```
VAR
    level : INT;
    r : REFERENCE TO INT;
END_VAR
r REF= level;
r := r + 1;            (* level + 1 *)
IF __ISVALIDREF(r) THEN ...
r REF= 0;              (* unbound *)
```

It starts unbound, and used unbound it is the same **fault** `NULL` as
`nr^`:

<!-- checked extension=reference-to -->
| Expression | Type | Result | Flag | Note |
|---|---|---|---|---|
| `__ISVALIDREF(ur)` | `BOOL` | `FALSE` | — | `ur` unbound |
| `ur` | `INT` | — | `fault NULL` | |
| `ur + INT#1` | `INT` | — | `fault NULL` | nothing is stored |
