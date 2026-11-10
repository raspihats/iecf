# Object orientation: the proposal

Ed.3 6.6.5 to 6.6.8 (Tables 48 to 53) adds classes, methods,
interfaces, inheritance and polymorphism, for a new POU, `CLASS`, and
for function blocks. iecf parses none of it today: `CLASS`,
`INTERFACE`, `METHOD`, `EXTENDS` and `IMPLEMENTS` are skipped with
E0009. This proposes how to build them, in three steps (§4), and asks
for the decisions the standard leaves open or gets wrong (§6). Nothing
is built until they are decided.

## 1. What Ed.3 says

**Classes** (6.6.5.1 to 6.6.5.3, Tables 48 and 49)

- `CLASS [FINAL | ABSTRACT] name [EXTENDS base] [IMPLEMENTS i1, i2]`,
  then `VAR` and `VAR_EXTERNAL` sections, then methods, then
  `END_CLASS`. A class has no body, no VAR_INPUT, VAR_OUTPUT,
  VAR_IN_OUT or VAR_TEMP; it is never called, only its methods are.
- Its variables may be RETAIN or NON_RETAIN, initialized, and `AT %I*`.
  Each VAR section has an access specifier, `VAR PUBLIC`: PUBLIC,
  PRIVATE, INTERNAL or PROTECTED, **PROTECTED by default** (Table 48
  10a-10d). A PUBLIC variable is read from outside as an output is,
  `inst.v`.
- An instance is declared as a structured variable is, its PUBLIC
  variables initialized by a list: `c2 : CCounter := (m_iUpperLimit :=
  20000, ...)` (Table 49). Instances of classes and function blocks may
  be declared in a class's VAR and VAR_EXTERNAL.

**Methods** (6.6.5.4)

- `METHOD [access] [FINAL | ABSTRACT] [OVERRIDE] name [: result]`,
  its VAR_INPUT, VAR_OUTPUT, VAR_IN_OUT, VAR and VAR_TEMP (VAR and
  VAR_TEMP mean the same: temporary), a body, `END_METHOD`. Listed
  after a class's variables; in a function block, between its
  declarations and its body.
- **Every method variable and the result are temporary**, "like the
  variables of a function": nothing is kept from one call to the next.
  Reading a method's output outside its call (`y := inst.outm1`) or
  setting its input (`inst.inm1 := a`) is an error (6.6.5.4.7).
- A method reads and writes its instance's variables and externals; in
  a function block, its inputs (not R_EDGE or F_EDGE ones), outputs,
  VARs and externals, but **not its VAR_TEMP or VAR_IN_OUT**
  (6.6.7.2.3). A method's variable may not have a name the class
  already uses.
- A method with a result is called in an expression, one without in a
  statement only; formal or not, EN/ENO as a function's (Table 50).
  Overloading is out of scope.
- A method of the same instance is called `THIS.m(...)`; of another,
  `inst.m(...)` (6.6.5.4.4).
- **Access**: PUBLIC anywhere the class can be used; PRIVATE only in
  the class itself; PROTECTED in the class and its derivations;
  INTERNAL in its namespace. **PROTECTED by default** (6.6.5.4.3 rule
  5, 6.6.5.9). "All improper uses shall be treated as errors."

**Inheritance** (6.6.5.5 to 6.6.5.8)

- `EXTENDS`: one base only. A class extends a class; a function block
  extends a function block **or a class**; a class never extends a
  function block (Table 48 row 6, Table 53 rows 7a, 7b).
- The derived type inherits every variable and every method but the
  PRIVATE ones (and INTERNAL ones outside the namespace). Declaring a
  variable its base has (unless PRIVATE there), a method named as a
  base variable, or deriving from itself is an error.
