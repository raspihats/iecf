# The last survey

2026-11-07, Prokop e61378c9, iecf 0173eda, matiec 7949c0b.

What stops the five that do not build yet:

- **IL** (`il_div`, `il_mod`): not iecf's - Ed.4 without IL (PROPOSAL.md §6.1).
- **TIME * and / by a number** (Ed.3 Table 30, MUL and DIV on TIME): two,
  one after EN and ENO.
- **EN and ENO on a function** (`DIV(EN := TRUE, ..., ENO => ok)`): one.
- **An array as a FUNCTION_BLOCK's input or output**: one.
- **SFC**: one.

55 programs; 51 matiec builds; 46 of those iecf builds

| program | iecf | first thing it stops at |
|---|---|---|
| 0510d587a215 | E0005 | an expression on its own is not a statement: assign it with ':=', or call a function or block |
| 03a46577ef84 | E0009 | EN, ENO or outputs of a standard function is not compiled to C yet |
| c583e73f52a4 | E0009 | SFC is not compiled to C yet |
| 5d9cc678a7bb | E0009 | TIME * and / on a duration is not compiled to C yet |
| 33310c6a515a | E0009 | a FUNCTION_BLOCK parameter of this type is not compiled to C yet |
| 179bdd6fe9b3 | matiec fails | 179bdd6fe9b3.st:7: error: invalid specification in global variable declaration. |
| 1ce81a4aa3fc | matiec fails | 1ce81a4aa3fc.st:4: error: invalid variable(s) declaration. |
| 248d0c3dc6e5 | matiec fails | 248d0c3dc6e5.st:4: error: invalid expression after ':=' in ST assignment statement. |
| 2c61deb63dc2 | matiec fails | 2c61deb63dc2.st:4: error: invalid expression after ':=' in ST assignment statement. |
| 0c84b3c7c398 | builds |  |
| 0ef1d1027ab4 | builds |  |
| 18c8c52b215e | builds |  |
| 1e6bcc1d3c5a | builds |  |
| 230769e20509 | builds |  |
| 255308ff121c | builds |  |
| 2a0e80d6c30c | builds |  |
| 2f3e6e7f563a | builds |  |
| 34cd70a978b2 | builds |  |
| 3ad71432cf5c | builds |  |
| 3e9f8d035dfc | builds |  |
| 464b42244ca3 | builds |  |
| 497b4eace0ba | builds |  |
| 4fc729c06cf9 | builds |  |
| 51ec13ab65f5 | builds |  |
| 51fc2c5b2de6 | builds |  |
| 5ea551f0ca8c | builds |  |
| 64523234ff8c | builds |  |
| 74423056b107 | builds |  |
| 74940fbd8c9d | builds |  |
| 7833ba53e5f5 | builds |  |
| 794d4d8bf3d7 | builds |  |
| 7ceadb39147f | builds |  |
| 7d80e41fddc8 | builds |  |
| 8b5aba97b8e3 | builds |  |
| 91702e819e6c | builds |  |
| 982a05cc407f | builds |  |
| a08a5c3a375e | builds |  |
| a2a5a26b53e9 | builds |  |
| a6b137a4922c | builds |  |
| a938c7dd8ee2 | builds |  |
| ad18002b9ab6 | builds |  |
| adc5e97901d9 | builds |  |
| aea623321632 | builds |  |
| bfb5e586cc47 | builds |  |
| c0169b32e6e3 | builds |  |
| d3f6f71ca368 | builds |  |
| d779cf508890 | builds |  |
| de13ecfb543e | builds |  |
| e083218e28b5 | builds |  |
| e2e500751e1a | builds |  |
| f1251103eeda | builds |  |
| f4297b60aff7 | builds |  |
| f751e9830fc3 | builds |  |
| fb5e1709002a | builds |  |
| ff2742d2d583 | builds |  |
