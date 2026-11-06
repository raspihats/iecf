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
#define IECF_FAULT_OVERFLOW 2u        /* a result beyond its type: a flag, not a fault */
#define IECF_FAULT_INDEX 3u          /* an array index out of its bounds */
#define IECF_FAULT_CONVERSION 4u     /* a value its target type cannot hold: a flag, not a fault */
#define IECF_FAULT_MUX 5u            /* MUX's K out of 0..n-1 (Ed.3 Table 32 e) */
#define IECF_FAULT_NEGATIVE 6u       /* a negative shift count */
#define IECF_FAULT_STRING_LENGTH 7u  /* a string longer than its type, cut to fit: a flag, not a fault */
#define IECF_FAULT_IMAGE 8u          /* the process image is smaller than the program addresses */
#define IECF_FAULT_NAN 9u            /* a REAL or LREAL result that is not a number: a flag, not a fault */
#define IECF_FAULT_STRING_POSITION 10u /* a string length or position out of its string: a flag, not a fault */

/* ---- the symbol table (docs/c-output.md §6) ------------------------------
 * <p>_symbols lists each top-level variable - a global, a program
 * instance's variable - by its path, upper-cased: `G`, `R.G` for a
 * resource's global, `R.P.X` for variable X of program instance P on
 * resource R. <p>_types describes each type they use once: an array's
 * element type and bounds, a structure's or function block's members
 * with their offsets. A platform finds `R.P.PTS[2].X` by walking them. */
#define IECF_KIND_ELEM 0u   /* elementary: `elem` is its IECF_T_* */
#define IECF_KIND_ENUM 1u   /* held as `elem`; `members` are its values */
#define IECF_KIND_STRUCT 2u
#define IECF_KIND_ARRAY 3u  /* `of`, `dims`, `bounds` (lo, hi each) */
#define IECF_KIND_BLOCK 4u  /* a function block: inputs, outputs, variables */

#define IECF_T_BOOL 1u
#define IECF_T_SINT 2u
#define IECF_T_INT 3u
#define IECF_T_DINT 4u
#define IECF_T_LINT 5u
#define IECF_T_USINT 6u
#define IECF_T_UINT 7u
#define IECF_T_UDINT 8u
#define IECF_T_ULINT 9u
#define IECF_T_BYTE 10u
#define IECF_T_WORD 11u
#define IECF_T_DWORD 12u
#define IECF_T_LWORD 13u
#define IECF_T_REAL 14u
#define IECF_T_LREAL 15u
#define IECF_T_TIME 16u
#define IECF_T_LTIME 17u
#define IECF_T_STRING 18u  /* `size` is n + 1: up to n characters, then a 0 */

#define IECF_SYM_INPUT 1u    /* a VAR_INPUT */
#define IECF_SYM_OUTPUT 2u   /* a VAR_OUTPUT */
#define IECF_SYM_CONSTANT 4u /* declared CONSTANT: not to be written */
#define IECF_SYM_RETAIN 8u   /* in ctx->retain: kept across a restart */

typedef struct {
    const char *name;   /* upper-cased */
    uint32_t type;      /* index into <p>_types */
    uint32_t offset;    /* in its structure; 0 for an enumerated value */
    uint32_t flags;     /* IECF_SYM_* */
    int64_t value;      /* an enumerated value's number */
} iecf_member_t;

typedef struct {
    const char *name;   /* `INT`, `POINT`, `TON`, `ARRAY [1..4] OF INT` */
    uint32_t kind;      /* IECF_KIND_* */
    uint32_t elem;      /* IECF_T_*, for an elementary or enumerated type */
    uint32_t size;      /* sizeof */
    uint32_t of;        /* an array's element type */
    uint32_t dims;
    const int64_t *bounds;
    uint32_t member_count;
    const iecf_member_t *members;
} iecf_type_t;

typedef struct {
    const char *path;   /* upper-cased */
    uint32_t type;      /* index into <p>_types */
    uint32_t offset;    /* in the context; in the process image, the byte */
    char area;          /* 0 for the context; 'I', 'Q' or 'M' */
    uint8_t bit;        /* a located BOOL's bit */
    uint16_t flags;     /* IECF_SYM_* */
} iecf_symbol_t;

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
 * generated code checks after each statement (iecf_stopped) and returns. */
static inline void iecf_fail(iecf_fault_t *f, uint16_t code) {
    if (f->code == IECF_FAULT_NONE) f->code = code;
}

/* After a statement: whether a fault was recorded - the generated code
 * returns then - and the first statement to see it records where. */
static inline int iecf_stopped(iecf_fault_t *f, uint16_t pou, uint32_t line) {
    if (f->code == IECF_FAULT_NONE) return 0;
    if (f->line == 0u) {
        f->pou = pou;
        f->line = line;
    }
    return 1;
}

/* ---- flags: carried on, recorded (docs/c-output.md §5) -------------------
 * An integer overflow, a conversion out of range and an infinite or NaN
 * real do not stop the task: the value carries on - wrapped, for an
 * integer - and the first is recorded in ctx->flag, its code and, from
 * iecf_noted after the statement, its POU and line, until the platform
 * clears it (PROPOSAL.md §6.3). */