- `OVERRIDE` replaces a base method: same signature (name, result,
  every parameter's name, type and order), same access; FINAL may be
  added. OVERRIDE where no base method is is an error.
- `FINAL` on a method: not overridden; on a type: not a base.
- `ABSTRACT` on a type: never instantiated, holds at least one
  abstract method; an abstract method is implemented by every
  non-abstract derived type. Not with OVERRIDE; only in an abstract
  type. An abstract type may be the type of an input or in-out.
- **Dynamic binding** (6.6.5.6): a call through `THIS`, an in-out, a
  reference or an interface runs the method of the instance's actual
  type. 6.6.5.6's example: CIRCLE's CF calls `THIS.PI()`; in a
  CIRCLE2, which overrides PI, CF gives the more accurate value.
- `SUPER.m()` calls the base's m, bound statically. Not
  `inst.SUPER.m`, not `SUPER.SUPER.m`.
- `THIS` is a reference to the own instance; it may be given to an
  interface variable (`ctrl(RM := THIS)`).

**Function blocks** (6.6.7)

- A function block may have methods, a body, or both; with methods
  only, "this function block can also be declared as a class".
- **The body is not inherited**: a derived block's body is its own,
  empty if it has none. `SUPER()` in it runs the base's body, "once
  ... and shall not be in a loop", never in a method (6.6.7.2.9).
- A call of a block through a base-typed in-out or reference runs the
  derived block's body: "the call of the function block shall be bound
  dynamically".
- Inputs and outputs are always PUBLIC, outputs read-only from
  outside; VAR_EXTERNAL is always PROTECTED; a block's VAR is
  PROTECTED by default, as a class's.

**Interfaces** (6.6.6, Tables 51 and 52)

- `INTERFACE name [EXTENDS i1, i2]`, method prototypes (a name, a
  result, VAR_INPUT, VAR_OUTPUT, VAR_IN_OUT; no body, no access
  specifier: always PUBLIC), `END_INTERFACE`. An interface extends any
  number of interfaces; redeclaring a base's prototype, or deriving
  from itself, is an error.
- A class or block `IMPLEMENTS` one or more interfaces, and implements
  each prototype with the same signature, PUBLIC or INTERNAL; one that
  leaves any out is ABSTRACT.
- **An interface is the type of a variable**: a reference to an
  instance of a type implementing it, or NULL, its initial value.
  It may be given an instance, an instance of a derived type, another
  interface variable of the same or a derived interface, or NULL;
  compared with `=` to another, equal when the same instance or both
  NULL. Calling a method of a NULL one "a runtime error will occur".
  **Not an in-out** (6.6.6.2).
- **Assignment attempt** `a ?= b` (6.6.6.7): `a` gets `b` if what `b`
  references is of a type `a` can hold, NULL otherwise. Between
  interfaces, from an interface to a reference to a type, and from a
  reference to a base type to a reference to a derived one (downcast).

**Polymorphism** (6.6.8): through an interface, a VAR_IN_OUT, a
reference, and THIS. A derived block passed to a base-typed in-out or
reference must add no in-out of its own (it would be unset when the
base's call runs its body); whether it may add inputs or outputs is
"Implementer specific" (6.6.8.3, 6.6.8.4; Table 53 rows 12a-12d).

**Grammar** (Annex A): `Class_Decl`, `Method_Decl`, `Interface_Decl`,
`Method_Prototype`, `Access_Spec`; `THIS.x` in `Symbolic_Variable`;
`Invocation` takes `m(...)`, `THIS(...)`, `THIS.m(...)` and
`a.b.m(...)`; `SUPER()` a statement; `Assignment_Attempt`;
`Interface_Spec_Init`.

## 2. What the vendors do

| | CLASS | THIS, SUPER | method access, none written | OVERRIDE | NULL interface | downcast |
|---|---|---|---|---|---|---|
| **CODESYS V3.5** | none: methods, EXTENDS and IMPLEMENTS on function blocks | `THIS^.x`, `SUPER^.m()` | PUBLIC | none found | 0, "methods can be called only after" assignment | `__QUERYINTERFACE`, `__QUERYPOINTER` |
| **TwinCAT 3** | none, as CODESYS | `THIS^`, `SUPER^`; `SUPER^()` "Call of FB-body of base class" | PUBLIC ("equivalent to specifying no access modifier") | none found | 0; check "<> 0" first | `__QUERYINTERFACE` |
| **Siemens SIMATIC AX** | yes, `CLASS ABSTRACT`, IMPLEMENTS a list | `THIS.x`, as Ed.3 | not documented (pages not readable) | `METHOD PUBLIC OVERRIDE`, `FINAL OVERRIDE` | compared with NULL | not found |
| **PLCnext Engineer** | none: methods of function blocks only, no inheritance or interfaces | `THIS.m()` | none: "an access specifier must be specified", Public or Private only | - | - | - |
| **RuSTy** | yes; no body, THIS rejected in one | `THIS^`, `SUPER^` | specifiers parsed and ignored | optional | fat pointer, NULL unchecked (a crash) | none |
| **Siemens S7-1500 SCL** | no OO documented by Siemens | | | | | |

- **Dynamic binding.** CODESYS and TwinCAT dispatch a call through a
  pointer, a reference, an interface, a base-typed VAR_IN_OUT, and a
  method's call of a method of its own block. RuSTy the same, through
  a vtable; `SUPER^.m()` and a call on a named instance are direct.
- **A method's variables** are reinitialized at every call (CODESYS,
  TwinCAT, RuSTy), as Ed.3. CODESYS and TwinCAT add `VAR_INST`, a
  method variable kept in the instance. PLCnext keeps a method's array
  or structure result between calls, which Ed.3 does not.
- **A derived block's body**: RuSTy runs its own only, `SUPER^();` for
  the base's, as Ed.3. TwinCAT has `SUPER^()`; neither it nor CODESYS
  says whether the base's body runs without one.
- **ABSTRACT** on function blocks and their methods: CODESYS and
  TwinCAT (build 4024); RuSTy parses it and an abstract method "returns
  the default value". **FINAL** on blocks and methods: TwinCAT.
- **Interfaces** extend several interfaces (CODESYS, TwinCAT, RuSTy),
  hold function block instances (CODESYS, TwinCAT; they have no
  classes) or both (RuSTy). RuSTy refuses an interface as VAR_IN_OUT
  (E126), as Ed.3.
- **Initialization methods** FB_init, FB_reinit, FB_exit (CODESYS,
  TwinCAT; RuSTy a parameterless FB_INIT). Ed.3 has none: an instance
  starts at its declared initial values.
- **What none documents**: Ed.3's PROTECTED default, `?=`, and what
  calling a method through a NULL interface does.

Sources: content.helpme-codesys.com (CODESYS Development System:
_cds_obj_method, _cds_obj_interface, _cds_extending_function_block,
_cds_implementing_interface, _cds_method_call, _cds_pointer_this,
_cds_pointer_super, _cds_vartypes_var_inst, _cds_operator_queryinterface,
_cds_operator_querypointer, _cds_method_fb_init_fb_reinit);
infosys.beckhoff.com (tc3_plc_intro: methods 2530307467, calls
2527348875, SUPER 2528837771, function block specifiers 2530268811,
interface extension 3537333899, ABSTRACT 6413748235, interface pointer
4256428299 and 5680748299, `__QUERYINTERFACE` 2529176459, FB_init
5482088715); engineer.plcnext.help (2026.6, POU_Method) and plcnext.help
(PLCnext Engineer features); Siemens' open-source SIMATIC AX library
github.com/simatic-ax/statemachine (the AX documentation is behind a
login); RuSTy at 11e8107 (book/user/language/inheritance.md,
methods-and-properties.md, book/technical/participants/03-polymorphism.md,
tests/lit/single/polymorphism). Siemens' and PLCnext's OO for classic
SCL was not found.

## 3. Where Ed.3 contradicts itself or is silent

1. **Is an access specifier required?** `Method_Decl` requires one
   (`'METHOD' Access_Spec ...`); 6.6.5.4.3 says the declaration "shall
   contain one" and in the next sentence that without one it is
   PROTECTED; 6.6.5.9 says PROTECTED "may be omitted".
2. **The order of the specifiers.** The grammar has access, then FINAL
   or ABSTRACT, then OVERRIDE; the examples write `METHOD OVERRIDE mb`
   and `METHOD PUBLIC OVERRIDE DAYTIME`, and 6.6.5.10 lets a VAR
   section's specifiers come "in any order".
3. **A method of the own instance without THIS.** 6.6.5.4.3 rule 13
   and 6.6.5.4.4 say it "shall be called using the THIS keyword";
   `Invocation` takes a bare `Method_Name(...)`. A bare `m()` could
   also be a FUNCTION of that name (Table 48 row 14 only says "should
   not").
4. **`THIS.x` or `THIS^.x`.** THIS is "a reference to the own class
   instance", and a reference is dereferenced with `^` (6.4.4.10); yet
   every example writes `THIS.x`, `THIS.m()`, never `THIS^`.
5. **`SUPER.m()` is not in the grammar.** `Invocation` has `THIS.` and
   instance names, `Subprog_Ctrl_Stmt` only `SUPER()`; the text and its
   examples use `SUPER.DAYTIME()`.
6. **A method's result in an expression is not in the grammar.**
   `Primary_Expr` has `Func_Call` (a function's name only); the text
   says a method with a result "shall be called as an operand of an
   expression" (`VALUE := CT.UP(INC := 5)`).
7. **A method of the base's name without OVERRIDE.** Only OVERRIDE
   without a base method is listed as an error; a derived method that
   has a base method's name and no OVERRIDE is not addressed (an error,
   a hidden base method, or an override anyway).
8. **What a function block's body is when the derived one has none.**
   6.6.7.2.9: "not automatically inherited ... It is empty by default";
   Figure 21's right column shows each derived block as if it had the
   base's variables written again, which rule 3 ("names ... shall be
   unique") forbids.
9. **Additional inputs and outputs of a derived block** behind a base
   in-out or reference: "Implementer specific" (6.6.8.3, 6.6.8.4),
   Table 53 offering both.
10. **An interface as an in-out.** 6.6.6.2: "Interfaces shall not be
    used as in-out variables"; 6.6.6.5.3's explanation passes an
    instance to one "as function block input, output, in-out, or
    result".
11. **What an interface references.** 6.6.6.2 and 6.6.6.5: "instances
    of classes"; Table 51 row 4c and Table 53 row 6c: "a function block
    instance"; 6.6.6.7: both.
12. **An interface method implemented INTERNAL** (6.6.6.4.2 error 3
    allows PUBLIC or INTERNAL), though a prototype is "always PUBLIC":
    called through the interface from outside the namespace, an
    INTERNAL method is reached where its specifier forbids it.
13. **What the runtime error is**, for a NULL interface's method call,
    and for `?=` there is none (it gives NULL). 6.6.6.5.2 suggests a
    "dummy" method as one way.
14. **Copying an instance**, `a := b` of a class or an OO block, is not
    addressed; nor whether an instance may be an input (by value) or a
    function's variable.
15. **6.6.8.4's only example shows no dynamic binding**: its
    ROOM_CTRL sets `RM^.DAYTIME` and never calls the block `RM`
    references, so neither body runs.
16. **An abstract type as an input.** 6.6.5.8.2: "An abstract class
    may be used as a type of an input or in-out parameter"; an input is
    passed by value, and an abstract class "cannot be instantiated".
17. **Mistakes in the examples**: Table 49's initialization list
    separates with `;` (Struct_Init uses `,`); Table 50 calls
    `COUNTER.UP` on the type and passes a `START` UP does not have;
    6.6.6.7.3 Example 3 declares `pinstBase2` and uses `rinstBase2`,
    and uses an `ITF3` never declared; 6.6.8.3 Example 2 and 6.6.8.4
    Example 1 write `VAR_INPUT: DAYTIME`; a `SUPER.NIGHTTIME()` without
    its `;`; `END_VAR;` and `END_PROGRAM;`. None changes a rule.

## 4. What iecf builds

In three steps, each a commit with its tests and book page, each
usable alone. What Ed.3 states plainly is built as written; where it
does not, as §6 decides.

**Step A: methods and classes, bound statically.**

- **Parser**: `Item::Class`, `Method` in a class and a function block
  (between its declarations and its body), access specifiers on a
  method and on a VAR section (`VAR PUBLIC`, `VAR PRIVATE RETAIN`),
  `THIS.x`, `THIS.m(...)`, `inst.m(...)` in a statement and in an
  expression. PUBLIC, PRIVATE, PROTECTED, INTERNAL, FINAL, ABSTRACT,
  OVERRIDE, THIS and SUPER are keywords of Ed.3 (Table 48), so no
  longer names.
- **Checker**: a class's sections (only VAR and VAR_EXTERNAL); a
  method's signature, its result only in an expression and a method
  without one only as a statement (6.6.5.4.5); a method's output read
  or its input set outside its call (6.6.5.4.7); a method's names
  apart from its instance's (rule 10); in a function block, no VAR_TEMP
  or VAR_IN_OUT of the block in a method (6.6.7.2.3); access, for
  methods and variables, from where the use is: the type, a derived
  type, the namespace; a class instance never called. New errors for
  each (E0030 on).
- **C**: a class is a function block without a body, `B_<NAME>_t` and
  `B_<NAME>_init`, so a block may extend one. A method is a C function
  `M_<TYPE>__<METHOD>(ctx, B_<TYPE>_t *self, ...)`, its parameters as
  a FUNCTION's (inputs by value, outputs and in-outs by pointer, EN/ENO
  as a FUNCTION's), its VAR and VAR_TEMP C locals set at each call, its
  result returned. `inst.m()` and `THIS.m()` call it directly: in step
  A no type has a derived one.
- **Symbols**: a class instance's variables, as a block's.

**Step B: inheritance and dynamic binding.**

- **Checker**: EXTENDS (one base; a class from a class, a block from a
  block or a class), the inherited variables and methods but PRIVATE
  ones, OVERRIDE's signature and access, FINAL, ABSTRACT (never
  instantiated, its abstract methods all implemented below), SUPER,
  `SUPER()` once in a block's body and not in a loop, the errors of
  6.6.5.5.5; a derived instance given to a base-typed VAR_IN_OUT or
  REF_TO (§6 D7).
- **C**: a derived type's structure starts with its base's,
  `B_<BASE>_t base;`, so a pointer to it is a pointer to its base (C99
  6.7.2.1); a base's variable is `self->base.V_X`. A type with no base
  starts with `const iecf_type_t *type`, which each `_init` sets to
  its own type: the instance knows what it is, for the whole run. An
  `iecf_type_t` (runtime/iecf.h) holds its base, its name, its table
  of methods and its interfaces' tables. The table is a C structure of
  function pointers, the body first for a block, then each method that
  can be overridden; a derived type's starts with its base's.
- **Which call is bound how**: through `THIS`, a VAR_IN_OUT, a
  reference, or a block's call through either, from the instance's
  table (6.6.5.6, 6.6.7.2.9 rule 4); `SUPER.m()`, `SUPER()` and a call
  on a named instance directly, its type known. A FINAL method, or one
  no type overrides, directly too (6.6.8.5 NOTE).
- **A NULL reference** is checked before the call, as `r^` is: a fault
  NULL, the call not made, the statement stopped (§6 D8).

**Step C: interfaces.**

- **Checker**: INTERFACE and its EXTENDS (any number, no prototype
  declared twice, no cycle), IMPLEMENTS (every prototype, same
  signature, PUBLIC or INTERNAL, or the type ABSTRACT), an interface
  variable's values (6.6.6.5.1) and `=`/`<>`, `?=` (6.6.6.7).
- **C**: an interface variable is `iecf_itf_t { void *self; const void
  *table; }`, NULL when `self` is. Given a named instance, its table is
  known (`&I_<ITF>__<TYPE>`); given THIS or another interface, it is
  found at run time in the instance's type (`iecf_itf_find`, walking
  the type and its bases). `?=` looks the same way: an interface for
  an interface, `iecf_type_is` for a reference to a derived type.
  Interfaces never reach the process image; neither do instances.
