/* iecf.h - the runtime header of iecf's C output (docs/c-output.md).
 *
 * The same for every project; copied next to the generated .c and .h.
 * ISO C99, freestanding: only <stdint.h>, <stddef.h>, <float.h> and
 * <limits.h>; no compiler extensions, no libc. Licensed Apache-2.0 OR
 * MIT, like iecf; the generated code that includes it is the user's.
 */
#ifndef IECF_H
#define IECF_H

#include <float.h>
#include <limits.h>
#include <stddef.h>
#include <stdint.h>

/* ---- what the output needs of the C implementation ---------------------
 * Checked when this header is compiled: a target where one fails cannot
 * run iecf's output as the standard means it. (C99 has no static_assert:
 * an array of negative size is the portable way.) */
#define IECF_REQUIRE(name, cond) typedef char iecf_require_##name[(cond) ? 1 : -1]
IECF_REQUIRE(bytes_are_8_bits, CHAR_BIT == 8);
IECF_REQUIRE(real_is_ieee_binary32, sizeof(float) == 4 && FLT_MANT_DIG == 24 && FLT_MAX_EXP == 128);
IECF_REQUIRE(lreal_is_ieee_binary64, sizeof(double) == 8 && DBL_MANT_DIG == 53 && DBL_MAX_EXP == 1024);

/* ---- the elementary types (docs/dependencies.md) ----------------------- */
typedef uint8_t IEC_BOOL; /* 0 or 1 */
typedef int8_t IEC_SINT;
typedef int16_t IEC_INT;
typedef int32_t IEC_DINT;
typedef int64_t IEC_LINT;
typedef uint8_t IEC_USINT;
typedef uint16_t IEC_UINT;
typedef uint32_t IEC_UDINT;
typedef uint64_t IEC_ULINT;
typedef uint8_t IEC_BYTE;
typedef uint16_t IEC_WORD;
typedef uint32_t IEC_DWORD;
typedef uint64_t IEC_LWORD;
typedef float IEC_REAL;
typedef double IEC_LREAL;
typedef int32_t IEC_TIME;   /* milliseconds, signed */
typedef int64_t IEC_LTIME;  /* nanoseconds, signed */
typedef uint32_t IEC_DATE;  /* seconds since 1970-01-01 */
typedef uint32_t IEC_TOD;   /* milliseconds since midnight */
typedef uint32_t IEC_DT;    /* seconds since 1970-01-01 00:00 */
typedef int64_t IEC_LDATE;  /* nanoseconds since 1970-01-01 */
typedef int64_t IEC_LTOD;   /* nanoseconds since midnight */
typedef int64_t IEC_LDT;    /* nanoseconds since 1970-01-01 00:00 */
typedef uint8_t IEC_CHAR;   /* Latin-1 */
typedef uint16_t IEC_WCHAR; /* UTF-16 unit */
/* STRING(n) is IEC_CHAR[n + 1] and WSTRING(n) IEC_WCHAR[n + 1], each
 * 0-terminated, declared where used. */

/* ---- faults (docs/c-output.md §5) -------------------------------------- */
typedef struct {
    uint16_t code; /* IECF_FAULT_*; IECF_FAULT_NONE while running */
    uint16_t pou;  /* index into the project's POU names */
    uint32_t line; /* source line */
} iecf_fault_t;

#define IECF_FAULT_NONE 0u
#define IECF_FAULT_DIVISION_BY_ZERO 1u
#define IECF_FAULT_OVERFLOW 2u
#define IECF_FAULT_INDEX 3u          /* an array index out of its bounds */
#define IECF_FAULT_CONVERSION 4u     /* a value its target type cannot hold */
#define IECF_FAULT_MUX 5u            /* MUX's K out of 0..n-1 (Ed.3 Table 32 e) */
#define IECF_FAULT_NEGATIVE 6u       /* a negative shift count or string position */
#define IECF_FAULT_STRING_LENGTH 7u  /* a string result longer than its type */
#define IECF_FAULT_IMAGE 8u          /* the process image is smaller than the program addresses */

/* ---- the process image (docs/c-output.md §4) --------------------------- */
typedef struct {
    uint8_t *i, *q, *m;
    uint32_t i_size, q_size, m_size;
} iecf_image_t;

