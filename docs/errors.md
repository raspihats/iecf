# Diagnostics

Every diagnostic iecf reports has a code: `E` for an error, which stops a
build, `W` for a warning, which does not. `iecf explain CODE` prints the
code's page; each is here too.

E0009 is not a fault in the program: it marks something the checker
accepts and the C generator does not cover yet.

| Code | What |
|---|---|
| [E0001](errors/E0001.md) | a character that has no place in IEC 61131-3 text |
| [E0002](errors/E0002.md) | a comment that is never closed |
| [E0003](errors/E0003.md) | a string literal that is never closed |
| [E0004](errors/E0004.md) | a literal that is not well formed |
| [E0005](errors/E0005.md) | the text does not fit the grammar here |
| [E0006](errors/E0006.md) | a block whose END keyword never comes |
| [E0007](errors/E0007.md) | a `$` escape that a string literal does not have |
| [E0008](errors/E0008.md) | Instruction List, removed from IEC 61131-3 in Ed.4 |
| [E0009](errors/E0009.md) | a part of IEC 61131-3 that iecf does not compile yet |
| [E0010](errors/E0010.md) | a name that nothing declares |
| [E0011](errors/E0011.md) | a type that nothing declares |
| [E0012](errors/E0012.md) | a name declared twice |
| [E0013](errors/E0013.md) | a value whose type does not convert implicitly |
| [E0014](errors/E0014.md) | operands an operator or index does not take |
| [E0015](errors/E0015.md) | an assignment to what cannot be assigned |
| [E0016](errors/E0016.md) | a call that does not fit what it calls |
| [E0017](errors/E0017.md) | a VAR_EXTERNAL that matches no VAR_GLOBAL |
| [E0018](errors/E0018.md) | a name the standard reserves for a type, function or block |
| [E0019](errors/E0019.md) | a condition that is not BOOL |
| [E0020](errors/E0020.md) | a literal outside its type's range |
| [E0021](errors/E0021.md) | EXIT or CONTINUE outside a loop |
| [E0022](errors/E0022.md) | an address where the standard does not allow one |
| [E0023](errors/E0023.md) | a `*` address that VAR_CONFIG does not complete |
| [E0024](errors/E0024.md) | a VAR_CONFIG entry that does not fit the variable it names |
| [E0025](errors/E0025.md) | a VAR_CONFIG outside a CONFIGURATION |
| [E0026](errors/E0026.md) | REF() of what cannot be referenced |
| [E0027](errors/E0027.md) | a reference where it cannot be |
| [E0028](errors/E0028.md) | a name a namespace does not have, or does not let this code see |
| [E0029](errors/E0029.md) | a short name that two USINGs both bring in |
| [W0001](errors/W0001.md) | an octal literal, removed from IEC 61131-3 in Ed.4 |
| [W0002](errors/W0002.md) | a direct address in a body, deprecated since Ed.3 |
| [W0003](errors/W0003.md) | TRUNC without a type, deprecated since Ed.3 |
| [W0004](errors/W0004.md) | an address whose size prefix is not its variable's size |
| [W0005](errors/W0005.md) | REF() of a variable at an input address |