- **A method called through a NULL interface**: as a NULL reference
  (D8).

**Tests, each step**: a test for each rule and each error; the book
page `behaviour/objects.md`, its checked rows where an expression can
show it (a method's result, 6.6.5.6's CIRCLE and CIRCLE2); harness
cases with traces worked by hand: `methods` (A), `inherit` (B: CIRCLE,
LIGHT2ROOM with SUPER, Figure 21's SUPER() chain, a base-typed in-out
and reference), `interfaces` (C: ROOM_CTRL, `?=` as 6.6.6.7's
examples). The reference compiler has no OO, so all are iecf's only
(`reference-skip`).

Prokop needs nothing changed: a project without OO compiles as today,
and the Workbench gets the new errors live. The cost is the largest
yet: step A about a namespace's (a parser, checker and generator each
touched), B and C each about as much again, B the most (the tables, the
initializers, the dispatch).

## 5. Not in this step

- **CODESYS's spelling**: `THIS^`, `SUPER^`, overriding without
  OVERRIDE, PUBLIC when no specifier is written. CODESYS, TwinCAT and
  RuSTy write OO so; Ed.3 does not. As an opt-in extension later
  (docs/extensions.md), if Prokop users bring CODESYS code.
- **VAR_INST** (CODESYS, TwinCAT): a method variable kept in the
  instance. Not Ed.3.
