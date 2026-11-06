/* matiec_host.c - runs matiec's C for the harness (harness/README.md).
 *
 * Compiled with the files iec2c wrote for one case and matiec's lib/C.
 * Generic: the case's located variables arrive through matiec's own
 * LOCATED_VARIABLES.h X-macro list, as in matiec/tests/plc.c.
 *
 * matiec keeps each located variable in a variable of its own; iecf reads
 * the process image at each use (docs/c-output.md §4). So the host copies
 * the image into matiec's variables before a cycle and the changed ones
 * back after it, the address numbers counted as bytes and the bytes
 * little-endian, as iecf has them. A program that writes one address and
 * reads an overlapping one in the same cycle sees a difference; the cases
 * do not do that.
 *
 * Reads the cycle script from stdin, as harness/src/main.rs writes it:
 *     ticks N
 *     set TICK OFFSET HEXBYTES     (input bytes, in tick order)
 * and writes the trace to stdout: one line after initialisation and one
 * per tick, the Q and M bytes in hex.
 *
 * Harness code, not iecf output: GNU C, x86-64 or another little-endian host.
 */
#include <math.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "iec_std_lib.h"

/* lib/C leaves these to the platform (matiec_fixes.h). */
double iec_lib_acos(double x) { return acos(x); }
double iec_lib_asin(double x) { return asin(x); }
double iec_lib_atan(double x) { return atan(x); }
double iec_lib_cos(double x) { return cos(x); }
double iec_lib_exp(double x) { return exp(x); }
double iec_lib_fmod(double x, double y) { return fmod(x, y); }
double iec_lib_log(double x) { return log(x); }
double iec_lib_log10(double x) { return log10(x); }
double iec_lib_pow(double x, double y) { return pow(x, y); }
double iec_lib_sin(double x) { return sin(x); }
double iec_lib_sqrt(double x) { return sqrt(x); }
double iec_lib_tan(double x) { return tan(x); }
int iec_lib_snprintf(char *s, size_t n, const char *format, ...) {
    va_list ap;
    int r;
    va_start(ap, format);
    r = vsnprintf(s, n, format, ap);
    va_end(ap);
    return r;
}

#define IMAGE_BYTES 4096
static uint8_t I[IMAGE_BYTES], Q[IMAGE_BYTES], M[IMAGE_BYTES];

TIME __CURRENT_TIME;
extern unsigned long long common_ticktime__;
void config_init__(void);
void config_run__(unsigned long tick);

/* matiec's variables, and a copy taken at copy-in to see which changed. */
#define __LOCATED_VAR(type, name, ...) type name;
typedef struct {
    char none_; /* a case with no located variable */
#include "LOCATED_VARIABLES.h"
} shadow_t;
#undef __LOCATED_VAR
static shadow_t SHADOW, BEFORE;

#define __LOCATED_VAR(type, name, ...) type *name = &SHADOW.name;
#include "LOCATED_VARIABLES.h"
#undef __LOCATED_VAR

static uint8_t *space(char c) {
    return c == 'I' ? I : c == 'Q' ? Q : M;
}

/* One located variable: (I|Q|M, X|B|W|D|L, byte[, bit]). */
static void copy_in(void *var, size_t size, const char *cls, const char *sz, unsigned byte, int bit) {
    uint8_t *p = space(cls[0]);
    if (byte + size > IMAGE_BYTES) {
        fprintf(stderr, "address %%%s%s%u beyond the host's image\n", cls, sz, byte);
        exit(2);
    }
    if (sz[0] == 'X')
        *(uint8_t *)var = (uint8_t)((p[byte] >> bit) & 1u);
    else
        memcpy(var, p + byte, size);
}