static inline void iecf_flag(iecf_fault_t *g, uint16_t code) {
    if (g->code == IECF_FAULT_NONE) g->code = code;
}
/* After a statement that may have flagged: where, the first time. */
static inline void iecf_noted(iecf_fault_t *g, uint16_t pou, uint32_t line) {
    if (g->code != IECF_FAULT_NONE && g->line == 0u) {
        g->pou = pou;
        g->line = line;
    }
}

/* ---- integer arithmetic (Ed.3 Table 29) ----------------------------------
 * A result out of the type's range wraps, two's complement, as CODESYS,
 * TwinCAT, Siemens and Rockwell have it, and is flagged
 * IECF_FAULT_OVERFLOW (g: &ctx->flag). Division by zero is a fault
 * (f: &ctx->fault) and gives 0. Division truncates toward zero (Table
 * 29 d). MOD is IN1 - (IN1/IN2)*IN2, and 0 when IN2 is 0 (Table 29 e);
 * x MOD -1 is 0 even where x / -1 overflows, as the remainder is exact.
 * Wrapping goes through unsigned types, where C defines it (C99 6.2.5),
 * and iecf_to_i*. */

/* SINT, INT, DINT: computed in 64 bits, where nothing overflows, then fitted. */
#define IECF_SIGNED(T, C, U, TO, MIN, MAX) \
    static inline C iecf_fit_##T(iecf_fault_t *g, int64_t r) { \
        if (r < (MIN) || r > (MAX)) iecf_flag(g, IECF_FAULT_OVERFLOW); \
        return TO((U)(uint64_t)r); \
    } \
    static inline C iecf_add_##T(iecf_fault_t *g, C a, C b) { return iecf_fit_##T(g, (int64_t)a + (int64_t)b); } \
    static inline C iecf_sub_##T(iecf_fault_t *g, C a, C b) { return iecf_fit_##T(g, (int64_t)a - (int64_t)b); } \
    static inline C iecf_mul_##T(iecf_fault_t *g, C a, C b) { return iecf_fit_##T(g, (int64_t)a * (int64_t)b); } \
    static inline C iecf_div_##T(iecf_fault_t *f, iecf_fault_t *g, C a, C b) { \
        if (b == 0) { \
            iecf_fail(f, IECF_FAULT_DIVISION_BY_ZERO); \
            return 0; \
        } \
        return iecf_fit_##T(g, (int64_t)a / (int64_t)b); \
    } \
    static inline C iecf_mod_##T(C a, C b) { return (C)(b == 0 ? 0 : (int64_t)a % (int64_t)b); } \
    static inline C iecf_neg_##T(iecf_fault_t *g, C a) { return iecf_fit_##T(g, -(int64_t)a); } \
    static inline C iecf_abs_##T(iecf_fault_t *g, C a) { return iecf_fit_##T(g, a < 0 ? -(int64_t)a : (int64_t)a); }
IECF_SIGNED(SINT, int8_t, uint8_t, iecf_to_i8, INT8_MIN, INT8_MAX)
IECF_SIGNED(INT, int16_t, uint16_t, iecf_to_i16, INT16_MIN, INT16_MAX)
IECF_SIGNED(DINT, int32_t, uint32_t, iecf_to_i32, INT32_MIN, INT32_MAX)

/* LINT: computed in 64 unsigned bits, which wrap; the overflow tested
 * before. */
static inline int64_t iecf_add_LINT(iecf_fault_t *g, int64_t a, int64_t b) {
    if ((b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b)) iecf_flag(g, IECF_FAULT_OVERFLOW);
    return iecf_to_i64((uint64_t)a + (uint64_t)b);
}
static inline int64_t iecf_sub_LINT(iecf_fault_t *g, int64_t a, int64_t b) {
    if ((b < 0 && a > INT64_MAX + b) || (b > 0 && a < INT64_MIN + b)) iecf_flag(g, IECF_FAULT_OVERFLOW);
    return iecf_to_i64((uint64_t)a - (uint64_t)b);
}
static inline int64_t iecf_mul_LINT(iecf_fault_t *g, int64_t a, int64_t b) {
    int over;
    if (a > 0)
        over = b > 0 ? a > INT64_MAX / b : b < INT64_MIN / a;
    else
        over = b > 0 ? a < INT64_MIN / b : (a != 0 && b < INT64_MAX / a);
    if (over) iecf_flag(g, IECF_FAULT_OVERFLOW);
    return iecf_to_i64((uint64_t)a * (uint64_t)b);
}
static inline int64_t iecf_neg_LINT(iecf_fault_t *g, int64_t a) {
    if (a == INT64_MIN) iecf_flag(g, IECF_FAULT_OVERFLOW);
    return iecf_to_i64((uint64_t)0 - (uint64_t)a);
}
static inline int64_t iecf_div_LINT(iecf_fault_t *f, iecf_fault_t *g, int64_t a, int64_t b) {
    if (b == 0) {
        iecf_fail(f, IECF_FAULT_DIVISION_BY_ZERO);
        return 0;
    }
    if (b == -1) return iecf_neg_LINT(g, a); /* INT64_MIN / -1 wraps to itself */
    return a / b;
}
static inline int64_t iecf_mod_LINT(int64_t a, int64_t b) {
    return b == 0 || b == -1 ? 0 : a % b;
}
static inline int64_t iecf_abs_LINT(iecf_fault_t *g, int64_t a) {
    return a < 0 ? iecf_neg_LINT(g, a) : a;
}