- **FB_init, FB_reinit, FB_exit** (CODESYS, TwinCAT) and
  `__QUERYINTERFACE`, `__QUERYPOINTER`: not Ed.3; `?=` does the second's
  work.
- **Properties** (`PROPERTY`, GET and SET): Ed.4, with the step 4
  additions.
- **Overloading** of methods: "not in the scope" of Ed.3 (6.6.5.4.3
  NOTE 1).
- **Methods in LD or FBD**: iecf compiles ST and SFC.

## 6. Decisions

**D1. A method or VAR section without an access specifier is
PROTECTED**, as Ed.3 says three times (6.6.5.4.3 rule 5, 6.6.5.9,
Table 48 10d). CODESYS and TwinCAT make it PUBLIC; PLCnext requires
one; no vendor documents PROTECTED. Calling a PROTECTED method from
outside is an error whose message says to write `METHOD PUBLIC`. The
other way: PUBLIC, as CODESYS, which would make code written for
CODESYS compile unchanged and code written for Ed.3 more open than its
author meant.

**D2. The specifiers in any order**, each at most once: `METHOD
OVERRIDE PUBLIC m` and `METHOD PUBLIC OVERRIDE m` both, as the examples
write them and as 6.6.5.10 allows for a VAR section, though the grammar
gives one order.

