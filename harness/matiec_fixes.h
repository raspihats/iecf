/* matiec_fixes.h - included ahead of every C file of matiec's output in
 * the harness (cc -include). Two gaps between matiec 7949c0b's code
 * generator and its own lib/C, found and fixed the same way in Prokop
 * (toolchain/matiec_overrides.h):
 *
 * - for `t + T#1ms` iec2c writes `_____time_add`, five underscores, and
 *   lib/C defines `__time_add`; nothing defines the long name.
 * - lib/C declares `iec_lib_sqrt` and the others and leaves their
 *   definitions to the platform; matiec_host.c defines them over libm.
 */
#define _____time_add __time_add
#define _____time_sub __time_sub
#define _____time_mul __time_mul
#define _____time_div __time_div