/* USINT, UINT, UDINT, ULINT: in 64 unsigned bits, tested first. */
#define IECF_UNSIGNED(T, C, MAX) \
    static inline C iecf_add_##T(iecf_fault_t *g, C a, C b) { \
        uint64_t x = a, y = b; \
        if (x > (uint64_t)(MAX) - y) iecf_flag(g, IECF_FAULT_OVERFLOW); \
        return (C)(x + y); \
    } \
    static inline C iecf_sub_##T(iecf_fault_t *g, C a, C b) { \
        uint64_t x = a, y = b; \
        if (x < y) iecf_flag(g, IECF_FAULT_OVERFLOW); \
        return (C)(x - y); \
    } \
    static inline C iecf_mul_##T(iecf_fault_t *g, C a, C b) { \
        uint64_t x = a, y = b; \
        if (y != 0u && x > (uint64_t)(MAX) / y) iecf_flag(g, IECF_FAULT_OVERFLOW); \
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
 * The value is kept where the target holds it. Where it does not - Ed.3
 * leaves that to the implementer (Table 23) - its low bits are kept, as
 * CODESYS and TwinCAT have it (INT_TO_SINT(4223) = 127), and it is
 * flagged IECF_FAULT_CONVERSION. These test the range and return the
 * value's two's complement bits; the generated code casts them to an
 * unsigned target, or through iecf_to_i* to a signed one. Conversions
 * of bit strings are binary transfers (6.6.2.5.4, 6.6.2.5.5): C casts
 * and iecf_to_i*. */
static inline uint64_t iecf_s_in(iecf_fault_t *g, int64_t v, int64_t lo, int64_t hi) {
    if (v < lo || v > hi) iecf_flag(g, IECF_FAULT_CONVERSION);
    return (uint64_t)v;
}
static inline uint64_t iecf_su_in(iecf_fault_t *g, int64_t v, uint64_t hi) {
    if (v < 0 || (uint64_t)v > hi) iecf_flag(g, IECF_FAULT_CONVERSION);
    return (uint64_t)v;
}
static inline uint64_t iecf_u_in(iecf_fault_t *g, uint64_t v, uint64_t hi) {
    if (v > hi) iecf_flag(g, IECF_FAULT_CONVERSION);
    return v;
}
static inline uint64_t iecf_us_in(iecf_fault_t *g, uint64_t v, int64_t hi) {
    if (v > (uint64_t)hi) iecf_flag(g, IECF_FAULT_CONVERSION);
    return v;
}

/* ---- subranges (Ed.3 6.4.4.4) -------------------------------------------
 * A value stored into a subrange outside its bounds is clamped to the
 * bound it passed and flagged IECF_FAULT_CONVERSION (g: &ctx->flag), as
 * CODESYS's CheckRange functions clamp (docs/dependencies.md). */
static inline int64_t iecf_clamp_s(iecf_fault_t *g, int64_t v, int64_t lo, int64_t hi) {
    if (v < lo) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return lo;
    }
    if (v > hi) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return hi;
    }
    return v;
}
static inline uint64_t iecf_clamp_u(iecf_fault_t *g, uint64_t v, uint64_t lo, uint64_t hi) {
    if (v < lo) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return lo;
    }
    if (v > hi) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return hi;
    }
    return v;
}

/* A FOR over a subrange stops at the bound it would pass: its end,
 * beyond the bound in the step's direction, is the bound, flagged. An end
 * on the other side is left: the loop then does not run, as written. */
static inline int64_t iecf_for_end_s(iecf_fault_t *g, int64_t end, int64_t step, int64_t lo, int64_t hi) {
    if (step >= 0 && end > hi) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return hi;
    }
    if (step < 0 && end < lo) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return lo;
    }
    return end;
}
static inline uint64_t iecf_for_end_u(iecf_fault_t *g, uint64_t end, uint64_t hi) {
    if (end > hi) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return hi;
    }
    return end;
}

/* ---- array indices (Ed.3 6.4.4.5) ---------------------------------------
 * An index's offset from its dimension's lower bound. Out of the bounds
 * it is a fault (PROPOSAL.md §6.3) and gives 0, an element that exists:
 * the statement is stopped before it stores anything. */