/* Little-endian, byte by byte: the same bytes on any host. */
static inline IEC_BOOL iecf_get_x(const uint8_t *p, uint32_t byte, unsigned bit) {
    return (IEC_BOOL)((p[byte] >> bit) & 1u);
}
static inline void iecf_set_x(uint8_t *p, uint32_t byte, unsigned bit, IEC_BOOL v) {
    p[byte] = (uint8_t)(v ? (p[byte] | (1u << bit)) : (p[byte] & ~(1u << bit)));
}
static inline uint8_t iecf_get_b(const uint8_t *p, uint32_t byte) {
    return p[byte];
}
static inline void iecf_set_b(uint8_t *p, uint32_t byte, uint8_t v) {
    p[byte] = v;
}
static inline uint16_t iecf_get_w(const uint8_t *p, uint32_t byte) {
    return (uint16_t)((uint16_t)p[byte] | (uint16_t)((uint16_t)p[byte + 1] << 8));
}
static inline void iecf_set_w(uint8_t *p, uint32_t byte, uint16_t v) {
    p[byte] = (uint8_t)(v & 0xFFu);
    p[byte + 1] = (uint8_t)(v >> 8);
}
static inline uint32_t iecf_get_d(const uint8_t *p, uint32_t byte) {
    return (uint32_t)iecf_get_w(p, byte) | ((uint32_t)iecf_get_w(p, byte + 2) << 16);
}
static inline void iecf_set_d(uint8_t *p, uint32_t byte, uint32_t v) {
    iecf_set_w(p, byte, (uint16_t)(v & 0xFFFFu));
    iecf_set_w(p, byte + 2, (uint16_t)(v >> 16));
}
static inline uint64_t iecf_get_l(const uint8_t *p, uint32_t byte) {
    return (uint64_t)iecf_get_d(p, byte) | ((uint64_t)iecf_get_d(p, byte + 4) << 32);
}
static inline void iecf_set_l(uint8_t *p, uint32_t byte, uint64_t v) {
    iecf_set_d(p, byte, (uint32_t)(v & 0xFFFFFFFFu));
    iecf_set_d(p, byte + 4, (uint32_t)(v >> 32));
}

/* A REAL or LREAL at an address: its IEEE bits, moved through unsigned
 * char, which C99 allows for any object (6.5 p7); no union punning. */
static inline void iecf_bytes(void *to, const void *from, size_t n) {
    unsigned char *t = (unsigned char *)to;
    const unsigned char *f = (const unsigned char *)from;
    while (n--) *t++ = *f++;
}
static inline IEC_REAL iecf_get_real(const uint8_t *p, uint32_t byte) {
    uint32_t bits = iecf_get_d(p, byte);
    IEC_REAL r;
    iecf_bytes(&r, &bits, sizeof r);
    return r;
}
static inline void iecf_set_real(uint8_t *p, uint32_t byte, IEC_REAL r) {
    uint32_t bits;
    iecf_bytes(&bits, &r, sizeof bits);
    iecf_set_d(p, byte, bits);
}
static inline IEC_LREAL iecf_get_lreal(const uint8_t *p, uint32_t byte) {
    uint64_t bits = iecf_get_l(p, byte);
    IEC_LREAL r;
    iecf_bytes(&r, &bits, sizeof r);
    return r;
}
static inline void iecf_set_lreal(uint8_t *p, uint32_t byte, IEC_LREAL r) {
    uint64_t bits;
    iecf_bytes(&bits, &r, sizeof bits);
    iecf_set_l(p, byte, bits);
}

/* Signed views of the same bytes: two's complement by value, without
 * relying on the implementation-defined unsigned-to-signed conversion. */
static inline int16_t iecf_to_i16(uint16_t u) {
    return u <= INT16_MAX ? (int16_t)u : (int16_t)(-(int16_t)(uint16_t)(~u) - 1);
}
static inline int32_t iecf_to_i32(uint32_t u) {
    return u <= INT32_MAX ? (int32_t)u : (int32_t)(-(int32_t)(uint32_t)(~u) - 1);
}
static inline int64_t iecf_to_i64(uint64_t u) {
    return u <= INT64_MAX ? (int64_t)u : (int64_t)(-(int64_t)(uint64_t)(~u) - 1);
}

#endif /* IECF_H */
