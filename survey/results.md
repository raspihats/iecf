# The last survey

2026-11-07, Prokop e61378c9, iecf (every variable, array element, field and block member compared), matiec 7949c0b.

What does not build: **IL** (`il_div`, `il_mod`), not iecf's - Ed.4
without IL (PROPOSAL.md §6.1).

Of those both build, each that does not run the same, and why - each
where iecf decided differently, or matiec's side cannot run it:

- **c583e73f52a4**, Prokop's SFC demo: iecf runs the actions before the
  transitions, with a final scan (docs/dependencies.md, SFC).
- **7d80e41fddc8**: `dout0 AT %QX0.0` and `pv AT %QW0` overlap; iecf's
  image is one memory, matiec keeps a variable per address.
- **d779cf508890**: its test expects matiec's text, `'16#ff'` and
  `'TOD#12:34:56.500000'`; iecf writes `'255'` and `'TOD#12:34:56.500'`
  (docs/dependencies.md, conversions to and from STRING).
- **de13ecfb543e**: divides by zero; iecf faults, matiec goes on.
- **255308ff121c**: divides by zero; matiec's C dies of SIGFPE.
- **64523234ff8c**: declares one address twice, which the harness's
  matiec host cannot hold.

55 programs; 51 matiec builds; 50 of those iecf builds; 44 of those run the same, every variable compared

| program | iecf | first thing it stops at, or how it runs beside matiec |
|---|---|---|
| 0510d587a215 | E0005 | an expression on its own is not a statement: assign it with ':=', or call a function or block |
| 179bdd6fe9b3 | matiec fails | 179bdd6fe9b3.st:7: error: invalid specification in global variable declaration. |
| 1ce81a4aa3fc | matiec fails | 1ce81a4aa3fc.st:4: error: invalid variable(s) declaration. |
| 248d0c3dc6e5 | matiec fails | 248d0c3dc6e5.st:4: error: invalid expression after ':=' in ST assignment statement. |
| 2c61deb63dc2 | matiec fails | 2c61deb63dc2.st:4: error: invalid expression after ':=' in ST assignment statement. |
| 7d80e41fddc8 | builds | differs at tick 0: matiec `RES0.INSTANCE0.DOUT0 0`, iecf `RES0.INSTANCE0.DOUT0 1` |
| d779cf508890 | builds | differs at tick 0: matiec `RES0.INSTANCE0.OK_DWORD 1`, iecf `RES0.INSTANCE0.OK_DWORD 0` |
| c583e73f52a4 | builds | differs at tick 4: matiec `4 Q 01 01 00 M`, iecf `4 Q 00 00 00 M` |
| de13ecfb543e | builds | iecf faults at tick 0: `0 Q M fault 1 RATIO 12` |
| 64523234ff8c | builds | matiec fails: cc |
| 255308ff121c | builds | matiec fails: the host (signal: 8 (SIGFPE) (core dumped)) |
| 230769e20509 | builds | same (40 ticks, 1 variables) |
| 3ad71432cf5c | builds | same (40 ticks, 1 variables) |
| 18c8c52b215e | builds | same (40 ticks, 10 variables) |
| 5d9cc678a7bb | builds | same (40 ticks, 10 variables) |
| 1e6bcc1d3c5a | builds | same (40 ticks, 11 variables) |
| 2a0e80d6c30c | builds | same (40 ticks, 11 variables) |
| bfb5e586cc47 | builds | same (40 ticks, 11 variables) |
| 982a05cc407f | builds | same (40 ticks, 15 variables) |
| 0c84b3c7c398 | builds | same (40 ticks, 2 variables) |
| 0ef1d1027ab4 | builds | same (40 ticks, 2 variables) |
| 497b4eace0ba | builds | same (40 ticks, 2 variables) |
| 74423056b107 | builds | same (40 ticks, 2 variables) |
| 7833ba53e5f5 | builds | same (40 ticks, 2 variables) |
| 794d4d8bf3d7 | builds | same (40 ticks, 2 variables) |
| a6b137a4922c | builds | same (40 ticks, 2 variables) |
| ad18002b9ab6 | builds | same (40 ticks, 2 variables) |
| e083218e28b5 | builds | same (40 ticks, 2 variables) |
| e2e500751e1a | builds | same (40 ticks, 2 variables) |
| fb5e1709002a | builds | same (40 ticks, 2 variables) |
| a08a5c3a375e | builds | same (40 ticks, 23 variables) |
| 5ea551f0ca8c | builds | same (40 ticks, 24 variables) |
| 7ceadb39147f | builds | same (40 ticks, 29 variables) |
| 03a46577ef84 | builds | same (40 ticks, 32 variables) |
| 34cd70a978b2 | builds | same (40 ticks, 4 variables) |
| 3e9f8d035dfc | builds | same (40 ticks, 4 variables) |
| 464b42244ca3 | builds | same (40 ticks, 4 variables) |
| 74940fbd8c9d | builds | same (40 ticks, 4 variables) |
| 91702e819e6c | builds | same (40 ticks, 4 variables) |
| a938c7dd8ee2 | builds | same (40 ticks, 4 variables) |
| f1251103eeda | builds | same (40 ticks, 4 variables) |
| 2f3e6e7f563a | builds | same (40 ticks, 5 variables) |
| adc5e97901d9 | builds | same (40 ticks, 5 variables) |
| aea623321632 | builds | same (40 ticks, 5 variables) |
| 51ec13ab65f5 | builds | same (40 ticks, 6 variables) |
| f4297b60aff7 | builds | same (40 ticks, 6 variables) |
| ff2742d2d583 | builds | same (40 ticks, 6 variables) |
| 8b5aba97b8e3 | builds | same (40 ticks, 69 variables) |
| 4fc729c06cf9 | builds | same (40 ticks, 7 variables) |
| 51fc2c5b2de6 | builds | same (40 ticks, 7 variables) |
| a2a5a26b53e9 | builds | same (40 ticks, 7 variables) |
| d3f6f71ca368 | builds | same (40 ticks, 7 variables) |
| f751e9830fc3 | builds | same (40 ticks, 8 variables) |
| c0169b32e6e3 | builds | same (40 ticks, 9 variables) |
| 33310c6a515a | builds | same (40 ticks, 99 variables) |
