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

## Libraries: a namespace given on the command line

CODESYS and TwinCAT put each library in a namespace, by default the
library's name: `TcoCore.TcoObject` names a block of the library
TcoCore, and `TcoObject` does too, unless the library is
qualified-only ([CODESYS](https://content.helpme-codesys.com/en/LibDevSummary/namespaces.html)).
Their files say nothing of it: the library manager does. iecf takes it
from the command line, for `check` and `build`:

```
iecf check --library TcoCore=src/TcoCore --library TcoElements=src/TcoElements src/**/*.st
iecf build -o out --qualified-library Lib=lib main.st lib/*.st
```

- `--library NAME=PATH`: each file whose name is PATH or under it has its
  types, FUNCTIONs, FUNCTION_BLOCKs, classes, interfaces and PROGRAMs
  put in the namespace NAME (a NAMESPACE around them, Ed.3 6.9); its
  global variables and configurations stay where they are. Its
  PROGRAMs went there on 2026-11-23 (florin: "go"): before, they stayed
  global, and two libraries each with a MAIN, as each of TcOpen's has,
  clashed ([E0012](../errors/E0012.md)). CODESYS: "The access becomes
  unambiguous by adding the namespace in front of the POU name"
  ([library development, ABB's copy](https://help.plc.abb.com/AB270_en/_cds_library_development_information.html));
  a PROGRAM is a POU, and TwinCAT builds TcOpen's apps beside libraries
  that each have a MAIN. Every file
  of the project then uses NAME (`USING NAME;`), so a short name works
  too, as in CODESYS.
- `--qualified-library NAME=PATH`: the same, but no file uses it - an
  element is named `NAME.x` only, CODESYS's qualified-only library.

From there it is Ed.3's namespaces, as above: a name two libraries both
have is ambiguous by its short name ([E0029](../errors/E0029.md)), and
`NAME.x` says which. A dotted name nothing declares says to give the
library ([E0011](../errors/E0011.md)).

## Global variable lists: a name given on the command line

A CODESYS or TwinCAT project keeps its global variables in global
variable lists, each an object with a name: `GVL.x` names the variable
x of the list GVL, and `x` does too, unless the list has
`{attribute 'qualified_only'}`, "only addressed by specifying the
global variable list name"
([CODESYS](https://content.helpme-codesys.com/en/CODESYS%20Development%20System/_cds_pragma_attribute_qualified_only.html),
[Beckhoff](https://infosys.beckhoff.com/content/1033/tc3_plc_intro/2529769099.html)).
An ST file has no way to say a list's name, and iecf reads no pragma,
so the command line says it, as for a library (decided 2026-11-20,
florin: "3 A"):

```
iecf check --extension codesys --gvl IO=gvl/io.st --qualified-gvl GVL=gvl/gvl.st src/*.st gvl/*.st
```

- `--gvl NAME=FILE`: the VAR_GLOBAL sections of FILE (or of each file
  under the directory FILE) are the list NAME. `NAME.x` names its x
  anywhere: in a body, an initial value, an array's bounds or a
  STRING's length. Its variables stay global variables as any other,
  named `x` too, and a POU reaches them as it reaches any (VAR_EXTERNAL,
  or the implicit-externals extension, which the `codesys` pack has).
- `--qualified-gvl NAME=FILE`: the same, but `NAME.x` only: the variable
  is the global `NAME.x` (in the C, `NAME__x`), which no plain `x`
  names; inside the list its own variables are named bare, as in
  `ARRAY[1..N]`. A plain `x` there is [E0010](../errors/E0010.md),
  which says to write `NAME.x`.
- A POU that declares a variable of the list's name means its own:
  `NAME.x` there is its variable's x.
- Two lists that are not qualified-only give one global namespace: a
  name both have is declared twice ([E0012](../errors/E0012.md)).

The harness case `codesysgvl` reads a list both ways, in its own files.

## In the C and the symbol table

A namespace's element is named in the C by its path, its parts joined by
two underscores (`B_LIB__SHAPES__AREA`), and in `<p>_pou_names`, the
symbol table and every message by its path with dots
([The C output](../c-output.md) §2).

The harness cases `namespaces` (over two files) and `globalns` run
each rule above with values worked by hand; the checker's tests
(`crates/iecf-sema/src/namespaces.rs`) give each error.
