# Implementer extensions

IEC 61131-3 Ed.3 clause 5 c asks an implementation for "a document that
specifies all Implementer specific extensions. These are any features
accepted by the system that are prohibited or not specified." This is
that document for iecf.

By default iecf accepts none: it checks the standard as written. Each
extension is switched on by name, for one run:

```
iecf check --extension implicit-externals FILE...
```

A program that needs an extension is not portable to another compliant
tool as it stands; the section of each extension says what to change to
make it so.

## implicit-externals: a POU uses a global variable without declaring it VAR_EXTERNAL

**The standard.** Ed.3 6.5.2.2: global variables are "only accessible
to a program organization unit via a VAR_EXTERNAL declaration", with
the global's type, and `CONSTANT` if the global is. Without the
extension, a global used undeclared is E0010.

**The extension.** A name a POU does not declare, which a `VAR_GLOBAL`
of the configuration, of a resource or of a global variable list does,
is that global, as if the POU had declared it
`VAR_EXTERNAL` (`VAR_EXTERNAL CONSTANT` for a constant global). The
POU's own declarations come first: a local of the same name hides the
global. Everything else about the global is checked as usual: its
type, and that a constant is not assigned to.

**Why.** Some tools and editors keep a project's globals in one list and
let every POU use them. A platform that wants that may switch this on,
or have its editor write the `VAR_EXTERNAL` declarations where the user
sees them; iecf supports both and decides neither.

**To make a program portable.** Add to each POU a `VAR_EXTERNAL` block
naming every global it uses, with the global's type.

## var-config-list: VAR_CONFIG outside a CONFIGURATION, paths from the PROGRAM's name

**The standard.** Ed.3 6.8.2 and Annex A (`Config_Decl`): `VAR_CONFIG`
is part of a `CONFIGURATION`, after its resources; a path starts with
the resource (if the configuration names one), then the program
instance. Without the extension, a `VAR_CONFIG` at the top of a file is
E0025.

**The extension.** As CODESYS writes it: `VAR_CONFIG ... END_VAR` at the
top of any file, in as many blocks as wanted. Each PROGRAM is its own
one instance, under its own name, and a path starts there:

```
VAR_CONFIG
  PLC_PRG.valve1.coil AT %QX0.0 : BOOL;
END_VAR
```

Everything else is checked as for the standard form: the path, the
repeated type, the address's area, the initial values, and every `*`
address of every PROGRAM completed (E0023). A project that has a
`CONFIGURATION` keeps its `VAR_CONFIG` there; a list of its own is then
still E0025.

**Why.** CODESYS projects, and those of the systems built on CODESYS,
complete their `AT %I*` addresses this way: there is no textual
`CONFIGURATION`, tasks call PROGRAMs by name. (Beckhoff TwinCAT has no
`VAR_CONFIG` at all; it links `%I*` variables in its I/O configuration,
outside the source text, which iecf does not read: PROPOSAL.md §2b.)

**To make a program portable.** Write a `CONFIGURATION` with a resource,
its tasks and a program instance per PROGRAM, and move the entries into
its `VAR_CONFIG`, each path prefixed with the resource's name.