static inline uint32_t iecf_index(iecf_fault_t *f, int64_t i, int64_t lo, int64_t hi) {
    if (i < lo || i > hi) {
        iecf_fail(f, IECF_FAULT_INDEX);
        return 0;
    }
    return (uint32_t)((uint64_t)i - (uint64_t)lo);
}
static inline uint32_t iecf_index_u(iecf_fault_t *f, uint64_t i, int64_t lo, int64_t hi) {
    if (hi < 0 || i > (uint64_t)hi) {
        iecf_fail(f, IECF_FAULT_INDEX);
        return 0;
    }
    return iecf_index(f, (int64_t)i, lo, hi);
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

/* ---- selection (Ed.3 Table 32): SEL, MIN, MAX, LIMIT --------------------
 * A function, so both of SEL's inputs are evaluated, as any function's. */
#define IECF_SELECT(T, C) \
    static inline C iecf_sel_##T(uint8_t g, C in0, C in1) { return g ? in1 : in0; } \
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

/* ---- STRING (docs/dependencies.md) ---------------------------------------
 * A STRING(n) is char[n + 1]: the characters, one byte each (Latin-1),
 * then a 0. A function's result goes into a buffer `d` the generated code
 * sizes for the most its inputs can make (n + 1 bytes), so only a store
 * into a shorter variable cuts a string short. Comparison is by the
 * characters' codes, left to right, a shorter string extended with zeros
 * (Ed.3 6.6.2.5.11). */
static inline uint32_t iecf_slen(const char *s) {
    uint32_t n = 0u;
    while (s[n] != 0) n++;
    return n;
}
static inline int iecf_scmp(const char *a, const char *b) {
    const unsigned char *x = (const unsigned char *)a, *y = (const unsigned char *)b;
    while (*x != 0u && *x == *y) {
        x++;
        y++;
    }
    return (*x > *y) - (*x < *y);
}
/* A STRING(n)'s initial value, which fits: the rest of it zeros. */
static inline void iecf_sinit(char *d, uint32_t n, const char *s) {
    uint32_t i = 0u;
    for (; s[i] != 0 && i < n; i++) d[i] = s[i];
    for (; i <= n; i++) d[i] = 0;
}
/* d := s for d a STRING(n): longer than n, cut to n and flagged
 * IECF_FAULT_STRING_LENGTH (g: &ctx->flag). s may be d. */
static inline char *iecf_scopy(iecf_fault_t *g, char *d, uint32_t n, const char *s) {
    uint32_t i = 0u;
    for (; s[i] != 0 && i < n; i++) d[i] = s[i];
    if (s[i] != 0) iecf_flag(g, IECF_FAULT_STRING_LENGTH);
    d[i] = 0;
    return d;
}
/* a then b into d, n characters at most. */
static inline const char *iecf_concat(iecf_fault_t *g, char *d, uint32_t n, const char *a, const char *b) {
    uint32_t i = 0u;
    for (; *a != 0 && i < n; a++) d[i++] = *a;
    for (; *b != 0 && i < n; b++) d[i++] = *b;
    if (*a != 0 || *b != 0) iecf_flag(g, IECF_FAULT_STRING_LENGTH);
    d[i] = 0;
    return d;
}
/* LEFT, RIGHT, MID, DELETE, INSERT, REPLACE (Ed.3 Table 34), positions
 * from 1. A length or position the standard calls an error - negative, a
 * position not in the string, a length past its end - gives what Siemens
 * gives (S7-1200 System Manual 8.2.4, its ENO tables) and is flagged
 * IECF_FAULT_STRING_POSITION (g: &ctx->flag); the task goes on (decided
 * 2026-11-06, docs/dependencies.md):
 *   LEFT, RIGHT: L < 0 empty; L past the end all of IN.
 *   MID: L < 0 or P < 1 empty; from P to the end if L runs past it.
 *   DELETE: L < 0 or P < 1 empty; P past the end IN unchanged; L past
 *     the end deletes to the end.
 *   INSERT: P < 0 empty; P past the end appends IN2.
 *   REPLACE: L < 0 or P < 1 empty; P past the end appends IN2; L past the
 *     end replaces to the end. */
static inline uint32_t iecf_put(char *d, uint32_t i, uint32_t n, const char *s, uint32_t k) {
    uint32_t j;
    for (j = 0u; j < k && s[j] != 0 && i < n; j++) d[i++] = s[j];
    return i;
}
static inline const char *iecf_bad(iecf_fault_t *g, char *d) {
    iecf_flag(g, IECF_FAULT_STRING_POSITION);
    d[0] = 0;
    return d;
}
/* l clamped to what is left from the 0-based p of a string of length m,
 * flagged if it ran past. */
static inline uint32_t iecf_upto(iecf_fault_t *g, int64_t l, uint32_t p, uint32_t m) {
    uint32_t left = m - p;
    if ((uint64_t)l > (uint64_t)left) {
        iecf_flag(g, IECF_FAULT_STRING_POSITION);
        return left;
    }
    return (uint32_t)l;
}
static inline const char *iecf_left(iecf_fault_t *g, char *d, uint32_t n, const char *in, int64_t l) {
    uint32_t m = iecf_slen(in), k;
    if (l < 0) return iecf_bad(g, d);
    k = iecf_upto(g, l, 0u, m);
    d[iecf_put(d, 0u, n, in, k)] = 0;
    return d;
}
static inline const char *iecf_right(iecf_fault_t *g, char *d, uint32_t n, const char *in, int64_t l) {
    uint32_t m = iecf_slen(in), k;
    if (l < 0) return iecf_bad(g, d);
    k = iecf_upto(g, l, 0u, m);
    d[iecf_put(d, 0u, n, in + (m - k), k)] = 0;
    return d;
}
static inline const char *iecf_mid(iecf_fault_t *g, char *d, uint32_t n, const char *in, int64_t l, int64_t p) {
    uint32_t m = iecf_slen(in), k;
    if (l < 0 || p < 1) return iecf_bad(g, d);
    if (l == 0) { /* no character taken */
        d[0] = 0;
        return d;
    }
    if ((uint64_t)p > (uint64_t)m) return iecf_bad(g, d);
    k = iecf_upto(g, l, (uint32_t)(p - 1), m);
    d[iecf_put(d, 0u, n, in + (p - 1), k)] = 0;
    return d;
}
static inline const char *iecf_delete(iecf_fault_t *g, char *d, uint32_t n, const char *in, int64_t l, int64_t p) {
    uint32_t m = iecf_slen(in), k, i;
    if (l < 0 || p < 1) return iecf_bad(g, d);
    if ((uint64_t)p > (uint64_t)m) {
        if (l != 0) iecf_flag(g, IECF_FAULT_STRING_POSITION);
        d[iecf_put(d, 0u, n, in, m)] = 0;
        return d;
    }
    k = iecf_upto(g, l, (uint32_t)(p - 1), m);
    i = iecf_put(d, 0u, n, in, (uint32_t)(p - 1));
    d[iecf_put(d, i, n, in + (p - 1) + k, m)] = 0;
    return d;
}
static inline const char *iecf_insert(iecf_fault_t *g, char *d, uint32_t n, const char *a, const char *b, int64_t p) {
    uint32_t m = iecf_slen(a), at, i;
    if (p < 0) return iecf_bad(g, d);
    at = (uint32_t)p;
    if ((uint64_t)p > (uint64_t)m) {
        iecf_flag(g, IECF_FAULT_STRING_POSITION);
        at = m;
    }
    i = iecf_put(d, 0u, n, a, at);
    i = iecf_put(d, i, n, b, UINT32_MAX);
    i = iecf_put(d, i, n, a + at, m);
    if ((uint64_t)m + iecf_slen(b) > n) iecf_flag(g, IECF_FAULT_STRING_LENGTH);
    d[i] = 0;
    return d;
}
static inline const char *iecf_replace(iecf_fault_t *g, char *d, uint32_t n, const char *a, const char *b, int64_t l, int64_t p) {
    uint32_t m = iecf_slen(a), at, k, i;
    if (l < 0 || p < 1) return iecf_bad(g, d);
    if ((uint64_t)p > (uint64_t)m) {
        iecf_flag(g, IECF_FAULT_STRING_POSITION);
        at = m;
        k = 0u;
    } else {
        at = (uint32_t)(p - 1);
        k = iecf_upto(g, l, at, m);
    }
    i = iecf_put(d, 0u, n, a, at);
    i = iecf_put(d, i, n, b, UINT32_MAX);
    i = iecf_put(d, i, n, a + at + k, m);
    if ((uint64_t)m - k + iecf_slen(b) > n) iecf_flag(g, IECF_FAULT_STRING_LENGTH);
    d[i] = 0;
    return d;
}
/* FIND: where b first starts in a, from 1; 0 if nowhere, or b is empty. */
static inline uint32_t iecf_find(const char *a, const char *b) {
    uint32_t i, j;
    if (*b == 0) return 0u;
    for (i = 0u; a[i] != 0; i++) {
        for (j = 0u; b[j] != 0 && a[i + j] == b[j]; j++) {
        }
        if (b[j] == 0) return i + 1u;
    }
    return 0u;
}

/* ---- REAL and LREAL (IEEE 754 binary32, binary64) ------------------------
 * Division by zero is a fault, as for integers (PROPOSAL.md §6.3). An
 * infinite or NaN result is not: it carries on, as IEEE 754 has it, and
 * is flagged in ctx->flag (docs/c-output.md §5) - code
 * IECF_FAULT_OVERFLOW for an infinity, IECF_FAULT_NAN for a NaN, with
 * the POU and line of the first, until the platform clears it. Each
 * result is cast to its type, so no wider precision survives
 * (C99 6.3.1.8). */
static inline double iecf_flag_real(iecf_fault_t *g, double x) {
    if (x != x)
        iecf_flag(g, IECF_FAULT_NAN);
    else if (x - x != 0.0)
        iecf_flag(g, IECF_FAULT_OVERFLOW);
    return x;
}
static inline IEC_REAL iecf_bits_real(uint32_t bits);
/* A double as a REAL: rounded; beyond REAL's range, the infinity of its
 * sign (C99 6.3.1.5 leaves converting it undefined); flagged if not finite. */
static inline IEC_REAL iecf_real_of(iecf_fault_t *g, double x) {
    IEC_REAL r;
    if (x > FLT_MAX && x - x == 0.0)
        r = iecf_bits_real(0x7F800000u);
    else if (x < -FLT_MAX && x - x == 0.0)
        r = iecf_bits_real(0xFF800000u);
    else
        r = (IEC_REAL)x;
    iecf_flag_real(g, (double)r);
    return r;
}
static inline IEC_LREAL iecf_lreal_of(iecf_fault_t *g, double x) {
    return iecf_flag_real(g, x);
}
#define IECF_REAL_OPS(T, C) \
    static inline C iecf_add_##T(iecf_fault_t *g, C a, C b) { return (C)iecf_flag_real(g, (double)(C)(a + b)); } \
    static inline C iecf_sub_##T(iecf_fault_t *g, C a, C b) { return (C)iecf_flag_real(g, (double)(C)(a - b)); } \
    static inline C iecf_mul_##T(iecf_fault_t *g, C a, C b) { return (C)iecf_flag_real(g, (double)(C)(a * b)); } \
    static inline C iecf_div_##T(iecf_fault_t *f, iecf_fault_t *g, C a, C b) { \
        if (b == (C)0) { \
            iecf_fail(f, IECF_FAULT_DIVISION_BY_ZERO); \
            return (C)0; \
        } \
        return (C)iecf_flag_real(g, (double)(C)(a / b)); \
    }
