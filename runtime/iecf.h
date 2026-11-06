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
 * relying on the implementation-defined unsigned-to-signed conversion.
 *
 * Throughout: a conditional `c ? x : y` has the type of x and y after
 * the usual arithmetic conversions - int, for anything narrower - so
 * each is cast as a whole, not branch by branch, or -Wconversion may
 * object on the return. */
static inline int8_t iecf_to_i8(uint8_t u) {
    return (int8_t)(u <= INT8_MAX ? (int)u : -(int)(uint8_t)(~u) - 1);
}
static inline int16_t iecf_to_i16(uint16_t u) {
    return (int16_t)(u <= INT16_MAX ? (int32_t)u : -(int32_t)(uint16_t)(~u) - 1);
}
static inline int32_t iecf_to_i32(uint32_t u) {
    return u <= INT32_MAX ? (int32_t)u : (int32_t)(-(int32_t)(uint32_t)(~u) - 1);
}
static inline int64_t iecf_to_i64(uint64_t u) {
    return u <= INT64_MAX ? (int64_t)u : (int64_t)(-(int64_t)(uint64_t)(~u) - 1);
}

/* ---- faults where they happen ----------------------------------------
 * A helper that meets a fault records the first one and returns 0; the
 * generated code checks after each statement (IECF_STOP) and returns. */
static inline void iecf_fail(iecf_fault_t *f, uint16_t code) {
    if (f->code == IECF_FAULT_NONE) f->code = code;
}

/* After a statement: if a fault was recorded, say where, and return. */
#define IECF_STOP(ctx, pou_, line_) \
    do { \
        if ((ctx)->fault.code != IECF_FAULT_NONE) { \
            if ((ctx)->fault.line == 0u) { \
                (ctx)->fault.pou = (pou_); \
                (ctx)->fault.line = (line_); \
            } \
            return; \
        } \
    } while (0)

/* ---- integer arithmetic (Ed.3 Table 29), every overflow a fault --------
 * Division truncates toward zero (Table 29 d). MOD is IN1 - (IN1/IN2)*IN2,
 * and 0 when IN2 is 0 (Table 29 e); x MOD -1 is 0 even where x / -1
 * overflows, as the remainder is exact. */

/* SINT, INT, DINT: computed in 64 bits, where nothing overflows, then fitted. */
#define IECF_SIGNED(T, C, MIN, MAX) \
    static inline C iecf_fit_##T(iecf_fault_t *f, int64_t r) { \
        if (r < (MIN) || r > (MAX)) { \
            iecf_fail(f, IECF_FAULT_OVERFLOW); \
            return 0; \
        } \
        return (C)r; \
    } \
    static inline C iecf_add_##T(iecf_fault_t *f, C a, C b) { return iecf_fit_##T(f, (int64_t)a + (int64_t)b); } \
    static inline C iecf_sub_##T(iecf_fault_t *f, C a, C b) { return iecf_fit_##T(f, (int64_t)a - (int64_t)b); } \
    static inline C iecf_mul_##T(iecf_fault_t *f, C a, C b) { return iecf_fit_##T(f, (int64_t)a * (int64_t)b); } \
    static inline C iecf_div_##T(iecf_fault_t *f, C a, C b) { \
        if (b == 0) { \
            iecf_fail(f, IECF_FAULT_DIVISION_BY_ZERO); \
            return 0; \
        } \
        return iecf_fit_##T(f, (int64_t)a / (int64_t)b); \
    } \
    static inline C iecf_mod_##T(C a, C b) { return (C)(b == 0 ? 0 : (int64_t)a % (int64_t)b); } \
    static inline C iecf_neg_##T(iecf_fault_t *f, C a) { return iecf_fit_##T(f, -(int64_t)a); } \
    static inline C iecf_abs_##T(iecf_fault_t *f, C a) { return iecf_fit_##T(f, a < 0 ? -(int64_t)a : (int64_t)a); }
IECF_SIGNED(SINT, int8_t, INT8_MIN, INT8_MAX)
IECF_SIGNED(INT, int16_t, INT16_MIN, INT16_MAX)
IECF_SIGNED(DINT, int32_t, INT32_MIN, INT32_MAX)

