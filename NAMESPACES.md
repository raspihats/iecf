# Namespaces: the proposal

Ed.3 6.9 (Tables 64 to 66) adds `NAMESPACE ... END_NAMESPACE`,
`INTERNAL` and `USING`. This proposed how to build them, and asked for
the decisions the standard leaves open (§6). DECIDED 2026-11-09
(florin): every recommendation of §6 as written. BUILT 2026-11-09, as
§4 says; what a program gets is in the book's
[Namespaces](../behaviour/namespaces.md) page.

## 1. What Ed.3 says

- A namespace groups **types, functions, function blocks, classes,
  interfaces and namespaces** (Annex A, `Namespace_Elements`); 6.9.2's
  text lists **programs** too. What has no namespace is in the
  **global namespace**, the standard functions and function blocks
  among it (6.9.1).
- Namespaces nest, lexically or by a dotted name:
  `NAMESPACE Standard.Timers ... END_NAMESPACE`. Several declarations of
  one name add to the same namespace, "e.g. in different Structured
  Text program files" (Table 65 NOTE).
- An element is public by default. `INTERNAL` makes it visible only
  inside its own namespace; `NAMESPACE INTERNAL N` makes N's public
  elements visible only in the namespace one level above (Figure 29,
  Example 3). An outer namespace can restrict access further, never
  widen it.
- From outside, an element is named with its namespaces in front:
  `Standard.Timers.TON` (6.9.3). Inside, the short name does; the long
  one is allowed too.
- `USING Standard.Timers;` makes a namespace's elements usable by their
  short names: in a file at global level, in a namespace after its name,
  in a function block after its name, in a function after its result
  type (6.9.4, Annex A). It does not bring in the namespaces nested in
  it: after `USING Standard;`, `Timers.TON` is an error (6.9.4
  Example 2).

6.9.4's wording follows the C# specification's using-directive section
nearly word for word: "enables the types contained in the given
namespace, but specifically does not enable types contained in nested
namespaces ... results in a compile-time error because no members named
... are in scope". Where Ed.3 is silent, this proposal takes C#'s rule
(§6).

## 2. What the vendors do

| | NAMESPACE in source | USING | global namespace written | INTERNAL |
|---|---|---|---|---|
| **PLCnext Engineer** | only around a TYPE block; a POU's namespace is a setting, or its name prefixed `NamespaceA.MyFB` | a POU's "Usings" setting; namespace-level USING mentioned, not documented | `_.`: "DERIVAT or _.DERIVAT" | not documented |
| **Siemens TIA Portal V21** (S7-1500 software units) | none: a block's or PLC data type's property, a unit's default | none | `_.`: `<Namespace>.<Block>` "searches relative to the calling block", `_.<Namespace>.<Block>` "searches globally" | not documented |
| **CODESYS** | none: a library's namespace, set in the library manager (`CAA.HANDLE`) | none | not documented | a POU keyword: "only allowed for the members of the current library" |
| **TwinCAT 3** | none: as CODESYS; `{attribute 'qualified_only'}` makes a GVL's variables `GVL.nVar` and an enumeration's values `E_Sample.eMember` | none | none | as CODESYS |
| **RuSTy** | none | none | none | none |
| **matiec** (the reference compiler) | none | none | none | none |

Sources: engineer.plcnext.help (Namespaces_General, Namespaces_Define_POU,
Namespaces_Define_DataTypes, Namespaces_Use); docs.tia.siemens.cloud
(v21, "Categorizing program elements in namespaces (S7-1500)");
content.helpme-codesys.com (LibDevSummary/namespaces, visibility);
infosys.beckhoff.com (attribute 'qualified_only'). Both vendors that
document namespaces written in programs, Siemens and PLCnext, write the
global namespace `_.`. Neither documents what happens when a short name
could mean two things.

## 3. Where Ed.3 contradicts itself or is silent

1. **Programs in a namespace.** 6.9.2 lists programs, and
   `Prog_Type_Access` (the type a CONFIGURATION names) takes namespaces
   in front; `Namespace_Elements` leaves `Prog_Decl` out.
2. **The global namespace's name.** 6.9.3's example writes
   `Ton2: PUBLIC.TON; (* uses the standard timer *)`. PUBLIC is a
   keyword (an access specifier), the grammar has no such form, and no
   other line mentions it. Without some spelling, a namespace that
   declares its own TON cannot reach the standard one.
3. **USING in a PROGRAM.** 6.9.4 says USING may follow "the name of ...
   a POU"; `Prog_Decl` has no `Using_Directive`.
4. **Which one a short name means**, when it could be a local
   variable, an element of the enclosing namespace, one brought in by
   USING, one of a parent namespace, or a global one; and whether two
   USINGs bringing in the same name is an error.
5. **A dotted name's first part**: `Counters.CUP` written inside
   namespace `Infeed` (6.9.4 Example 1) finds the global `Counters`;
   whether an `Infeed.Counters` would come first is not said.
6. **Variables.** `Symbolic_Variable` takes namespaces in front
   (`N.x`), but a namespace holds no variables: `Namespace_Elements`
   has no `Global_Var_Decls`.
7. **What a global-level USING reaches**: "the enclosing namespace ...
   might be the global namespace", so either the file it is written in
   or the whole project.
