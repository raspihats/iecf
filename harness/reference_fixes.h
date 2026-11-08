/* reference_fixes.h - included ahead of every C file of the reference compiler's output in
 * the harness (cc -include). Two gaps between the reference compiler 7949c0b's code
 * generator and its own lib/C, found and fixed the same way in Prokop's
 * toolchain:
 *
 * - for `t + T#1ms` iec2c writes `_____time_add`, five underscores, and
 *   lib/C defines `__time_add`; nothing defines the long name.
 * - lib/C declares `iec_lib_sqrt` and the others and leaves their
 *   definitions to the platform; reference_host.c defines them over libm.
 */
#define _____time_add __time_add
#define _____time_sub __time_sub
#define _____time_mul __time_mul
#define _____time_div __time_div
