# Pointers: the proposal

CODESYS and TwinCAT have C-like pointers, `POINTER TO T` with `ADR`,
`^`, `SIZEOF` and arithmetic in bytes; Ed.3 has none of that, only
references (`REF_TO`, Table 12), which iecf builds. Code written for
CODESYS uses pointers a great deal: 74 of the 562 objects of the OSCAT
BASIC library (2026-11-19, the OSCAT round). This proposes whether and
how iecf could take them as an extension, `pointers`, and lists the
decisions that are florin's (§6). NOT BUILT; NOTHING DECIDED. Without
the extension `POINTER TO` is E0005 and `ADR`, `SIZEOF` E0010, as today.

## 1. What Ed.3 says

- A reference, `REF_TO T` (6.4.4.10, Table 12), is set by `REF(x)` to a
  variable, or to NULL; `r^` dereferences it; references are compared
  with `=` and `<>` only. There is no arithmetic on a reference, and no
  conversion between a reference and a number.
- `REF()` of a temporary - a FUNCTION's or method's variable, a VAR_TEMP
  - is not permitted (6.4.4.10.3), so a reference never outlives what
  it references. iecf refuses it (E0026), and refuses a reference kept
  where it could outlive its target (E0027).
- iecf builds all of it, NULL dereferenced being a fault
  ([References](../behaviour/references.md)). Clause 5 lets an
  implementation add features as extensions (Ed.3 5 c); pointers would
  be one.

## 2. What the vendors do

| | CODESYS | TwinCAT | PLCnext | Siemens S7-1500 |
|---|---|---|---|---|
| `POINTER TO T`, `p^` | yes | yes | no | no (VARIANT, `REF_TO` from V15) |
| `ADR(x)` | yes: "the 32-bit address (or the 64-bit address, if possible)", into DWORD, LWORD, `__XWORD` or a pointer | yes: into PVOID, DWORD (32-bit), LWORD, `__XWORD` | no | no |
| arithmetic | in **bytes**: `(piData + i * SIZEOF(INT))^` | the same; its static analysis flags any but `=`, `<>`, `+`, ADR | - | - |
| difference of two pointers | a DWORD, "even on 64-bit platforms" | the same | - | - |
| `SIZEOF(x)` | the smallest unsigned type that holds it (USINT, UINT, UDINT, ULINT); XSIZEOF from 3.5.16 | the same | SIZEOF gives a UDINT | - |
| checking | a user-written `CheckPointer` function, called before each use | the same | - | VARIANT checks types |
| `__NEW`, `__DELETE` | yes, with dynamic memory switched on | yes | - | - |

Sources: CODESYS help ("POINTER TO", "ADR", "SIZEOF", "CheckPointer",
"__NEW"); Beckhoff InfoSys ("POINTER TO", "ADR", PVOID and __XWORD, the
static analysis rules SA0061 and SA0064); PLCnext Engineer help (its
keyword index has no pointer; SIZEOF); Siemens programming guideline
v14 §2.8.5 and the REF_TO example for S7-1500 V15. CODESYS documents
neither pointer comparison nor casts between pointer types; code relies
on both.

## 3. What code does with them (OSCAT BASIC 3.35)

- **Walking a STRING's bytes** (34 objects, `POINTER TO BYTE`, 20 more
  `POINTER TO ARRAY[...] OF BYTE`): `pt := ADR(str); WHILE pt^ <> 0 DO
  ... pt := pt + 1;` - the character codes, in place.
- **An array of any length as an argument** (18 objects, `POINTER TO
  ARRAY[0..32000] OF REAL` with `SIZEOF`): `_ARRAY_SORT(ADR(a),
  SIZEOF(a))`, the callee indexing `pt^[i]` up to `size / SIZEOF(REAL)`.
  Ed.3's `ARRAY[*]` (6.5.3), which iecf builds, does the same safely.