IECF_REAL_OPS(REAL, IEC_REAL)
IECF_REAL_OPS(LREAL, IEC_LREAL)
/* ABS: -0.0 gives +0.0, as fabs does. */
static inline IEC_REAL iecf_abs_REAL(IEC_REAL x) {
    return x < 0.0f ? -x : (IEC_REAL)(x + 0.0f);
}
static inline IEC_LREAL iecf_abs_LREAL(IEC_LREAL x) {
    return x < 0.0 ? -x : x + 0.0;
}

/* A real to an integer (Ed.3 6.6.2.5.3): rounded to the nearest, a half
 * to the even one; TRUNC toward zero. A value the target cannot hold, an
 * infinity and a NaN have no integer to carry on with: the helper flags
 * IECF_FAULT_CONVERSION (g: &ctx->flag), sets *keep and gives 0, and the
 * generated code leaves the statement undone, its target as it was
 * (docs/c-output.md §5). Every real
 * arrives as a double, which holds a REAL exactly; the range is tested
 * before C converts, where out of range is undefined (C99 6.3.1.4). */
static inline int64_t iecf_unconverted(iecf_fault_t *g, int *keep) {
    iecf_flag(g, IECF_FAULT_CONVERSION);
    *keep = 1;
    return 0;
}
static inline int64_t iecf_round_s(iecf_fault_t *g, int *keep, double x, int64_t lo, int64_t hi) {
    int64_t t;
    double r;
    if (!(x >= -9223372036854775808.0 && x < 9223372036854775808.0)) return iecf_unconverted(g, keep);
    t = (int64_t)x;
    r = x - (double)t; /* exact: |x| < 2^63 and t its integral part */
    if (r > 0.5 || (r == 0.5 && t % 2 != 0))
        t += 1; /* from 2^52 up every double is an integer: r is 0 there */
    else if (r < -0.5 || (r == -0.5 && t % 2 != 0))
        t -= 1;
    if (t < lo || t > hi) return iecf_unconverted(g, keep);
    return t;
}
static inline uint64_t iecf_round_u(iecf_fault_t *g, int *keep, double x, uint64_t hi) {
    uint64_t t;
    double r;
    if (!(x >= -0.5 && x < 18446744073709551616.0)) return (uint64_t)iecf_unconverted(g, keep);
    if (x <= 0.0) return 0; /* -0.5 rounds to the even 0 */
    t = (uint64_t)x;
    r = x - (double)t;
    if (r > 0.5 || (r == 0.5 && (t & 1u) != 0u)) t += 1u;
    if (t > hi) return (uint64_t)iecf_unconverted(g, keep);
    return t;
}
static inline int64_t iecf_trunc_s(iecf_fault_t *g, int *keep, double x, int64_t lo, int64_t hi) {
    int64_t t;
    if (!(x >= -9223372036854775808.0 && x < 9223372036854775808.0)) return iecf_unconverted(g, keep);
    t = (int64_t)x;
    if (t < lo || t > hi) return iecf_unconverted(g, keep);
    return t;
}
static inline uint64_t iecf_trunc_u(iecf_fault_t *g, int *keep, double x, uint64_t hi) {
    uint64_t t;
    if (!(x > -1.0 && x < 18446744073709551616.0)) return (uint64_t)iecf_unconverted(g, keep);
    t = x <= 0.0 ? 0u : (uint64_t)x;
    if (t > hi) return (uint64_t)iecf_unconverted(g, keep);
    return t;
}
/* DWORD_TO_REAL and the others of Table 25: the bits, moved. */
static inline IEC_REAL iecf_bits_real(uint32_t bits) {
    IEC_REAL r;
    iecf_bytes(&r, &bits, sizeof r);
    return r;
}
static inline uint32_t iecf_real_bits(IEC_REAL r) {
    uint32_t bits;
    iecf_bytes(&bits, &r, sizeof bits);
    return bits;
}
static inline IEC_LREAL iecf_bits_lreal(uint64_t bits) {
    IEC_LREAL r;
    iecf_bytes(&r, &bits, sizeof r);
    return r;
}
static inline uint64_t iecf_lreal_bits(IEC_LREAL r) {
    uint64_t bits;
    iecf_bytes(&bits, &r, sizeof bits);
    return bits;
}