/* LINT: checked before the operation. */
static inline int64_t iecf_add_LINT(iecf_fault_t *f, int64_t a, int64_t b) {
    if ((b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b)) {
        iecf_fail(f, IECF_FAULT_OVERFLOW);
        return 0;
    }
    return a + b;
}
static inline int64_t iecf_sub_LINT(iecf_fault_t *f, int64_t a, int64_t b) {
    if ((b < 0 && a > INT64_MAX + b) || (b > 0 && a < INT64_MIN + b)) {
        iecf_fail(f, IECF_FAULT_OVERFLOW);
        return 0;
    }
    return a - b;
}
static inline int64_t iecf_mul_LINT(iecf_fault_t *f, int64_t a, int64_t b) {
    int over;
    if (a > 0)
        over = b > 0 ? a > INT64_MAX / b : b < INT64_MIN / a;
    else
        over = b > 0 ? a < INT64_MIN / b : (a != 0 && b < INT64_MAX / a);
    if (over) {
        iecf_fail(f, IECF_FAULT_OVERFLOW);
        return 0;
    }
    return a * b;
}
static inline int64_t iecf_div_LINT(iecf_fault_t *f, int64_t a, int64_t b) {
    if (b == 0) {
        iecf_fail(f, IECF_FAULT_DIVISION_BY_ZERO);
        return 0;
    }
    if (a == INT64_MIN && b == -1) {
        iecf_fail(f, IECF_FAULT_OVERFLOW);
        return 0;
    }
    return a / b;
}
static inline int64_t iecf_mod_LINT(int64_t a, int64_t b) {
    return b == 0 || b == -1 ? 0 : a % b;
}
static inline int64_t iecf_neg_LINT(iecf_fault_t *f, int64_t a) {
    if (a == INT64_MIN) {
        iecf_fail(f, IECF_FAULT_OVERFLOW);
        return 0;
    }
    return -a;
}
static inline int64_t iecf_abs_LINT(iecf_fault_t *f, int64_t a) {
    return a < 0 ? iecf_neg_LINT(f, a) : a;
}

/* USINT, UINT, UDINT, ULINT: in 64 unsigned bits, checked first. */
#define IECF_UNSIGNED(T, C, MAX) \
    static inline C iecf_add_##T(iecf_fault_t *f, C a, C b) { \
        uint64_t x = a, y = b; \
        if (x > (uint64_t)(MAX) - y) { \
            iecf_fail(f, IECF_FAULT_OVERFLOW); \
            return 0; \
        } \
        return (C)(x + y); \
    } \
    static inline C iecf_sub_##T(iecf_fault_t *f, C a, C b) { \
        uint64_t x = a, y = b; \
        if (x < y) { \
            iecf_fail(f, IECF_FAULT_OVERFLOW); \
            return 0; \
        } \
        return (C)(x - y); \
    } \
    static inline C iecf_mul_##T(iecf_fault_t *f, C a, C b) { \
        uint64_t x = a, y = b; \
        if (y != 0u && x > (uint64_t)(MAX) / y) { \
            iecf_fail(f, IECF_FAULT_OVERFLOW); \
            return 0; \
        } \
        return (C)(x * y); \
    } \
    static inline C iecf_div_##T(iecf_fault_t *f, C a, C b) { \
        if (b == 0u) { \
            iecf_fail(f, IECF_FAULT_DIVISION_BY_ZERO); \
            return 0; \
        } \
        return (C)((uint64_t)a / (uint64_t)b); \
    } \
    static inline C iecf_mod_##T(C a, C b) { return (C)(b == 0u ? 0u : (uint64_t)a % (uint64_t)b); }
IECF_UNSIGNED(USINT, uint8_t, UINT8_MAX)
IECF_UNSIGNED(UINT, uint16_t, UINT16_MAX)
IECF_UNSIGNED(UDINT, uint32_t, UINT32_MAX)
IECF_UNSIGNED(ULINT, uint64_t, UINT64_MAX)

/* ---- conversions of numbers (Ed.3 6.6.2.5.3) ----------------------------
 * The value is kept, or it is a fault: Ed.3 leaves a value out of the
 * target's range to the implementer (Table 23). Conversions of bit
 * strings are binary transfers (6.6.2.5.4, 6.6.2.5.5): C casts and
 * iecf_to_i*. */
static inline int64_t iecf_s_in(iecf_fault_t *f, int64_t v, int64_t lo, int64_t hi) {
    if (v < lo || v > hi) {
        iecf_fail(f, IECF_FAULT_CONVERSION);
        return 0;
    }
    return v;
}
static inline uint64_t iecf_su_in(iecf_fault_t *f, int64_t v, uint64_t hi) {
    if (v < 0 || (uint64_t)v > hi) {
        iecf_fail(f, IECF_FAULT_CONVERSION);
        return 0;
    }
    return (uint64_t)v;
}
static inline uint64_t iecf_u_in(iecf_fault_t *f, uint64_t v, uint64_t hi) {
    if (v > hi) {
        iecf_fail(f, IECF_FAULT_CONVERSION);
        return 0;
    }
    return v;
}
static inline int64_t iecf_us_in(iecf_fault_t *f, uint64_t v, int64_t hi) {
    if (v > (uint64_t)hi) {
        iecf_fail(f, IECF_FAULT_CONVERSION);
        return 0;
    }
    return (int64_t)v;
}

/* ---- shifts and rotations (Ed.3 Table 30) -------------------------------
 * A negative N is an error (Table 30 a): a fault. SHL and SHR by the
 * width or more give 0; ROL and ROR go round. `_u` takes an unsigned N. */
