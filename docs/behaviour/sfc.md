# SFC

A PROGRAM or FUNCTION_BLOCK may be a Sequential Function Chart (Ed.3
6.7): steps, transitions between them, and actions associated with the
steps. iecf compiles the textual form — `STEP`, `TRANSITION`, `ACTION` —
which is also what a graphical editor writes. A POU is either a chart or
statements, not both ([E0005](../errors/E0005.md)).

## One scan

Each time the POU runs, in this order:

1. **Step times.** Each active step's `T` is the time since it was
   entered, in whole milliseconds, from the task's clock.
2. **Action control.** Each action's qualifier logic — the standard's
   ACTION_CONTROL — is evaluated with the steps active now: its Q (the
   action flag) and whether it runs (A).
3. **Actions.** A BOOL variable used as an action takes Q. Action bodies
   run in two passes, as CODESYS and TwinCAT run them: first those that
   are ending (their final scan, a P1 or P0 pulse), then those whose Q
   is TRUE, each pass in source order. An action that is ending cannot
   overwrite what an active one writes.
4. **Transitions.** Against the steps active when the scan began: a
   transition whose preceding steps are all active, and whose condition
   is TRUE, is cleared — its preceding steps left, then its following
   steps entered.

A step entered in this scan runs its actions from the next one: every
step's actions run at least once before it can be left ("the duration
of a step activity can never be considered to be zero", Ed.3 6.7.5).
CODESYS and TwinCAT work the same way; matiec tests the transitions
first. ([Implementer choices](../dependencies.md#sfc-ed3-67))

## Steps

`S.X` is TRUE while step S is active, `S.T` the time since it was
entered; when S is left, `S.T` keeps its last value, and entering it
again starts it at `T#0s`. Neither can be assigned (Ed.3 6.7.2). `S.T`
stops at TIME's largest value, after 24 days. The initial step is active
from the start; its time starts at the first scan.

## Actions and qualifiers

An action runs under the qualifiers of its active associations (Ed.3
Table 59), with **final scan** (Table 60, feature 1): once its Q falls,
it runs one more time with Q FALSE, so it can tidy up. Inside the
action, `ACTION.Q` tells which: `IF NOT Fill.Q THEN valve := FALSE;`.

| Qualifier | The action's Q | Runs |
|---|---|---|
| N (or none) | while the step is active | while Q, and one final scan |
| S | from the step's start, stored after it | until an R, then one final scan |
| R | resets a stored action (S, SD, DS, SL) and overrides N | — |
| L t | from the step's start, for at most t | while Q, and one final scan |
| D t | after t, if the step is still active | while Q, and one final scan |
| P | for one scan, on entry | twice: with Q, then its final scan |
| P1 | never | once, on entry, Q FALSE |
| P0 | never | once, on leaving, Q FALSE |
| SD t | t after the step's start, stored | until an R, then one final scan |
| DS t | after t if the step is still active, then stored | until an R, then one final scan |
| SL t | from the step's start, stored, for at most t | while Q, and one final scan |

**P runs twice.** The standard's action control gives every qualifier
but P1 and P0 its final scan, P too (Ed.3 6.7.4.6, NOTE 3), and CODESYS
documents that its P action is "executed exactly two times". For an
action that should run once on entry — counting entries — use **P1**.

The final scan and P0 follow a falling edge that happened: the
standard's figure draws them with F_TRIG, which would also fire on the
very first scan; no vendor does that, and iecf does not.

## Transitions and their priority

Of the transitions leaving the same active step, the first true one is
cleared: those with `(PRIORITY := n)` by the lowest n (Ed.3 Table 61,
2b), then in source order (2a, as the leftmost branch first). The
others wait. Transitions leaving different steps clear together, so a
simultaneous divergence `TO (S2, S3)` activates both branches, and a
convergence `FROM (S2b, S3)` waits for every one of its steps.

## Errors

Ed.3 asks the system to treat these as errors. iecf flags them
`SFC` (code 11) and goes on:

- **An "unsafe" chart**: a step entered while it is still active (Ed.3
  Figure 26 a). It stays active, once; its time goes on.
- **More than one time-related association active** for one action
  (6.7.4.6 d): the last one's duration is used.
- **SD while the action's SL is stored**, or SL while its SD is (6.7.4.6
  d).

A chart that can lock up ("unreachable", Figure 26 b) is the program's
own logic; nothing at run time can tell it from one that waits.

## Restarts

A cold start, and a warm one, set every chart to its initial step. A
FUNCTION_BLOCK instance kept whole across a warm start ([RETAIN](restarts.md))
keeps its steps, times and action states, as Ed.3 6.7.2 has it for a
retentive instance.

An indicator variable on an association, `act(N, done)` (Ed.3 6.7.4.2,
Table 58 "c"; deprecated), is read and warned of
([W0007](../errors/W0007.md)): it names a BOOL variable the action may
set, and does nothing of its own (the harness case `sfcind`).

A POU may hold several networks (Ed.3 Annex A, `SFC : Sfc_Network +`):
the steps its transitions join make one, each has exactly one
INITIAL_STEP, and all evolve side by side in each scan (the harness
case `sfcnets`).

Not yet: steps and actions in the symbol table, and an ACTION whose
body is itself an SFC (Ed.3 Annex A, `Action : ... FB_Body`), which is
E0009.