/* ---- the standard function blocks (Ed.3 6.6.3.5, Tables 43-46) ---------
 * Each is a structure - its inputs and outputs by their standard names,
 * then what it keeps for itself, ending in `_` - and a function the
 * generated code calls after setting the inputs. iecf_<FB>_init gives
 * every field its initial value. */

/* Bistables (Table 43): SR is set-dominant, RS reset-dominant. */
typedef struct {
    IEC_BOOL S1, R, Q1;
} iecf_SR_t;
static inline void iecf_SR_init(iecf_SR_t *fb) {
    fb->S1 = fb->R = fb->Q1 = 0;
}
static inline void iecf_SR(iecf_SR_t *fb) {
    fb->Q1 = (IEC_BOOL)(fb->S1 | ((!fb->R) & fb->Q1));
}
typedef struct {
    IEC_BOOL S, R1, Q1;
} iecf_RS_t;
static inline void iecf_RS_init(iecf_RS_t *fb) {
    fb->S = fb->R1 = fb->Q1 = 0;
}
static inline void iecf_RS(iecf_RS_t *fb) {
    fb->Q1 = (IEC_BOOL)((!fb->R1) & (fb->S | fb->Q1));
}

/* Edge detection (Table 44), as the standard writes their bodies: an
 * F_TRIG called first with CLK FALSE gives Q TRUE once (M starts FALSE). */