#define IECF_SHIFTS(T, C, W) \
    static inline C iecf_shl_##T##_u(C x, uint64_t n) { return (C)(n >= (W) ? 0u : (uint64_t)x << n); } \
    static inline C iecf_shr_##T##_u(C x, uint64_t n) { return (C)(n >= (W) ? 0u : (uint64_t)x >> n); } \
    static inline C iecf_rol_##T##_u(C x, uint64_t n) { \
        unsigned k = (unsigned)(n % (W)); \
        return (C)(k == 0u ? (uint64_t)x : ((uint64_t)x << k) | ((uint64_t)x >> ((W) - k))); \
    } \
    static inline C iecf_ror_##T##_u(C x, uint64_t n) { \
        unsigned k = (unsigned)(n % (W)); \
        return (C)(k == 0u ? (uint64_t)x : ((uint64_t)x >> k) | ((uint64_t)x << ((W) - k))); \
    } \
    static inline C iecf_shl_##T(iecf_fault_t *f, C x, int64_t n) { \
        if (n < 0) { \
            iecf_fail(f, IECF_FAULT_NEGATIVE); \
            return 0; \
        } \
        return iecf_shl_##T##_u(x, (uint64_t)n); \
    } \
    static inline C iecf_shr_##T(iecf_fault_t *f, C x, int64_t n) { \
        if (n < 0) { \
            iecf_fail(f, IECF_FAULT_NEGATIVE); \
            return 0; \
        } \
        return iecf_shr_##T##_u(x, (uint64_t)n); \
    } \
    static inline C iecf_rol_##T(iecf_fault_t *f, C x, int64_t n) { \
        if (n < 0) { \
            iecf_fail(f, IECF_FAULT_NEGATIVE); \
            return 0; \
        } \
        return iecf_rol_##T##_u(x, (uint64_t)n); \
    } \
    static inline C iecf_ror_##T(iecf_fault_t *f, C x, int64_t n) { \
        if (n < 0) { \
            iecf_fail(f, IECF_FAULT_NEGATIVE); \
            return 0; \
        } \
        return iecf_ror_##T##_u(x, (uint64_t)n); \
    }
IECF_SHIFTS(BYTE, uint8_t, 8u)
IECF_SHIFTS(WORD, uint16_t, 16u)
IECF_SHIFTS(DWORD, uint32_t, 32u)
IECF_SHIFTS(LWORD, uint64_t, 64u)

/* ---- FOR (Ed.3 7.3.3.4.2) -----------------------------------------------
 * done: whether the loop runs not even once; next: the step, or 1 when the
 * next value would pass the end - checked before adding, so a loop to
 * the type's last value ends instead of overflowing. A step of 0 never
 * ends, as written. The difference end - i is exact in 64 unsigned bits,
 * as i is between start and end. */
#define IECF_FOR_SIGNED(T, C) \
    static inline int iecf_for_done_##T(C i, C end, C step) { return step >= 0 ? i > end : i < end; } \
    static inline int iecf_for_next_##T(C *i, C end, C step) { \
        uint64_t e = (uint64_t)(int64_t)end, v = (uint64_t)(int64_t)*i, s = (uint64_t)(int64_t)step; \
        if (step >= 0 ? e - v < s : v - e < (uint64_t)0 - s) return 1; \
        *i = (C)((int64_t)*i + (int64_t)step); \
        return 0; \
    }
#define IECF_FOR_UNSIGNED(T, C) \
    static inline int iecf_for_done_##T(C i, C end, C step) { \
        (void)step; \
        return i > end; \
    } \
    static inline int iecf_for_next_##T(C *i, C end, C step) { \
        if ((uint64_t)end - (uint64_t)*i < (uint64_t)step) return 1; \
        *i = (C)((uint64_t)*i + (uint64_t)step); \
        return 0; \
    }
IECF_FOR_SIGNED(SINT, int8_t)
IECF_FOR_SIGNED(INT, int16_t)
IECF_FOR_SIGNED(DINT, int32_t)
IECF_FOR_SIGNED(LINT, int64_t)
IECF_FOR_UNSIGNED(USINT, uint8_t)
IECF_FOR_UNSIGNED(UINT, uint16_t)
IECF_FOR_UNSIGNED(UDINT, uint32_t)
IECF_FOR_UNSIGNED(ULINT, uint64_t)

/* ---- selection (Ed.3 Table 32): MIN, MAX, LIMIT ------------------------- */
#define IECF_SELECT(T, C) \
    static inline C iecf_min_##T(C a, C b) { return (C)(b < a ? b : a); } \
    static inline C iecf_max_##T(C a, C b) { return (C)(b > a ? b : a); } \
    static inline C iecf_limit_##T(C mn, C in, C mx) { return iecf_min_##T(iecf_max_##T(in, mn), mx); }
IECF_SELECT(SINT, int8_t)
IECF_SELECT(INT, int16_t)
IECF_SELECT(DINT, int32_t)
IECF_SELECT(LINT, int64_t)
IECF_SELECT(USINT, uint8_t)
IECF_SELECT(UINT, uint16_t)
IECF_SELECT(UDINT, uint32_t)
IECF_SELECT(ULINT, uint64_t)
IECF_SELECT(BYTE, uint8_t)
IECF_SELECT(WORD, uint16_t)
IECF_SELECT(DWORD, uint32_t)
IECF_SELECT(LWORD, uint64_t)
IECF_SELECT(BOOL, uint8_t)
IECF_SELECT(REAL, float)
IECF_SELECT(LREAL, double)

#endif /* IECF_H */
