# Compliance statement

IEC 61131-3 Ed.3 clause 5.3 asks an implementation for an "Implementer's
compliance statement": the features of the standard's tables it
provides, each by table and feature number and the language it is
provided in, in the form of the standard's Figure 4. This is iecf's.
With [Extensions](extensions.md) (clause 5 c), [Implementer
choices](dependencies.md) (clause 5 d) and [Diagnostics](errors.md)
(clause 5 e) it is what clause 5 asks of a compliant system.

| | |
|---|---|
| Implementer | the iecf project, <https://github.com/raspihats/iecf> |
| Product | iecf 0.0.1, a compiler of IEC 61131-3 text to portable C ([The C output](c-output.md)) |
| Controller type | any whose C compiler takes C99 or later; the platform runs the C, schedules its tasks and owns its I/O |
| Standard | IEC 61131-3:2013 (Ed.3) |
| Date | 2026-11-18 |

This product complies with the requirements of the standard for the
following language features. The languages are **ST** (Structured
Text) and **SFC** in its textual form (Ed.3 Annex A); iecf implements
no LD, FBD or IL ([below](#not-implemented)). Each feature listed is
built: checked, and compiled to C. Where the standard leaves a choice
to the implementer, the note links to it. Without an extension switched
on, iecf accepts nothing the standard does not define
([Extensions](extensions.md#what-is-an-extension)).

## Common elements (Ed.3 clause 6)

| Table | Features (ST) | Implementer's note |
|---|---|---|
| 1 – Character set | 1, 2a, 2b, 2c | The source text is UTF-8; a name's letters are ASCII. A STRING holds Latin-1, a WSTRING UTF-16 ([STRING](behaviour/strings.md)) |
| 2 – Identifiers | 1, 2, 3 | Every character of a name is significant |
| 3 – Comments | 1, 2a, 2b, 3a, 3b | |
| 4 – Pragma | 1 | Read and ignored: no pragma means anything to iecf |
| 5 – Numeric literals | 1, 2, 3, 4, 5, 6, 7, 8, 9 | An octal literal is warned (W0001): Ed.4 removed it. A real has its point; `1E37` without one only with `--extension real-without-point` ([Extensions](extensions.md#real-without-point-a-real-literal-with-an-exponent-and-no-point-1e37)) |
| 6 – Character string literals | 1a–1g, 2a–2h, 3a, 3b, 4a, 4b, 4c, 4d | |
| 7 – Two-character combinations | 1–8 | |
| 8 – Duration literals | 1a–1g, 2a, 2b, 3a, 3b | A TIME counts whole milliseconds, an LTIME nanoseconds; a literal finer than its type is E0020, as the table's own `t#5d_14h_12m_18s_3.5ms` is for a TIME (`LT#` takes it) ([TIME and LTIME](behaviour/time.md)) |
| 9 – Date and time of day literals | 1a–6b | DATE, TOD and DT from 1970 to 2106, TOD in ms and DT in s ([dates](behaviour/dates.md)) |
| 10 – Elementary data types | 1–21 | Ranges and resolutions: [Implementer choices](dependencies.md#the-time-types-ranges-and-resolution-ed3-642-table-10); a STRING is at most 65535 characters |
| 11 – User-defined data types | 1a–12 | `STRUCT OVERLAP` needs relative locations (E0034). As the grammar writes them: `END_STRUCT;` and an array's values in brackets; CODESYS's forms without them only with `--extension struct-semicolon` and `bare-array-init` ([Extensions](extensions.md)) |
| 12 – Reference operations | 1, 2a, 2b, 2c, 3a, 3b, 4 | Dereferencing NULL faults ([References](behaviour/references.md)) |
| 13 – Declaration of variables | 1, 2, 3, 4 | |
| 14 – Initialization of variables | 1–6 | |
| 15 – Variable-length ARRAY | 1, 2a, 2b | |
| 16 – Directly represented variables | 1–11 | A hierarchical address is resolved by the platform ([The C output](c-output.md)) |
| 17 – Partial access of ANY_BIT | 1a–4 | |
| 18 – EN and ENO | 1, 2, 3, 4 | In ST, as formal parameters ([EN and ENO](behaviour/en-eno.md)) |
| 19 – Function declaration | 1a–3c | |
| 20 – Function call | 1a, 1b, 2, 3 | |
| 21 – Typed and overloaded functions | 1a, 1b, 2a, 2b | The typed forms: [Typed functions](behaviour/typed-functions.md) |
| 22 – Data type conversion function | 1a, 1b, 2a, 2b, 2c, 3a, 3b, 4a, 4b | TRUNC without a type is warned (W0003, deprecated) |
| 23 – Conversion of numeric types | all | [Conversions](behaviour/conversions.md) |
| 24 – Conversion of bit types | all | |
| 25 – Conversion of bit and numeric types | all but 4, 8, 12, 16 | |
| 26 – Conversion of date and time types | all | |
| 27 – Conversion of character types | 1–8 | |
| 28 – Numerical functions | 1–12 | From the target's `<math.h>` or the platform's own |
| 29 – Arithmetic functions | 1–7 | An overflow wraps and is flagged, a division by zero faults ([Results, flags and faults](behaviour/overview.md)) |
| 30 – Bit shift functions | 1–4 | |
| 31 – Bitwise Boolean functions | 1–4 | |
| 32 – Selection functions | 1–6 | SEL, MUX and MOVE of user-defined types too (footnote d) |
| 33 – Comparison functions | 1–6 | |
| 34 – Character string functions | 1–9 | LEN and FIND give INT |
| 35 – Functions of time and duration types | 1a–11c | |
| 36 – CONCAT and SPLIT of dates and times | 1a–8 | DAY_OF_WEEK gives USINT |
| 37 – Endianness conversion | 1–4 | The PLC's order is the target's |
| 38 – Functions of enumerated types | 1, 2, 3a, 4a | |
| 39 – Validate functions | 1, 2 | IS_VALID of REAL and LREAL |

## Program organization units (Ed.3 6.6)

| Table | Features (ST) | Implementer's note |
|---|---|---|
| 40 – Function block type declaration | 1–6b | |
| 41 – Function block instance declaration | 1, 2 | |
| 42 – Function block call | 1, 2, 6a, 7, 8a, 8b, 9a, 10a, 11a | |
| 43 – Bistable function blocks | 1a, 1b, 2a, 2b | |
| 44 – Edge detection | 1, 2 | |
| 45 – Counters | 1a–3e | PVmax and PVmin are the type's range |
| 46 – Timers | 1a, 1b, 1c, 2a, 2b, 2c, 3a, 3b, 3c | They time against the task's clock, which the platform gives |
| 47 – Program declaration | 1–6b, 8a, 8b, 9 | |
| 48 – Class | 1–11b | |
| 49 – Class instance declaration | 1, 2 | |
| 50 – Textual call of methods | 1a, 1b, 2 | |
| 51 – Interface | 1, 2, 3, 4a, 4b, 4c | |
| 52 – Assignment attempt | 1, 2 | |
| 53 – Object oriented function block | 1–12d | |

## Sequential function chart (Ed.3 6.7), textual

| Table | Features (SFC) | Implementer's note |
|---|---|---|
| 54 – SFC step | 2a, 2b, 3a, 4a | |
| 55 – Transition and transition condition | 7b, 13 | A condition in ST |
| 56 – Declaration of actions | 1, 2s, 3s | An action in ST, or an SFC of its own |
| 57 – Step/action association | 3 | |
| 58 – Action block | 1a, 2, 3b, 4s | In a textual step body. An indicator variable (3b, deprecated) is read and warned (W0007), and no code is written for it |
| 59 – Action qualifiers | 1–12 | |
| 60 – Action control | 1 | With final scan |
| 61 – Sequence evolution | 1, 2a, 2b, 2c, 3, 4a, 4b, 4c, 4d, 5a–5c, 6a–6c | Written as textual transitions; 2c is evaluated as 2a, the exclusion being the user's ([SFC](behaviour/sfc.md)) |

## Configuration elements and namespaces (Ed.3 6.8, 6.9)

| Table | Features (ST) | Implementer's note |
|---|---|---|
| 62 – Configuration and resource | 1, 2, 3, 4, 5a, 5b, 6a, 6c, 7, 8a, 8b, 9a, 9b, 10a–10i, 11a, 11b, 12a, 12b | VAR_ACCESS is checked and written to the symbol table; how it is reached is the platform's (IEC 61131-5) |
| 63 – Task | 1a, 1b, 3a | iecf schedules nothing: the platform runs each task, preemptive or not (5a, 5b), from the task table ([The C output](c-output.md)) |
| 64 – Namespace | 1a, 1b, 2, 3, 4, 5 | |
| 65 – Nested namespace declarations | 1, 2, 3 | |
| 66 – USING | 1, 2, 3 | |

## Textual languages (Ed.3 clause 7)

| Table | Features (ST) | Implementer's note |
|---|---|---|
| 71 – Operators of ST | 1–18 | |
| 72 – ST statements | 1, 1a–1d, 2a, 2b, 2c, 3–11 | |

## Not implemented

| Table | Features | Why |
|---|---|---|
| 20, 40, 42, 46, 47 | 20: 4, 5, 6; 40: 7a, 7b; 42: 3, 4, 5a, 5b, 6b, 9b, 10b, 11b; 46: 2d, 3d; 47: 7a, 7b | Graphical: iecf reads text |
| 25 | 4, 8, 12, 16 (BYTE_TO_BOOL and the others) | Figure 11 defines no such conversion, and Table 25 does: iecf follows Figure 11 (E0013). `--extension bit-string-to-bool` takes them with another meaning ([Extensions](extensions.md#bit-string-to-bool-byte-word-dword-and-lword-to-bool-true-for-any-value-but-0)) |
| 54–58, 61 | 54: 1a, 1b, 3b; 55: 1a–6a, 8b, 9a, 10, 11, 12; 56: 2l, 2f, 3i; 57: 1, 2, 4a; 58: 4i, 4l, 4f, 5l, 5f; 61: 7 | Graphical SFC, and transitions and actions in LD, FBD or IL |
| 60 | 2 | An implementation has one; iecf's is with final scan |
| 62 | 6b | A function block instance on a task of its own: refused ([Implementer choices](dependencies.md#a-function-block-instance-on-a-task-of-its-own-ed3-682)) |
| 62 | 13a, 13b | VAR_EXTERNAL in a RESOURCE: Annex A gives it no place (Resource_Decl) |
| 63 | 2a, 2b, 3b, 4a, 4b | Graphical, and 3b a function block on a task (62, 6b) |
| 67–70 | all | IL, removed in Ed.4: not read (E0008) |
| 73–76 | all | LD and FBD |

What iecf does not compile yet, though its checker reads it, is
reported as E0009 ([E0009](errors/E0009.md)).
