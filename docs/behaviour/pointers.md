# Pointers

Only with `--extension pointers` (in the `codesys` pack): CODESYS's
and TwinCAT's `POINTER TO`, `ADR` and `SIZEOF`. The standard has none;
its references, `REF_TO` and `REF()`, are iecf's always
([References](references.md)). Why iecf builds them so, and what it
does not: [Pointers: the proposal](../design/pointers.md) and
[extensions](../extensions.md#pointers-codesyss-pointer-to-adr-and-sizeof-each-pointer-checked-where-it-is-used).

## What a pointer is

A pointer is an address **inside one variable**: `ADR(x)` takes the
variable `x` is part of - an array of which `x` is an element, a
structure of which it is a field - and points at `x` in it. Arithmetic
counts **bytes**, as CODESYS's does, and may move the pointer anywhere,
past either end of the variable too; only a use checks it.

```
VAR a : ARRAY[0..3] OF REAL; pt : POINTER TO REAL; END_VAR
pt := ADR(a[1]);          (* a[1], inside a *)
pt := pt + SIZEOF(REAL);  (* a[2] *)
x := pt^;                 (* read, checked *)
pt := pt + 100;           (* past a: allowed, as long as it is not used *)
```

- `p^` reads or writes what the pointer points to; `p^[i]` and `p^.f`
  an element or a field of the array or structure it points to. The
  element's index is checked against the array type's bounds, as any
  index is, and **every read and write is checked against the
  variable**: outside it is a fault, `IECF_FAULT_INDEX`; through NULL,
  `IECF_FAULT_NULL`. The statement stops, as for an index out of range
  ([The C output](../c-output.md) §5). C's undefined behaviour is never
  reached.
- **Another type through a pointer** is allowed, as in CODESYS: `POINTER
  TO DWORD` over a `BYTE` array reads four bytes as a DWORD, **in the
  platform's byte order** - the bytes as they are in memory, as CODESYS
  reads them (little-endian on x86, ARM and wasm). A BOOL read through a
  pointer is TRUE for any byte but 0.
- **Arithmetic**: `p + n`, `n + p`, `p - n` with any integer `n`, in
  bytes; `p - q` of two pointers into one variable, a DINT (into two
  variables: `IECF_FAULT_POINTER`). **Comparisons**: `=` and `<>` of
  pointers, or with 0 or NULL; `<`, `<=`, `>`, `>=` of two pointers into
  one variable (into two, or with NULL, a fault).
- **`SIZEOF(x)`** of a variable or a type: its size in bytes, as the
  platform's C compiler lays it out, of the smallest unsigned type that
  holds it - USINT, UINT, UDINT or ULINT - chosen from iecf's layout,
  each part aligned to its size, which no platform's exceeds (the C
  checks it as it compiles). **SIZEOF of a pointer is refused**: CODESYS
  gives 4 or 8 by the platform, iecf's checked pointer more, and no
  number is right for every platform; write `SIZEOF(p^)` for what it
  points to.
- A pointer is **0 or NULL** until set: `p = 0` tests it. No number
  converts to a pointer, nor a pointer to a number.

## Where a pointer may be kept

A pointer to a variable gone when its POU returns - a FUNCTION's or a
method's variable, a VAR_TEMP - would dangle once the call is over.
iecf refuses ([E0027](../errors/E0027.md)) storing one where it
outlives the call: a block's static variable or input, a global, an
in-out, an output, a FUNCTION's result. Passing it down, as OSCAT
does - `_ARRAY_SORT(ADR(local), SIZEOF(local))` - is fine: the callee's
input lives only during the call.

What counts as "may point to a temporary": `ADR` of a temporary or of
an in-out (whose variable is the caller's), and any pointer held in a
temporary or given as an input, whose value no one can vouch for. So a
pointer passed in cannot be stored in a static variable; take the
address of a variable that lives as long as the program there.

## What a pointer may point to

Data: an elementary value, a STRING, an array or a structure of them,
with no reference, interface, instance or pointer in it (E0027 for a
`POINTER TO` anything else).

- **A STRING** is read and written whole through a pointer, `ps^ :=
  'abc'`, `t := ps^`, `LEN(ps^)`, its n + 1 bytes checked to lie in the
  variable. A WSTRING is not compiled through a pointer (E0009): its
  16-bit units need an alignment no pointer promises; read them through
  a `POINTER TO WORD`.
- **An array or a structure** is copied whole through a pointer, `dst :=
  pa^`, `pq^ := r`, its bytes checked and copied as they are.
- **A variable at an address**, `ADR(i2)` of `i2 AT %IB2`, or an element
  of one: the pointer keeps the image area and the byte, found through
  the image at each use, as the platform may move the image between
  runs; it may walk the whole area.
- **An initial value** `ADR(x)` of a variable that lives as long as the
  pointer: set up as `REF()`'s is. Of a temporary into a static
  pointer, E0027.

Not compiled (E0009, said where written): a structure with relative
locations through a pointer, whose bytes are laid out little-endian as
the image is, not as the platform's; `ADR` of a bit (`b.3`). `REF()` of
what a pointer points to is refused ([E0026](../errors/E0026.md)).

The harness cases `pointers` (a STRING walked byte by byte, an array
passed by ADR and SIZEOF, a buffer cleared by pointer arithmetic, a
DWORD read over bytes, a field through a pointer, two pointers
subtracted), `pointers2` (a STRING and whole values through pointers,
ADR at an address and in an initial value) and `pointerfault` (a read
past the variable) run them.