- **Reinterpreting bytes** (6 objects, `POINTER TO DWORD` over a
  BYTE array; `CHK_REAL` reads a REAL's bits through a DWORD pointer,
  "real_to_dword does not work because it treats dword as a number on
  many systems").
- **Pointer arithmetic and comparison** in loops (`WHILE pt < end DO`,
  `pt := pt + 4`), the end computed as `ADR(x) + SIZEOF(x)`.
- **A pointer kept in a DWORD** (2 objects, `_BUFFER_CLEAR` and its
  kin): `temp := pt` to test its alignment. On a 64-bit platform the
  address does not fit: this code is not portable even between CODESYS
  targets.

## 4. What iecf cannot do: C's pointers as they are

The C iecf writes has no undefined behaviour ([The C
output](../c-output.md) §1), allocates nothing, and is the same on 8-
to 64-bit platforms. CODESYS's pointers, translated to C's, would break
each of these:

- **Out of bounds**: `pt := pt + 1` past an array, then `pt^`, is
  undefined in C - a fault iecf could not report.
- **Type punning**: a DWORD read through a pointer to BYTEs breaks C's
  aliasing rule (C99 6.5 p7) and, unaligned, traps on some processors.
- **Addresses in DWORDs**: a 64-bit address does not fit; a DWORD made
  back into a pointer is no pointer C can use.
- **Dangling**: `ADR` of a FUNCTION's variable kept after the call.
- **`__NEW`**: dynamic memory, which the output never uses.

## 5. The proposal: checked pointers

`--extension pointers`, each pointer a **fat pointer** - the object it
was taken of, its size, and an offset into it - checked where it is
used, so every case of §4 is a fault or an error, never undefined:

1. **`POINTER TO T`** of any elementary type, STRING, array or
   structure; `ADR(x)` of a variable, an element, a field, a STRING;
   `p^` and `p^[i]`, `p^.f` through it. NULL is 0, as CODESYS's: a
   pointer is NULL until set, `p = 0` tests it.
2. **Arithmetic in bytes**, as CODESYS's: `p + n`, `p - n`, `p := p +
   SIZEOF(T)`; `p - q` of two pointers into one object, a DINT (iecf's
   choice, where CODESYS gives a DWORD); `=`, `<>`, `<`, `<=`, `>`, `>=`
   between pointers into one object. A pointer may stand anywhere
   inside its object and one past its end, as C's may; only a use
   checks.
3. **A use is checked**: `p^` reads or writes `SIZEOF(T)` bytes at the
   offset; outside the object, or NULL, it is a fault
   (`IECF_FAULT_INDEX`, `IECF_FAULT_NULL`), as an index out of range
   is. Pointers into different objects compared or subtracted: a
   fault.
4. **Another type through a pointer**: `POINTER TO BYTE` and `POINTER
   TO DWORD` (any two elementary types and arrays of them) convert to
   each other, as in CODESYS; the bytes are read and written through
   `memcpy` (portable C, no aliasing or alignment fault), little-endian
   as iecf lays out a process image (§4 of the C output).
5. **`SIZEOF(x)`** of a variable or a type: its size in bytes as iecf
   lays it out, a constant of the smallest unsigned type that holds it,
   as CODESYS's. Built with the extension, or on its own (§6, 5).
6. **Lifetime**: `ADR` of a temporary is allowed, as an argument passed
   down (OSCAT's use); kept where it could outlive the call - a static
   variable, a global, an output, a block's input - it is E0027, as a
   reference would be.
7. **Refused**: a pointer converted to or from a number (DWORD, LWORD,
   `__XWORD`, PVOID) - an address has no portable value; function
   pointers; `__NEW` and `__DELETE`; `CheckPointer` (iecf checks every
   use itself; a user's CheckPointer is an ordinary function).

What it would take: a new type in the checker (`Ty::Pointer`), ADR and
SIZEOF, the arithmetic and comparison rules, the lifetime rule (the
check E0027 makes for references, reused); in the C, a fat pointer
`{ unsigned char *base; size_t size; size_t off; }`, a load and a
store per elementary type through memcpy, the faults; harness cases for
each rule and the OSCAT patterns of §3. Larger than any of the 13
extensions of 2026-11-19 - about as large as REF_TO was.

What OSCAT would still need: `_BUFFER_CLEAR` and the two other blocks
that keep an address in a DWORD (§3, last) rewritten; the rest of its
74 pointer-using objects would compile.

## 6. Decisions for florin

1. **Build pointers at all?** For: 74 OSCAT objects, and CODESYS code
   at large. Against: neither the standard nor PLCnext and Siemens have
   them; Ed.3's `ARRAY[*]` and `REF_TO` do the safe half of it already.
   Recommendation: build, as §5 - checked, so iecf keeps its no-UB
   guarantee.
2. **Fat, checked pointers, or C's own?** Recommendation: fat (§5). C's
   would be faster and undefined on the first mistake.
3. **Pointers to and from numbers?** Recommendation: refuse (§5, 7).
   The alternative, an opaque 64-bit `__XWORD`, would let `temp := pt`
   compile and mean nothing portable.
4. **`p - q`: DINT or DWORD?** Recommendation: DINT, signed, as C's
   ptrdiff; CODESYS's DWORD cannot be negative.
5. **SIZEOF on its own, now?** It is small and needs no pointer:
   `SIZEOF(x)` as a constant. Recommendation: yes, as part of the
   pointers extension if that is built, else as its own (`sizeof`).
