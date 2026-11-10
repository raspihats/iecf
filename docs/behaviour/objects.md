# Classes and methods

Ed.3 (6.6.5 to 6.6.7) gives a CLASS, and a FUNCTION_BLOCK, **methods**:
operations on an instance's variables, called on the instance. iecf
compiles them as [the proposal](../design/objects.md) says, its
decisions D1 to D12 florin's. This page is steps A and B: methods and
classes, and inheritance with its dynamic binding. Interfaces
(INTERFACE, IMPLEMENTS, `?=`) come with step C; until then they are
E0009.

## A class

```
CLASS Counter
    VAR count : UINT; END_VAR              (* PROTECTED: no specifier *)
    VAR PUBLIC limit : UINT := 10; END_VAR (* read and written outside *)

    METHOD PUBLIC Up : UINT                (* a result, as a FUNCTION's *)
        VAR_INPUT by_ : UINT := 1; END_VAR
        VAR_OUTPUT full : BOOL; END_VAR
        IF count < limit THEN
            count := count + by_;
        END_IF;
        full := count >= limit;
        Up := count;
    END_METHOD

    METHOD PUBLIC Reset                    (* no result: a statement *)
        THIS.count := 0;
    END_METHOD
END_CLASS
```

- A CLASS has **variables and methods, no body**: only `VAR` and
  `VAR_EXTERNAL` sections; its methods have the inputs and outputs. An
  instance is never called, only its methods are (6.6.5.2).
- A method is **a function on its instance**: its inputs, outputs,
  in-outs and result as a FUNCTION's, its VAR and VAR_TEMP temporary,
  set at each call (6.6.5.4.3). It reads and writes the instance's
  variables by their names, or as `THIS.count`.
- `inst.m(...)` calls a method of another instance, `THIS.m(...)` one of
  its own; a bare `m()` is a FUNCTION's call, as Ed.3 says (D3). A
  method with a result goes in an expression or a statement; one
  without, in a statement only. Formal or in order, with EN and ENO, as
  a FUNCTION's call.
- **Access** (6.6.5.9, 6.6.5.10): PUBLIC anywhere; PROTECTED in the type
  and the types derived from it; PRIVATE in the type only; INTERNAL in
  its namespace. **Without a specifier a method or VAR
  section is PROTECTED**, as Ed.3 says (D1); CODESYS and TwinCAT make it
  PUBLIC, so code written for them needs `PUBLIC` written
  ([E0030](../errors/E0030.md)).
- Specifiers come in any order: `METHOD PUBLIC FINAL m` or `METHOD
  FINAL PUBLIC m` (D2). They are words only there: a variable may still
  be named `final`. THIS and SUPER are reserved.

```
VAR c : Counter; n : UINT; full : BOOL; END_VAR

n := c.Up(by_ := 2, full => full);
c.limit := 20;
IF c.Up() > 15 THEN
    c.Reset();
END_IF;
```

## A function block with methods

A FUNCTION_BLOCK may have methods too, written after its declarations
and before its body (6.6.7.2). They reach its inputs, outputs, VARs and
externals, not its VAR_TEMP, VAR_IN_OUT or R_EDGE and F_EDGE inputs
(6.6.7.2.3). Its body may call them with `THIS.m()`. Its inputs may be
set from outside before a call (`lamp.sw := TRUE;`), as Ed.3 Figure 20
shows.

## Inheritance

```
CLASS CIRCLE
    METHOD PUBLIC PI : REAL
        PI := 3.0;
    END_METHOD
    METHOD PUBLIC CF : REAL
        VAR_INPUT d : REAL; END_VAR
        CF := THIS.PI() * d;           (* the instance's own PI *)
    END_METHOD
END_CLASS

CLASS CIRCLE2 EXTENDS CIRCLE
    METHOD PUBLIC OVERRIDE PI : REAL
        PI := 3.14;
    END_METHOD
END_CLASS
```

- `EXTENDS` names **one base** (6.6.5.5.2): a CLASS extends a CLASS, a
  FUNCTION_BLOCK a FUNCTION_BLOCK or a CLASS. The derived type has
  every variable and method of its base but the PRIVATE ones, and
  declares none of the same names (6.6.5.5.5).
- `OVERRIDE` replaces a base method, with the same result, inputs,
  outputs, in-outs and access (6.6.5.5.3). **A method with a base
  method's name and no OVERRIDE is an error** (D6), which says to write
  it; CODESYS, which has no OVERRIDE, overrides silently. `FINAL` on a
  method: not overridden; on a type: not a base.
- **Dynamic binding** (6.6.5.6): a call through `THIS`, a VAR_IN_OUT or
  a reference runs the method of the instance's own type. Above,
  `c2.CF(d := 100.0)` of a CIRCLE2 is 314, its CF calling CIRCLE2's
  PI; `c1.CF(d := 100.0)` of a CIRCLE is 300. A call on a named
  instance, and `SUPER.m()`, the base's method, are bound when
  compiling.
- A FUNCTION_BLOCK's **body is not inherited** (6.6.7.2.9): a derived
  block runs its own, empty if it has none; `SUPER()` in it runs its
  base's, once and not in a loop, never in a method. A block called
  through a base-typed in-out or reference runs the body of the
  instance's type.
- A derived instance stands where its base is wanted: given to a
  base-typed VAR_IN_OUT, of a block, FUNCTION or method, or referenced
  by a base-typed `REF_TO`. Its type may add inputs and outputs, which
  keep their values through a call by the base, **not in-outs**, which
  the base's call would leave unbound (D7).
- An `ABSTRACT` type has no instances and holds an ABSTRACT method, one
  with no statements (D12); the types derived from it implement each
  one or are ABSTRACT too (6.6.5.8). It is the type of a VAR_IN_OUT or
  a `REF_TO`, given an instance of a type derived from it.
- A method called through a **NULL reference** is not called: the
  fault is NULL, and the statement stops, as `r^` of one (D8).

```
FUNCTION_BLOCK BASE
    VAR_INPUT a : INT; END_VAR
    VAR_OUTPUT x : INT; END_VAR
    x := a + 1;
END_FUNCTION_BLOCK

FUNCTION_BLOCK DERIVED_1 EXTENDS BASE
    VAR_INPUT b : INT; END_VAR
    SUPER();                           (* BASE's body: x := a + 1 *)
    x := 3 * x + b;
END_FUNCTION_BLOCK
```

## What an instance is not

An instance of a CLASS, or of a FUNCTION_BLOCK with methods, a base or
a derived type, is **not copied** with `:=`, is **not an input or
output**, and is **not a variable of a FUNCTION or method**: it is
passed as a VAR_IN_OUT (D11, [E0031](../errors/E0031.md)). A copy into
a base-typed variable would cut the derived part away.

## In C

A CLASS is a structure `B_<NAME>_t` with its `_init`, as a
FUNCTION_BLOCK, without a body function. A method is
`M_<TYPE>__<METHOD>(ctx, self, ...)`, its parameters as a FUNCTION's;
`inst.m(...)` passes `&inst` ([The C output](../c-output.md)). A fault
in a method stops the statement that called it, which stores nothing;
the fault record names it `TYPE.METHOD`. A derived type's structure
starts with its base's; each instance in a hierarchy points to its
type's table of methods, from which a call through THIS, an in-out or a
reference takes the function to run.

Not yet: an instance of a CLASS or FUNCTION_BLOCK in an ARRAY or a
STRUCT (as for any block, E0009), and a method's inputs, outputs or
result of an array, structure or STRING type (as a FUNCTION's).
