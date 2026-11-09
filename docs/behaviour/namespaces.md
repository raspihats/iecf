# Namespaces

A namespace (Ed.3 6.9) groups types, functions, function blocks and
programs under a name, so that two libraries can each have a `Motor`.
Everything declared outside one is in the **global namespace**, the
standard functions and function blocks among it. A project without a
NAMESPACE compiles as it always did.

The choices below are iecf's where Ed.3 says nothing or contradicts
itself; each is a decision of [the proposal](../design/namespaces.md)
(D1 to D8), checked against the vendors there.

## Declaring

```
NAMESPACE Lib.Shapes                   (* Lib, and Shapes inside it *)
    TYPE Corner : (Sharp, Round); END_TYPE
    FUNCTION area : INT
    VAR_INPUT w, h : INT; END_VAR
        area := w * h + offset();
    END_FUNCTION
    FUNCTION INTERNAL offset : INT     (* seen only inside Lib.Shapes *)
        offset := 100;
    END_FUNCTION
END_NAMESPACE
```

- A namespace holds **types, functions, function blocks, programs and
  namespaces**. A PROGRAM may be in one (D1); its CONFIGURATION names
  it by its path, `PROGRAM p WITH fast : Plant.Main;`.
- **No variables**: VAR_GLOBAL, a CONFIGURATION and VAR_CONFIG stay
  outside every NAMESPACE (E0005), and `Lib.x` is never a variable
  (E0028, D7).
- `NAMESPACE A.B` is `NAMESPACE A` with `NAMESPACE B` inside. Declared
  again, in the same file or another, a namespace gets more elements
  (Table 65 NOTE); an element declared twice is E0012.
- An element and a namespace of one name side by side - a FUNCTION_BLOCK
  `Motor` and a NAMESPACE `Motor` - is E0012 (D8): `Motor.Start` could
  be read two ways.
- An element's name may not be an elementary type's (`INT`, E0018), but
  may be a standard function's or function block's: a namespace's own
  `TON` hides the standard one inside it (6.9.2 Example 2).
- `INTERNAL` before a function's, function block's or program's name
  keeps it inside its namespace and the namespaces in that one;
  `NAMESPACE INTERNAL N` shows N's elements only in the namespace one
  level above (Figure 29, E0028). Ed.3 lists types too, but gives them
  no place to write INTERNAL; iecf's types are public.

## Using

From outside, an element is named with its namespaces in front,
`Lib.Shapes.area(2, 3)`. `USING Lib.Shapes;` brings in its elements by
their short names; not the namespaces inside it (6.9.4 Example 2):

| Where | Written | For |
|---|---|---|
| at the top of a file, before its declarations | `USING Lib.Shapes;` | that file (D6) |
| after a namespace's name | `NAMESPACE Plant USING Lib.Shapes;` | that declaration of the namespace |
| after a POU's name, a function's result type | `FUNCTION_BLOCK user USING Lib;` | that POU; a PROGRAM too (D5) |

A USING names a namespace from where it is written outwards: inside
`Lib`, `USING Shapes;` is `Lib.Shapes`.

## Which one a name means

A short name, or the first part of a dotted one, is looked for from the
inside out (D2, D3), as C#'s names are:

1. the POU's own variables, then what the POU's USINGs bring in;
2. the namespace the POU is in, then what its USINGs bring in;
3. each namespace around that one, outwards, the same;
4. the global namespace, then what the file's USINGs bring in.

The first step that has the name decides. Within a step, a namespace's
own element comes before one a USING brings in. Two USINGs of one step
that both bring in the name is E0029, only where it is used: name the
one meant in full.

So inside a namespace `Plant` that has a namespace `Lib` of its own,
`Lib.twice` is `Plant.Lib.twice`, and the top-level `Lib` cannot be
reached by name. With the extension
[global-namespace](../extensions.md), `_.` starts at the global
namespace: `_.Lib.twice`, and `_.TON` for the standard timer where a
namespace has its own (D4).

## In the C and the symbol table

A namespace's element is named in the C by its path, its parts joined by
two underscores (`B_LIB__SHAPES__AREA`), and in `<p>_pou_names`, the
symbol table and every message by its path with dots
([The C output](../c-output.md) §2).

The harness cases `namespaces` (over two files) and `globalns` run
each rule above with values worked by hand; the checker's tests
(`crates/iecf-sema/src/namespaces.rs`) give each error.
