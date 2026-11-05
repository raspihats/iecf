# Implementer specific dependencies

IEC 61131-3 Ed.3 clause 5 d asks an implementation for "a document that
specifies all Implementer specific dependencies": the choices the
standard leaves open, and its limits. This is that document for iecf,
growing as the compiler does.

Each choice is made after checking what the big vendors do (CODESYS and
the systems built on it, Beckhoff TwinCAT, Siemens), and says so: a
program that compiles there should, as far as the standard lets it,
mean the same here.

## Untyped literals (Ed.3 6.6.1.6, rule 8)

"The Implementer shall define the rules for non-typed literals."

- An integer literal without a type (`5`, `16#FF`) takes the type the
  place it is used needs, if its value fits: any integer or bit-string
  type, and also REAL or LREAL. `SQRT(2)` is `SQRT(2.0)`; `r := 2` is
  `r := 2.0`. A value that does not fit is E0020.
- A real literal without a type (`2.5`) goes only where REAL or LREAL
  goes.
- Between two untyped literals, an operation keeps them untyped until a
  typed operand or the target decides.

Vendors: CODESYS documents `SQRT` as taking "any numeric base type" and
gives `q := SQRT(16)` as its example. It goes further and takes integer
*variables* too; iecf does not: an INT variable where ANY_REAL is
required is E0014 (convert it, `INT_TO_REAL`).

## LEN and FIND give INT (Ed.3 Table 34)

The standard gives their result as ANY_INT. iecf gives INT.

Vendors: INT in the CODESYS Standard library, in Beckhoff's
Tc2_Standard, and in Siemens S7-SCL (FC21, FC11). (CODESYS's separate
StrFunc library has a FIND of its own that gives UDINT.)

## DAY_OF_WEEK gives USINT (Ed.3 Table 36)

The standard gives the result as ANY_INT, 0 for Sunday to 6 for
Saturday, and leaves the type to the implementer (NOTE 2). iecf gives
USINT, the type of the standard's own example (`myDoW: USINT`).

Vendors: none of those checked has the standard's DAY_OF_WEEK; each has
a function of its own (Beckhoff `F_GetDayOfWeek` giving WORD, CODESYS
Util `DayOfWeek` giving an enumeration), so none sets a precedent.

## SPLIT_DATE, SPLIT_TOD, SPLIT_DT outputs (Ed.3 Table 36)

The outputs are ANY_INT and "the Implementer specifies the provided data
types" (NOTE 2). iecf takes any integer variable for each.