8. **A namespace and an element of the same name** side by side
   (`NAMESPACE Motor` and `FUNCTION_BLOCK Motor`) is not addressed.
   Siemens forbids it ("Do not give names to program elements that you
   already use as a namespace"), and CODESYS forbids a namespace named
   as a data type.

## 4. What iecf builds

- **Parser**: `Item::Namespace { name, internal, using, items }`,
  nested, and `Item::Using` at a file's top; `INTERNAL` (or `PUBLIC`)
  before a POU's name, read as a specifier only there, so both stay
  names elsewhere; `USING` where §1 and §6 say. Dotted names in a type
  and a CONFIGURATION's program type; a call's `a.b.c` stays a dotted
  expression, told from a variable by looking its first part up. Ed.3
  gives a type no place for INTERNAL, so iecf's types are public.
- **Before the checker** (`crates/iecf-sema/src/namespaces.rs`): every
  element known by its path; each element taken out of its namespace
  and named by that path, its parts joined by `__`
  (`Standard__Timers__TON`), which no IEC name can be (the lexer now
  refuses two underscores together inside a name, Ed.3 6.1.2); each
  type name, called function, enumerated value's type and program type
  rewritten to the element it means, in the order of D2, access checked
  as Figure 29. New errors: E0028 (not in the namespace, not visible
  from here, or a namespace where a type, function or variable is
  meant), E0029 (a short name two USINGs bring in). The checker and
  the generator see no namespace at all, and changed in nothing but
  showing the path with dots in a message, the symbol table and
  `<p>_pou_names`.
- **C**: a namespaced element's C name is its path upper-cased, each
  part joined by `__`: `B_STANDARD__TIMERS__TON_t`. An IEC identifier
  cannot hold two underscores in a row (Ed.3 6.1.2), so no other name
  can collide with it. The symbol table and the fault record's POU
  names give the dotted path, `Standard.Timers.TON`.
- **Tests**: a test for each rule of §6 and each error
  (`namespaces.rs`); the book page `behaviour/namespaces.md`, without
  checked rows, which evaluate expressions and cannot declare a
  namespace; harness cases `namespaces`, over two files (the harness
  now takes a case's other files from `cases/<name>.d/`), and
  `globalns` for D4, both with traces worked by hand. The reference
  compiler has no namespaces, so both are iecf's only
  (`reference-skip`).

Prokop needs nothing changed: namespaces are optional, and a project
without one compiles as today. The flat namespace of tags and
`globals.gvl` (Prokop's spec/io-tags.md §8) is a different thing:
Ed.3's namespaces hold no variables. A `GVL.x` qualifier, like
`qualified_only`, would be an extension of its own, later.

The cost was smaller than a REF_TO's: one commit, the lookup most of
it.

## 5. Not in this step

- `VAR PUBLIC` / `VAR INTERNAL` on a function block's variables and
  `METHOD INTERNAL` (Table 64 rows 3 and 4) come with classes and
  methods, which iecf does not have yet.
- USING inside a METHOD, CLASS or INTERFACE: the same.

## 6. Decisions

All eight DECIDED 2026-11-09 (florin: "ok, with the recommendations").

**D1. Programs in a namespace: allowed.** 6.9.2's text and
`Prog_Type_Access` both want them; only the element list leaves them
out. A CONFIGURATION names one by its full path.

**D2. Which one a short name means**, as C#, innermost first:

1. the POU's own variables, then the namespace elements its USINGs
   bring in;
2. the elements of the namespace the POU is in, then those its USINGs
   bring in;
3. the same for each parent namespace, outwards;
4. the global namespace, then the file's global-level USINGs.

At each step, a namespace's own element hides one brought in by
USING; two USINGs of one step bringing in the same name is an error
only where the name is used (E0029), as C#. The standard functions and
function blocks are in the global namespace (6.9.1), so a namespace's
own TON hides the standard one inside it, as 6.9.2 Example 2 means
("defines an implementation of TON with a new name").

**D3. A dotted name's first part is found by D2** (C#): inside
`Infeed`, `Counters.CUP` is `Infeed.Counters.CUP` if there is one, the
global `Counters.CUP` otherwise. Siemens does the same ("searches
relative to the calling block").

**D4. The global namespace written `_.`, as an extension.** As Siemens
and PLCnext write it: `_.TON` is the standard timer from inside a
namespace that has its own. Ed.3's `PUBLIC.TON` cannot be parsed
(PUBLIC is a keyword) and is accepted by no vendor. As every iecf
extension it is opt-in (docs/extensions.md): `--extension
global-namespace`. Without it, the standard TON cannot be reached from
a namespace with its own TON, as Ed.3 leaves it.

**D5. USING in a PROGRAM: allowed**, after its name, as in a function
block: the text says "a POU", and a PROGRAM is one.

**D6. A global-level USING is the file's**, as in C#: a project of
several files brings in what each file says. Prokop builds one file
today, so for Prokop it is the project's.

**D7. No variables in a namespace; `N.x` refused** where `N` is a
namespace (E0028). The grammar's `(Namespace_Name '.')+` before a
variable has nothing to name: a namespace holds no VAR_GLOBAL.

**D8. A namespace and an element of the same name in the same place: an
error**, as Siemens and CODESYS have it. Otherwise `Motor.Start`
could be read two ways.
