# Starts and restarts: RETAIN

Ed.3 6.5.6 has two ways to start: a **cold start**, which sets every
variable to its initial value, and a **warm start**, which keeps the
retentive ones. The platform chooses: `<p>_init` is a cold start,
`<p>_warm` a warm one ([The C output](../c-output.md) §3 and §7).

## What a warm start keeps

| Declared | Cold start | Warm start |
|---|---|---|
| `VAR RETAIN`, `VAR_GLOBAL RETAIN` | initial value | **kept** |
| `VAR NON_RETAIN` | initial value | initial value |
| neither | initial value | initial value — Ed.3 leaves it to the implementer; CODESYS, TwinCAT and Siemens set it anew |

## Function blocks

A function block's own declarations decide for its variables; the
section an instance is declared in decides for the rest (Ed.3 6.5.6.2):

| Variable of the block | Instance in a plain section | Instance in `VAR RETAIN` |
|---|---|---|
| in its `VAR RETAIN` | kept | kept |
| in its `VAR NON_RETAIN` | initial value | initial value |
| neither | initial value | kept |
| a block inside it | its own type and section decide, not the outer instance | the same |

An instance of a block that keeps anything — or that contains a block
that does — lives whole in the retained region, `ctx->retain`, as on
CODESYS; at a warm start iecf sets the parts it does not keep.

An array or a structure of instances keeps as its instances do, each
by the table above. In `VAR RETAIN` its other parts are kept too, and
a standard block in it (a timer, a counter) keeps its state. A global
instance follows its `VAR_GLOBAL` section the same way.

## What the platform does

The platform keeps the bytes of `ctx->retain` across a restart — in a
file, FRAM, flash — and hands them back before `<p>_warm`. Their size
and place are `sizeof(<p>_retain_t)` and `offsetof(<p>_t, retain)`; the
symbol table marks each retained variable `IECF_SYM_RETAIN`, so a
platform can also move them by name to a changed program.
