# Classes and methods

Ed.3 (6.6.5 to 6.6.7) gives a CLASS, and a FUNCTION_BLOCK, **methods**:
operations on an instance's variables, called on the instance. iecf
compiles them as [the proposal](../design/objects.md) says, its
decisions D1 to D12 florin's. This page is step A: methods and classes,
each call bound when compiling. Inheritance (EXTENDS, OVERRIDE, SUPER,
ABSTRACT) and interfaces come with steps B and C; until then they are
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
  (and, with step B, the types derived from it); PRIVATE in the type
  only; INTERNAL in its namespace. **Without a specifier a method or VAR
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

## What an instance is not

An instance of a CLASS, or of a FUNCTION_BLOCK with methods, is **not
copied** with `:=`, is **not an input or output**, and is **not a
variable of a FUNCTION or method**: it is passed as a VAR_IN_OUT (D11,
[E0031](../errors/E0031.md)). With inheritance, a copy into a base-typed
variable would cut the derived part away.

## In C

A CLASS is a structure `B_<NAME>_t` with its `_init`, as a
FUNCTION_BLOCK, without a body function. A method is
`M_<TYPE>__<METHOD>(ctx, self, ...)`, its parameters as a FUNCTION's;
`inst.m(...)` passes `&inst` ([The C output](../c-output.md)). A fault
in a method stops the statement that called it, which stores nothing;
the fault record names it `TYPE.METHOD`.

Not yet: an instance of a CLASS or FUNCTION_BLOCK in an ARRAY or a
STRUCT (as for any block, E0009), and a method's inputs, outputs or
result of an array, structure or STRING type (as a FUNCTION's).
