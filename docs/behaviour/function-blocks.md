# Standard function blocks

The bistables, edge detectors, counters and timers of Ed.3 Tables 43 to
46, with what the standard leaves to the implementer.

## Inputs and outputs that are arrays, structures or strings

A FUNCTION_BLOCK's VAR_INPUT or VAR_OUTPUT may be an array, a structure
or a STRING. A call **copies** an input in, `f(xs := a)`, and an output
out, `f(hist => b)`; each source is found or computed before any input
is set, so a call whose arguments fault changes nothing. A STRING too
long for the input or the variable it is taken into is cut and flagged
`STRING_LENGTH`, as any string store. From outside, an output reads as
the instance's member, `f.hist[0]`, `f.text`.

## Instances: initial values, arrays, structures, globals

```
TYPE Cooler : STRUCT
    temp : INT;
    cooling : TOF := (PT := T#100ms);       (* Ed.3 Table 11 7a *)
END_STRUCT; END_TYPE

VAR
    t : TON := (PT := T#500ms);
    c : Count := (inc := 2, limit := 100);   (* an input, a PUBLIC variable *)
    tons : ARRAY[1..50] OF TON := [50(PT := T#100ms)];   (* Table 11 5b *)
    line : ARRAY[1..4] OF Cooler;
END_VAR

tons[i](IN := start);
line[2].cooling(IN := hot);
```

- An instance's **initial value** names its inputs, outputs and PUBLIC
  variables (Ed.3 6.6.3.3, and 6.6.5.3 for a class); the others keep
  their type's own. `[50(PT := T#100ms)]` repeats one, as Table 11
  writes it; the grammar's `50((PT := T#100ms))` is the same.
- An instance is **an array's element or a structure's field** as any
  value is, and is called, read and given there: `tons[i](...)`,
  `tons[i].Q`, `line[2].cooling(...)`, a method `accs[i].Add(...)`, an
  in-out or an interface given `tons[i]` (Figure 8 d, e). Its index is
  checked first: out of bounds is an INDEX fault, and the statement
  stops before anything runs. An element is an instance of exactly its
  type, so a call on it is bound statically.
- A **global instance**, in the CONFIGURATION's or a resource's
  VAR_GLOBAL, is reached by VAR_EXTERNAL from a program or a block.
- An array or structure that holds an instance is not copied with `:=`
  (not compiled yet, E0009, as an instance's copy is not); one that
  holds a class's instance, or a block's with methods, a base or an
  interface, is not copied nor an input or output at all (E0031). A
  restart keeps them as [it keeps their instances](restarts.md).

## Bistables and edges

SR is set-dominant, RS reset-dominant (Table 43). R_TRIG and F_TRIG see
an edge between one call and the next: a rising edge at the first call
counts when CLK is already TRUE, as the standard's ST body has it
(`M` starts FALSE).

## Counters count over their type's range

CTU and CTUD count up while CV is below PVmax, CTD and CTUD down while
CV is above PVmin, and the standard leaves both to the implementer
(Table 45). In iecf they are the limits of CV's own type: a CTU (CV an
INT) stops at 32767, a CTD at -32768, a CTD_UDINT at 0. **A counter
never wraps.** Q compares CV with PV as the table says, whatever PV is.

Siemens counts the same way, to the range limit of the type. CODESYS
and TwinCAT give CV the type WORD, so their CTD stops at 0; Rockwell's
CTUD wraps. matiec stops CTU at PV and CTD at 0, which the standard does
not say.

| Counter | Type of CV | Counts up to | Counts down to |
|---|---|---|---|
| CTU, CTD, CTUD | INT | 32767 | -32768 |
| CTU_DINT, CTD_DINT, CTUD_DINT | DINT | 2147483647 | -2147483648 |
| CTU_LINT, CTD_LINT, CTUD_LINT | LINT | 2⁶³-1 | -2⁶³ |
| CTU_UDINT, CTD_UDINT, CTUD_UDINT | UDINT | 4294967295 | 0 |
| CTU_ULINT, CTD_ULINT, CTUD_ULINT | ULINT | 2⁶⁴-1 | 0 |

## Timers

TP, TON and TOF (Table 46) time against the task's clock,
`ctx->now_ns`, which the platform sets for each run of a task: a timer
sees time pass between scans, not during one. ET counts from 0 to PT
and stops there. A negative PT counts as 0: the timer's output follows
its input at once. The LTIME forms (TP_LTIME, TON_LTIME, TOF_LTIME) time
in nanoseconds.