typedef struct {
    IEC_BOOL CLK, Q, M_;
} iecf_R_TRIG_t;
static inline void iecf_R_TRIG_init(iecf_R_TRIG_t *fb) {
    fb->CLK = fb->Q = fb->M_ = 0;
}
static inline void iecf_R_TRIG(iecf_R_TRIG_t *fb) {
    fb->Q = (IEC_BOOL)(fb->CLK & (!fb->M_));
    fb->M_ = fb->CLK;
}
typedef struct {
    IEC_BOOL CLK, Q, M_;
} iecf_F_TRIG_t;
static inline void iecf_F_TRIG_init(iecf_F_TRIG_t *fb) {
    fb->CLK = fb->Q = fb->M_ = 0;
}
static inline void iecf_F_TRIG(iecf_F_TRIG_t *fb) {
    fb->Q = (IEC_BOOL)((!fb->CLK) & (!fb->M_));
    fb->M_ = (IEC_BOOL)!fb->CLK;
}

/* Counters (Table 45), one set per type of PV and CV. CU and CD are
 * R_EDGE inputs: the block keeps their last value. CV counts up only
 * below HI and down only above LO, Table 45's PVmax and PVmin
 * (docs/dependencies.md). */
#define IECF_COUNTERS(S, C, LO, HI) \
    typedef struct { \
        IEC_BOOL CU, R; \
        C PV; \
        IEC_BOOL Q; \
        C CV; \
        IEC_BOOL CU_; \
    } iecf_CTU##S##_t; \
    static inline void iecf_CTU##S##_init(iecf_CTU##S##_t *fb) { \
        fb->CU = fb->R = fb->Q = fb->CU_ = 0; \
        fb->PV = fb->CV = 0; \
    } \
    static inline void iecf_CTU##S(iecf_CTU##S##_t *fb) { \
        IEC_BOOL up = (IEC_BOOL)(fb->CU & (!fb->CU_)); \
        fb->CU_ = fb->CU; \
        if (fb->R) \
            fb->CV = 0; \
        else if (up && fb->CV < (HI)) \
            fb->CV = (C)(fb->CV + 1); \
        fb->Q = (IEC_BOOL)(fb->CV >= fb->PV); \
    } \
    typedef struct { \
        IEC_BOOL CD, LD; \
        C PV; \
        IEC_BOOL Q; \
        C CV; \
        IEC_BOOL CD_; \
    } iecf_CTD##S##_t; \
    static inline void iecf_CTD##S##_init(iecf_CTD##S##_t *fb) { \
        fb->CD = fb->LD = fb->Q = fb->CD_ = 0; \
        fb->PV = fb->CV = 0; \
    } \
    static inline void iecf_CTD##S(iecf_CTD##S##_t *fb) { \
        IEC_BOOL down = (IEC_BOOL)(fb->CD & (!fb->CD_)); \
        fb->CD_ = fb->CD; \
        if (fb->LD) \
            fb->CV = fb->PV; \
        else if (down && fb->CV > (LO)) \
            fb->CV = (C)(fb->CV - 1); \
        fb->Q = (IEC_BOOL)(fb->CV <= 0); \
    } \
    typedef struct { \
        IEC_BOOL CU, CD, R, LD; \
        C PV; \
        IEC_BOOL QU, QD; \
        C CV; \
        IEC_BOOL CU_, CD_; \
    } iecf_CTUD##S##_t; \
    static inline void iecf_CTUD##S##_init(iecf_CTUD##S##_t *fb) { \
        fb->CU = fb->CD = fb->R = fb->LD = fb->QU = fb->QD = fb->CU_ = fb->CD_ = 0; \
        fb->PV = fb->CV = 0; \
    } \
    static inline void iecf_CTUD##S(iecf_CTUD##S##_t *fb) { \
        IEC_BOOL up = (IEC_BOOL)(fb->CU & (!fb->CU_)), down = (IEC_BOOL)(fb->CD & (!fb->CD_)); \
        fb->CU_ = fb->CU; \
        fb->CD_ = fb->CD; \
        if (fb->R) \
            fb->CV = 0; \
        else if (fb->LD) \
            fb->CV = fb->PV; \
        else if (!(up && down)) { \
            if (up && fb->CV < (HI)) \
                fb->CV = (C)(fb->CV + 1); \
            else if (down && fb->CV > (LO)) \
                fb->CV = (C)(fb->CV - 1); \
        } \
        fb->QU = (IEC_BOOL)(fb->CV >= fb->PV); \
        fb->QD = (IEC_BOOL)(fb->CV <= 0); \
    }