static void copy_out(const void *var, const void *before, size_t size, const char *cls, unsigned byte, int bit,
                     const char *sz) {
    uint8_t *p = space(cls[0]);
    if (cls[0] == 'I' || memcmp(var, before, size) == 0) return;
    if (sz[0] == 'X') {
        if (*(const uint8_t *)var)
            p[byte] = (uint8_t)(p[byte] | (1u << bit));
        else
            p[byte] = (uint8_t)(p[byte] & ~(1u << bit));
    } else {
        memcpy(p + byte, var, size);
    }
}

#define BIT_(byte, bit, ...) bit
#define BIT(...) BIT_(__VA_ARGS__, -1, -1)
#define BYTE_(byte, ...) byte
#define BYTE(...) BYTE_(__VA_ARGS__, 0)

static void all_in(void) {
#define __LOCATED_VAR(type, name, cls, sz, ...) \
    copy_in(&SHADOW.name, sizeof SHADOW.name, #cls, #sz, BYTE(__VA_ARGS__), BIT(__VA_ARGS__));
#include "LOCATED_VARIABLES.h"
#undef __LOCATED_VAR
    BEFORE = SHADOW;
}

static void all_out(void) {
#define __LOCATED_VAR(type, name, cls, sz, ...) \
    copy_out(&SHADOW.name, &BEFORE.name, sizeof SHADOW.name, #cls, BYTE(__VA_ARGS__), BIT(__VA_ARGS__), #sz);
#include "LOCATED_VARIABLES.h"
#undef __LOCATED_VAR
}

/* The highest byte the program addresses in Q or M, plus one. */
static unsigned extent(char which) {
    unsigned n = 0, end;
#define __LOCATED_VAR(type, name, cls, sz, ...) \
    end = BYTE(__VA_ARGS__) + (#sz[0] == 'X' ? 1u : (unsigned)sizeof SHADOW.name); \
    if (#cls[0] == which && end > n) n = end;
#include "LOCATED_VARIABLES.h"
#undef __LOCATED_VAR
    (void)end;
    return n;
}

static void trace(const char *label) {
    unsigned q = extent('Q'), m = extent('M'), k;
    printf("%s Q", label);
    for (k = 0; k < q; k++) printf(" %02x", Q[k]);
    printf(" M");
    for (k = 0; k < m; k++) printf(" %02x", M[k]);
    printf("\n");
}

int main(void) {
    unsigned long ticks = 0, tick, at, offset;
    char word[16], hex[2 * IMAGE_BYTES + 1];
    int have = 0;
    const uint16_t one = 1;

    if (*(const uint8_t *)&one != 1) {
        fprintf(stderr, "the host must be little-endian\n");
        return 2;
    }
    if (scanf("%15s %lu", word, &ticks) != 2 || strcmp(word, "ticks") != 0) {
        fprintf(stderr, "script: expected 'ticks N'\n");
        return 2;
    }

    memset(&SHADOW, 0, sizeof SHADOW);
    BEFORE = SHADOW;
    config_init__();
    all_out(); /* initial values of located variables into the image */
    trace("init");

    have = scanf("%15s %lu %lu %8192s", word, &at, &offset, hex) == 4;
    for (tick = 0; tick < ticks; tick++) {
        char label[32];
        while (have && at == tick) {
            size_t k, n = strlen(hex) / 2;
            if (strcmp(word, "set") != 0 || offset + n > IMAGE_BYTES) {
                fprintf(stderr, "script: bad line at tick %lu\n", at);
                return 2;
            }
            for (k = 0; k < n; k++) {
                unsigned b;
                sscanf(hex + 2 * k, "%2x", &b);
                I[offset + k] = (uint8_t)b;
            }
            have = scanf("%15s %lu %lu %8192s", word, &at, &offset, hex) == 4;
        }
        unsigned long long ns = (unsigned long long)tick * common_ticktime__;
        __CURRENT_TIME.tv_sec = (long)(ns / 1000000000ull);
        __CURRENT_TIME.tv_nsec = (long)(ns % 1000000000ull);
        all_in();
        config_run__(tick);
        all_out();
        snprintf(label, sizeof label, "%lu", tick);
        trace(label);
    }
    return 0;
}