**D3. A method of the own instance is called `THIS.m()`**; a bare
`m()` inside a method or body is a FUNCTION's call, an error if there
is none, the message saying to write `THIS.m()`. As 6.6.5.4.3 rule 13
and 6.6.5.4.4 say, though `Invocation` allows the bare form; it keeps
a method and a FUNCTION of one name apart (Table 48 row 14). The other
way: a bare `m()` is the method if the type has one, as RuSTy and (by
their examples) CODESYS and TwinCAT.

**D4. `THIS.x` and `SUPER.m()`, as Ed.3 and SIMATIC AX write them**,
though THIS is called a reference; `THIS^` refused, until the CODESYS
spelling (§5).

**D5. `SUPER.m()` and a method's result in an expression, though the
grammar has neither**: the text and every example use them, and
without them SUPER and results have no use.

**D6. A method with a base method's name and no OVERRIDE is an error**,
the message saying to write OVERRIDE: Ed.3 asks OVERRIDE of every
override, and without the error the method would override silently or
hide its base's, whichever iecf chose. CODESYS has no OVERRIDE; RuSTy
does not require it.

**D7. A derived block behind a base-typed in-out or reference may add
inputs and outputs, not in-outs.** The added inputs keep the values
they have (the call through the base sets only the base's); an added
in-out is an error where the instance is given or its reference taken
(6.6.8.3, 6.6.8.4, which leave the first "Implementer specific", the
second an error). Table 53 rows 12b and 12d.

**D8. A method or block called through a NULL reference or interface is
a fault `NULL`**: not called, the statement stopped, as `r^` of a NULL
reference (decided 2026-11-08). Ed.3 says "a runtime error"; CODESYS
and TwinCAT say only to check first; RuSTy crashes.

**D9. An interface is not an in-out, and references class and block
instances both**: 6.6.6.2 forbids the in-out, as RuSTy (E126);
6.6.6.7 and Table 51 row 4c take block instances, as CODESYS and
TwinCAT, whose interfaces hold nothing else.

**D10. An interface's method may be implemented INTERNAL**, as
6.6.6.4.2 allows, and is then reachable through the interface from
anywhere: the interface's prototypes are PUBLIC. The other way:
PUBLIC only.

**D11. An instance of a class, or of a block with methods, a base or
an interface, is not copied with `:=`, not an input and not a
FUNCTION's or method's variable.** A copy into a base-typed variable
would cut the derived part and keep the derived type's table; an
input is a copy. An abstract type is then an in-out, a reference or an
interface only, which reads 6.6.5.8.2's "input" as the in-out it can
only be. A plain block keeps what it has today.

**D12. An ABSTRACT method has no statements**: its body would never
run. RuSTy runs one, giving the result's default.