/* PVmax and PVmin are the CV type's own range: a counter stops at its
 * type's ends, never wraps (decided 2026-11-06, as Siemens; docs/dependencies.md). */
IECF_COUNTERS(, IEC_INT, INT16_MIN, INT16_MAX)
IECF_COUNTERS(_DINT, IEC_DINT, INT32_MIN, INT32_MAX)
IECF_COUNTERS(_LINT, IEC_LINT, INT64_MIN, INT64_MAX)
IECF_COUNTERS(_UDINT, IEC_UDINT, 0u, UINT32_MAX)
IECF_COUNTERS(_ULINT, IEC_ULINT, 0u, UINT64_MAX)

/* Timers (Table 46), against the time the platform passes the task
 * (docs/c-output.md §3). ET counts from the edge that starts the timer
 * and stops at PT; a negative PT counts as 0. State 0 idle, 1 timing, 2
 * done. */
#define IECF_TIMERS(S, C, UNIT) \
    typedef struct { \
        IEC_BOOL IN; \
        C PT; \
        IEC_BOOL Q; \
        C ET; \
        uint8_t state_; \
        IEC_BOOL in_; \
        int64_t start_; \
    } iecf_TON##S##_t, iecf_TOF##S##_t, iecf_TP##S##_t; \
    static inline void iecf_TON##S##_init(iecf_TON##S##_t *fb) { \
        fb->IN = fb->Q = fb->in_ = 0; \
        fb->PT = fb->ET = 0; \
        fb->state_ = 0; \
        fb->start_ = 0; \
    } \
    static inline void iecf_TOF##S##_init(iecf_TOF##S##_t *fb) { \
        iecf_TON##S##_init(fb); \
    } \
    static inline void iecf_TP##S##_init(iecf_TP##S##_t *fb) { \
        iecf_TON##S##_init(fb); \
    } \
    /* Whether PT has passed since start_, and ET if not. */ \
    static inline int iecf_elapsed##S(const iecf_TON##S##_t *fb, int64_t now, C *et) { \
        uint64_t ns = now > fb->start_ ? (uint64_t)now - (uint64_t)fb->start_ : 0u; \
        uint64_t el = ns / (UNIT); \
        if (fb->PT <= 0 || el >= (uint64_t)fb->PT) return 1; \
        *et = (C)el; \
        return 0; \
    } \
    static inline void iecf_TON##S(iecf_TON##S##_t *fb, int64_t now) { \
        if (!fb->IN) { \
            fb->Q = 0; \
            fb->ET = 0; \
            fb->state_ = 0; \
        } else { \
            if (fb->state_ == 0) { \
                fb->state_ = 1; \
                fb->start_ = now; \
            } \
            if (fb->state_ == 1 && iecf_elapsed##S(fb, now, &fb->ET)) { \
                fb->state_ = 2; \
                fb->Q = 1; \
                fb->ET = fb->PT > 0 ? fb->PT : 0; \
            } \
        } \
        fb->in_ = fb->IN; \
    } \
    static inline void iecf_TOF##S(iecf_TOF##S##_t *fb, int64_t now) { \
        if (fb->IN) { \
            fb->ET = 0; \
            fb->state_ = 0; \
        } else { \
            if (fb->state_ == 0 && fb->in_) { \
                fb->state_ = 1; \
                fb->start_ = now; \
            } \
            if (fb->state_ == 1 && iecf_elapsed##S(fb, now, &fb->ET)) { \
                fb->state_ = 2; \
                fb->ET = fb->PT > 0 ? fb->PT : 0; \
            } \
        } \
        fb->Q = (IEC_BOOL)(fb->IN | (fb->state_ == 1)); \
        fb->in_ = fb->IN; \
    } \
    static inline void iecf_TP##S(iecf_TP##S##_t *fb, int64_t now) { \
        if (fb->state_ == 0 && fb->IN && !fb->in_) { \
            fb->state_ = 1; \
            fb->Q = 1; \
            fb->start_ = now; \
        } \
        if (fb->state_ == 1 && iecf_elapsed##S(fb, now, &fb->ET)) { \
            fb->state_ = 2; \
            fb->Q = 0; \
            fb->ET = fb->PT > 0 ? fb->PT : 0; \
        } \
        if (fb->state_ == 2 && !fb->IN) { \
            fb->ET = 0; \
            fb->state_ = 0; \
        } \
        fb->in_ = fb->IN; \
    }
IECF_TIMERS(, IEC_TIME, UINT64_C(1000000))
IECF_TIMERS(_LTIME, IEC_LTIME, UINT64_C(1))

#endif /* IECF_H */
