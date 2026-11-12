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
#define IECF_FAULT_SFC 11u           /* an SFC step activated while active, or an action's associations in conflict: a flag */
#define IECF_FAULT_NULL 12u          /* a NULL reference dereferenced (Ed.3 6.4.4.10.3) */

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
#define IECF_KIND_REF 5u    /* REF_TO `of`: an iecf_ref_t */

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
#define IECF_T_DATE 19u    /* seconds since 1970-01-01, at midnight */
#define IECF_T_TOD 20u     /* ms since midnight */
#define IECF_T_DT 21u      /* seconds since 1970-01-01 00:00 */
#define IECF_T_LDATE 22u   /* ns since 1970-01-01, at midnight */
#define IECF_T_LTOD 23u    /* ns since midnight */
#define IECF_T_LDT 24u     /* ns since 1970-01-01 00:00 */
#define IECF_T_CHAR 25u    /* a Latin-1 character, its code in a byte */
#define IECF_T_WCHAR 26u   /* a UTF-16 unit */
#define IECF_T_WSTRING 27u /* `size` is 2(n + 1): up to n UTF-16 units, then a 0 */

#define IECF_SYM_INPUT 1u    /* a VAR_INPUT */
#define IECF_SYM_OUTPUT 2u   /* a VAR_OUTPUT */
#define IECF_SYM_CONSTANT 4u /* declared CONSTANT: not to be written */
#define IECF_SYM_RETAIN 8u   /* in ctx->retain: kept across a restart */
#define IECF_SYM_PACKED 16u  /* an array, structure or STRING at an address:
                                its parts back to back from its byte, each
                                its type's width, a BOOL a byte, little-endian;
                                not at the type table's offsets */

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

/* A located variable not at a bit: a BOOL there takes the whole byte. */
#define IECF_NO_BIT 0xFFu

/* An INTERFACE as the run knows it: its name. Its address is what an
   instance's type is searched for (iecf_itf_of). */
typedef struct iecf_itype {
    const char *name; /* upper-cased, with its namespaces */
} iecf_itype_t;

/* An interface a type implements, and that type's table for it: a
   structure of function pointers the generated C declares
   (`IT_<ITF>_t`), each taking the instance as `void *self`. */
typedef struct {
    const iecf_itype_t *itf;
    const void *table;
} iecf_itfimpl_t;

/* A CLASS's or FUNCTION_BLOCK's type as the run knows it, when it has a
   base or a derived type, or implements an interface (OO.md §4): its
   base's, its name, its table of methods, a structure of function
   pointers the generated C declares (`VT_<NAME>_t`), and the interfaces
   it implements, its bases' and those they extend too, each with its
   table. Each instance of such a type points to its own, first in its
   structure, set by its `_init`: a call through THIS, a VAR_IN_OUT, a
   reference or an interface runs the method of the instance's type
   (Ed.3 6.6.5.6, 6.6.6.5.2). */
typedef struct iecf_otype {
    const struct iecf_otype *base; /* NULL for the root of a hierarchy */
    const char *name;              /* upper-cased, with its namespaces */
    const void *table;
    const iecf_itfimpl_t *itfs;
    uint32_t itf_count;
} iecf_otype_t;

typedef struct {
    const char *path;   /* upper-cased */
    uint32_t type;      /* index into <p>_types */
    uint32_t offset;    /* in the context; in the process image, the byte */
    char area;          /* 0 for the context; 'I', 'Q' or 'M' */
    uint8_t bit;        /* a BOOL's bit; IECF_NO_BIT when not at a bit */
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
/* As iecf_bytes, the two allowed to overlap: an array or structure at an
   address copied to another whose bytes it shares. */
static inline void iecf_move(void *to, const void *from, size_t n) {
    unsigned char *t = (unsigned char *)to;
    const unsigned char *f = (const unsigned char *)from;
    if (t < f) {
        while (n--) *t++ = *f++;
    } else if (t > f) {
        while (n--) t[n] = f[n];
    }
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

/* ---- variable-length arrays (Ed.3 6.5.3) ---------------------------------
 * An ARRAY[*] parameter is a pointer to an iecf_flex_t: the caller's array,
 * and each dimension's lower and upper bound in turn. */
typedef struct {
    void *p;
    const int64_t *b;
} iecf_flex_t;

/* The count of elements of dimension d (from 0). */
static inline uint32_t iecf_flex_n(const iecf_flex_t *a, unsigned d) {
    return (uint32_t)(a->b[2u * d + 1u] - a->b[2u * d] + 1);
}

/* An unsigned dimension, as a signed one: past INT64_MAX it is out of
 * any array's dimensions anyway. */
static inline int64_t iecf_dim_u(uint64_t d) {
    return d > (uint64_t)INT64_MAX ? 0 : (int64_t)d;
}

/* LOWER_BOUND or UPPER_BOUND (Table 15) of dimension `dim` (from 1) of an
 * array of `n` dimensions with bounds `b`: outside 1..n a fault, giving 0. */
static inline int32_t iecf_bound(iecf_fault_t *f, int64_t dim, int64_t n, const int64_t *b, int upper) {
    if (dim < 1 || dim > n) {
        iecf_fail(f, IECF_FAULT_INDEX);
        return 0;
    }
    return (int32_t)b[2 * (dim - 1) + (upper ? 1 : 0)];
}

/* ---- references (Ed.3 6.4.4.10, docs/c-output.md §4) --------------------
 * REF_TO anything is an iecf_ref_t: a C pointer to a variable in the
 * context (`area` 0), or the area, byte and bit of a variable in the
 * process image, found through ctx->image at each use - the platform may
 * move the image between runs. NULL is area 0 and no pointer. */
typedef struct {
    void *p;        /* area 0: the variable; NULL for NULL */
    uint32_t off;   /* in the image: the byte */
    uint8_t area;   /* 0, or 'I', 'Q', 'M' */
    uint8_t bit;    /* in the image, a BOOL at a bit: 0 to 7; else IECF_NO_BIT */
} iecf_ref_t;

static inline iecf_ref_t iecf_ref_null(void) {
    iecf_ref_t r;
    r.p = NULL;
    r.off = 0u;
    r.area = 0u;
    r.bit = IECF_NO_BIT;
    return r;
}
static inline iecf_ref_t iecf_ref_c(void *p) {
    iecf_ref_t r = iecf_ref_null();
    r.p = p;
    return r;
}
static inline iecf_ref_t iecf_ref_image(uint8_t area, uint32_t off, uint8_t bit) {
    iecf_ref_t r = iecf_ref_null();
    r.area = area;
    r.off = off;
    r.bit = bit;
    return r;
}
static inline int iecf_ref_is_null(iecf_ref_t r) {
    return r.area == 0u && r.p == NULL;
}
/* Equal when they reference the same variable (Ed.3 Table 12). */
static inline IEC_BOOL iecf_ref_eq(iecf_ref_t a, iecf_ref_t b) {
    if (a.area != b.area) return 0;
    if (a.area == 0u) return (IEC_BOOL)(a.p == b.p);
    return (IEC_BOOL)(a.off == b.off && a.bit == b.bit);
}
/* `r^`: a NULL reference is a fault (PROPOSAL.md §6.3, as an index out
 * of range), and gives `nowhere`, scratch memory of the program's size
 * for any target, while the statement is stopped before it stores. */
static inline iecf_ref_t iecf_deref(iecf_fault_t *f, iecf_ref_t r, void *nowhere) {
    if (iecf_ref_is_null(r)) {
        iecf_fail(f, IECF_FAULT_NULL);
        return iecf_ref_c(nowhere);
    }
    return r;
}
/* ---- interfaces (Ed.3 6.6.6.5, docs/c-output.md §4) -------------------
 * A variable of an INTERFACE is an iecf_itf_t: the instance it
 * references and that instance's type's table for the interface; NULL
 * when `self` is. An instance whose type implements an interface starts
 * with the pointer to its type (OO.md §4), whatever its type: `self`
 * read as one finds it. */
typedef struct {
    void *self;
    const void *table;
} iecf_itf_t;

static inline iecf_itf_t iecf_itf_null(void) {
    iecf_itf_t i;
    i.self = NULL;
    i.table = NULL;
    return i;
}
/* An instance whose type is known where it is given, and its table. */
static inline iecf_itf_t iecf_itf_make(void *self, const void *table) {
    iecf_itf_t i;
    i.self = self;
    i.table = table;
    return i;
}
/* The type of an instance of a type that has one (OO.md §4). */
static inline const iecf_otype_t *iecf_type_of(const void *self) {
    return *(const iecf_otype_t *const *)self;
}
/* The instance `self` as the interface `want`: its type's table for it,
   NULL when its type does not implement it. */
static inline iecf_itf_t iecf_itf_of(void *self, const iecf_itype_t *want) {
    const iecf_otype_t *t = self == NULL ? NULL : iecf_type_of(self);
    uint32_t k;
    if (t == NULL) return iecf_itf_null();
    for (k = 0u; k < t->itf_count; k++) {
        if (t->itfs[k].itf == want) return iecf_itf_make(self, t->itfs[k].table);
    }
    return iecf_itf_null();
}
/* `a ?= b` of interfaces, and `a := b` where b's extends a's: b's
   instance as `want`, NULL when it is NULL or does not implement it
   (Ed.3 6.6.6.7). */
static inline iecf_itf_t iecf_itf_as(iecf_itf_t from, const iecf_itype_t *want) {
    return iecf_itf_of(from.self, want);
}
/* Equal when they reference the same instance or are both NULL (Ed.3
   6.6.6.5.1). */
static inline IEC_BOOL iecf_itf_eq(iecf_itf_t a, iecf_itf_t b) {
    return (IEC_BOOL)(a.self == b.self);
}
/* Whether `t` is `want` or derives from it. */
static inline int iecf_otype_is(const iecf_otype_t *t, const iecf_otype_t *want) {
    for (; t != NULL; t = t->base) {
        if (t == want) return 1;
    }
    return 0;
}
/* `r ?= i`, `r` a REF_TO `want`: the instance `i` references, if it is a
   `want` or of a type derived from it; else NULL (Ed.3 6.6.6.7). */
static inline iecf_ref_t iecf_itf_ref(iecf_itf_t i, const iecf_otype_t *want) {
    if (i.self != NULL && iecf_otype_is(iecf_type_of(i.self), want)) return iecf_ref_c(i.self);
    return iecf_ref_null();
}
/* `r ?= b` of references to a CLASS or FUNCTION_BLOCK: `b` if what it
   references is a `want` or of a type derived from it; else NULL. An
   instance is never in the process image. */
static inline iecf_ref_t iecf_ref_as(iecf_ref_t b, const iecf_otype_t *want) {
    if (b.area == 0u && b.p != NULL && iecf_otype_is(iecf_type_of(b.p), want)) return b;
    return iecf_ref_null();
}

/* Part `n` of what `r` references, parts `c` bytes apart in C memory
 * and `packed` in the image (an element; a field: n 1 and its offset). */
static inline iecf_ref_t iecf_ref_at(iecf_ref_t r, uint32_t n, size_t c, uint32_t packed) {
    if (r.area == 0u)
        r.p = (void *)((char *)r.p + (size_t)n * c);
    else
        r.off += n * packed;
    return r;
}
/* The bytes of the image area `r` is in. */
static inline uint8_t *iecf_ref_area(const iecf_image_t *im, iecf_ref_t r) {
    return r.area == 'I' ? im->i : r.area == 'Q' ? im->q : im->m;
}
/* Where `r`'s bytes start: a STRING's, the same in C and in the image. */
static inline void *iecf_ref_ptr(const iecf_image_t *im, iecf_ref_t r) {
    return r.area == 0u ? r.p : (void *)(iecf_ref_area(im, r) + r.off);
}

/* `iecf_rget_T` and `iecf_rset_T`: an elementary T through a reference,
 * in C memory as itself, in the image as a variable there is read and
 * written (little-endian, its type's width). */
#define IECF_REF_ACCESS(T, C, GET, SET) \
    static inline C iecf_rget_##T(const iecf_image_t *im, iecf_ref_t r) { \
        const uint8_t *b; \
        if (r.area == 0u) return *(const C *)r.p; \
        b = iecf_ref_area(im, r); \
        return GET; \
    } \
    static inline void iecf_rset_##T(const iecf_image_t *im, iecf_ref_t r, C v) { \
        uint8_t *b; \
        if (r.area == 0u) { \
            *(C *)r.p = v; \
            return; \
        } \
        b = iecf_ref_area(im, r); \
        SET; \
    }
IECF_REF_ACCESS(BOOL, IEC_BOOL,
    (IEC_BOOL)(r.bit != IECF_NO_BIT ? iecf_get_x(b, r.off, r.bit) : (iecf_get_b(b, r.off) != 0u)),
    if (r.bit != IECF_NO_BIT) iecf_set_x(b, r.off, r.bit, v); else iecf_set_b(b, r.off, (uint8_t)(v ? 1u : 0u)))
IECF_REF_ACCESS(SINT, IEC_SINT, iecf_to_i8(iecf_get_b(b, r.off)), iecf_set_b(b, r.off, (uint8_t)v))
IECF_REF_ACCESS(USINT, IEC_USINT, iecf_get_b(b, r.off), iecf_set_b(b, r.off, v))
IECF_REF_ACCESS(BYTE, IEC_BYTE, iecf_get_b(b, r.off), iecf_set_b(b, r.off, v))
IECF_REF_ACCESS(CHAR, IEC_CHAR, iecf_get_b(b, r.off), iecf_set_b(b, r.off, v))
IECF_REF_ACCESS(INT, IEC_INT, iecf_to_i16(iecf_get_w(b, r.off)), iecf_set_w(b, r.off, (uint16_t)v))
IECF_REF_ACCESS(UINT, IEC_UINT, iecf_get_w(b, r.off), iecf_set_w(b, r.off, v))
IECF_REF_ACCESS(WORD, IEC_WORD, iecf_get_w(b, r.off), iecf_set_w(b, r.off, v))
IECF_REF_ACCESS(WCHAR, IEC_WCHAR, iecf_get_w(b, r.off), iecf_set_w(b, r.off, v))
IECF_REF_ACCESS(DINT, IEC_DINT, iecf_to_i32(iecf_get_d(b, r.off)), iecf_set_d(b, r.off, (uint32_t)v))
IECF_REF_ACCESS(UDINT, IEC_UDINT, iecf_get_d(b, r.off), iecf_set_d(b, r.off, v))
IECF_REF_ACCESS(DWORD, IEC_DWORD, iecf_get_d(b, r.off), iecf_set_d(b, r.off, v))
IECF_REF_ACCESS(TIME, IEC_TIME, iecf_to_i32(iecf_get_d(b, r.off)), iecf_set_d(b, r.off, (uint32_t)v))
IECF_REF_ACCESS(DATE, IEC_DATE, iecf_get_d(b, r.off), iecf_set_d(b, r.off, v))
IECF_REF_ACCESS(TOD, IEC_TOD, iecf_get_d(b, r.off), iecf_set_d(b, r.off, v))
IECF_REF_ACCESS(DT, IEC_DT, iecf_get_d(b, r.off), iecf_set_d(b, r.off, v))
IECF_REF_ACCESS(LINT, IEC_LINT, iecf_to_i64(iecf_get_l(b, r.off)), iecf_set_l(b, r.off, (uint64_t)v))
IECF_REF_ACCESS(ULINT, IEC_ULINT, iecf_get_l(b, r.off), iecf_set_l(b, r.off, v))
IECF_REF_ACCESS(LWORD, IEC_LWORD, iecf_get_l(b, r.off), iecf_set_l(b, r.off, v))
IECF_REF_ACCESS(LTIME, IEC_LTIME, iecf_to_i64(iecf_get_l(b, r.off)), iecf_set_l(b, r.off, (uint64_t)v))
IECF_REF_ACCESS(LDATE, IEC_LDATE, iecf_to_i64(iecf_get_l(b, r.off)), iecf_set_l(b, r.off, (uint64_t)v))
IECF_REF_ACCESS(LTOD, IEC_LTOD, iecf_to_i64(iecf_get_l(b, r.off)), iecf_set_l(b, r.off, (uint64_t)v))
IECF_REF_ACCESS(LDT, IEC_LDT, iecf_to_i64(iecf_get_l(b, r.off)), iecf_set_l(b, r.off, (uint64_t)v))
IECF_REF_ACCESS(REAL, IEC_REAL, iecf_get_real(b, r.off), iecf_set_real(b, r.off, v))
IECF_REF_ACCESS(LREAL, IEC_LREAL, iecf_get_lreal(b, r.off), iecf_set_lreal(b, r.off, v))

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
/* A BOOL is a bit string of one bit (Ed.3 Table 33, ANY_BIT): shifted
 * by one or more it is 0, rotated it is itself. */
IECF_SHIFTS(BOOL, IEC_BOOL, 1u)
IECF_SHIFTS(BYTE, uint8_t, 8u)
IECF_SHIFTS(WORD, uint16_t, 16u)
IECF_SHIFTS(DWORD, uint32_t, 32u)
IECF_SHIFTS(LWORD, uint64_t, 64u)

/* ---- FOR (Ed.3 7.3.3.4.2) -----------------------------------------------
 * done: whether the loop runs not even once. next: the step, and whether
 * it passed the end - so after the loop the variable is one step past
 * its end, as Ed.3's examples have it ("terminates when the value of the
 * variable I reaches 0") and CODESYS (decided 2026-11-07,
 * docs/dependencies.md). The last step is the type's own addition: past
 * the type's limit it wraps and is flagged, and the loop still ends - a
 * loop to the type's last value does not run forever. A step of 0 never
 * ends, as written. The difference end - i is exact in 64 unsigned bits,
 * as i is between start and end.
 * next_in, for a subrange variable: the loop ends at its last value
 * instead, which stays inside the subrange. */
#define IECF_FOR_SIGNED(T, C) \
    static inline int iecf_for_done_##T(C i, C end, C step) { return step >= 0 ? i > end : i < end; } \
    static inline int iecf_for_past_##T(C i, C end, C step) { \
        uint64_t e = (uint64_t)(int64_t)end, v = (uint64_t)(int64_t)i, s = (uint64_t)(int64_t)step; \
        return step >= 0 ? e - v < s : v - e < (uint64_t)0 - s; \
    } \
    static inline int iecf_for_next_##T(iecf_fault_t *g, C *i, C end, C step) { \
        int past = iecf_for_past_##T(*i, end, step); \
        *i = iecf_add_##T(g, *i, step); \
        return past; \
    } \
    static inline int iecf_for_next_in_##T(C *i, C end, C step) { \
        if (iecf_for_past_##T(*i, end, step)) return 1; \
        *i = (C)((int64_t)*i + (int64_t)step); \
        return 0; \
    }
#define IECF_FOR_UNSIGNED(T, C) \
    static inline int iecf_for_done_##T(C i, C end, C step) { \
        (void)step; \
        return i > end; \
    } \
    static inline int iecf_for_next_##T(iecf_fault_t *g, C *i, C end, C step) { \
        int past = (uint64_t)end - (uint64_t)*i < (uint64_t)step; \
        *i = iecf_add_##T(g, *i, step); \
        return past; \
    } \
    static inline int iecf_for_next_in_##T(C *i, C end, C step) { \
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

/* ---- selection (Ed.3 Table 32): SEL, MIN, MAX, LIMIT, MUX ---------------
 * A function, so both of SEL's inputs are evaluated, as any function's. */
/* A ULINT K for MUX: past LINT it is out of range anyway. */
static inline int64_t iecf_mux_k(uint64_t k) { return k > (uint64_t)INT64_MAX ? -1 : (int64_t)k; }
#define IECF_SELECT(T, C) \
    static inline C iecf_sel_##T(uint8_t g, C in0, C in1) { return g ? in1 : in0; } \
    static inline C iecf_min_##T(C a, C b) { return (C)(b < a ? b : a); } \
    static inline C iecf_max_##T(C a, C b) { return (C)(b > a ? b : a); } \
    static inline C iecf_limit_##T(C mn, C in, C mx) { return iecf_min_##T(iecf_max_##T(in, mn), mx); } \
    /* MUX: in[k], every input evaluated by the caller; K outside 0 .. n-1 \
     * faults (Ed.3 Table 32 e) and gives in[0]. */ \
    static inline C iecf_mux_##T(iecf_fault_t *f, int64_t k, uint32_t n, const C *in) { \
        if (k < 0 || k >= (int64_t)n) { \
            iecf_fail(f, IECF_FAULT_MUX); \
            return in[0]; \
        } \
        return in[k]; \
    }
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
/* The functions of a string type, made twice by IECF_STRINGS below:
 * STRING's, iecf_slen ..., on `char`, and WSTRING's, iecf_wslen ..., on
 * IEC_WCHAR, the same code. W is the names' infix, C the unit, UC the
 * unit compared, CH the character type. */
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
#define IECF_STRINGS(W, C, UC, CH) \
/* A CHAR as a STRING of one character (Ed.3 Table 27, CHAR_TO_STRING; \
   Figure 11's implicit CHAR to STRING), in `b`, two bytes: '$00' gives \
   the empty string, as '$00' ends any string. */ \
static inline const C *iecf_##W##char_str(C *b, CH c) { \
    b[0] = (C)c; \
    b[1] = 0; \
    return b; \
} \
 \
/* STRING_TO_CHAR (Ed.3 Table 27): the first character; an empty string, \
   for which the standard leaves the result undefined, gives CHAR's \
   initial value '$00', flagged CONVERSION (docs/dependencies.md). */ \
static inline CH iecf_##W##string_char(iecf_fault_t *g, const C *s) { \
    if (s[0] == 0) { \
        iecf_flag(g, IECF_FAULT_CONVERSION); \
        return 0u; \
    } \
    return (CH)(UC)s[0]; \
} \
 \
static inline uint32_t iecf_##W##slen(const C *s) { \
    uint32_t n = 0u; \
    while (s[n] != 0) n++; \
    return n; \
} \
/* s[i], the i-th character of a string, from 1 (Ed.3 6.4.2). Read: \
   STRING_TO_CHAR(MID(s, 1, i)), as the standard defines it; a position \
   not in the string gives MID's empty string, flagged STRING_POSITION, \
   and so '$00'. */ \
static inline CH iecf_##W##schar(iecf_fault_t *g, const C *s, int64_t i) { \
    if (i < 1 || (uint64_t)i > iecf_##W##slen(s)) { \
        iecf_flag(g, IECF_FAULT_STRING_POSITION); \
        return 0u; \
    } \
    return (CH)(UC)s[i - 1]; \
} \
/* s[i] := c: the character at i replaced; '$00' there ends the string \
   at i, as '$00' ends any string (CODESYS and TwinCAT cut there too). A \
   position not in the string, below 1 or past its end, leaves s as it \
   is, flagged STRING_POSITION (both decided, florin, 2026-11-09). */ \
static inline void iecf_##W##sput(iecf_fault_t *g, C *s, int64_t i, CH c) { \
    if (i < 1 || (uint64_t)i > iecf_##W##slen(s)) { \
        iecf_flag(g, IECF_FAULT_STRING_POSITION); \
        return; \
    } \
    s[i - 1] = (C)c; \
} \
static inline int iecf_##W##scmp(const C *a, const C *b) { \
    const UC *x = (const UC *)a, *y = (const UC *)b; \
    while (*x != 0u && *x == *y) { \
        x++; \
        y++; \
    } \
    return (*x > *y) - (*x < *y); \
} \
/* SEL, MIN, MAX, LIMIT, MUX of strings (Ed.3 Tables 31, 32), as of \
   the other types: every input evaluated by the caller, a string by \
   where it is; MIN and MAX by iecf_scmp; MUX's K outside 0 .. n-1 \
   faults and gives in[0]. */ \
static inline const C *iecf_##W##ssel(uint8_t g, const C *in0, const C *in1) { return g ? in1 : in0; } \
static inline const C *iecf_##W##smin(const C *a, const C *b) { return iecf_##W##scmp(b, a) < 0 ? b : a; } \
static inline const C *iecf_##W##smax(const C *a, const C *b) { return iecf_##W##scmp(b, a) > 0 ? b : a; } \
static inline const C *iecf_##W##slimit(const C *mn, const C *in, const C *mx) { \
    return iecf_##W##smin(iecf_##W##smax(in, mn), mx); \
} \
static inline const C *iecf_##W##smux(iecf_fault_t *f, int64_t k, uint32_t n, const C *const *in) { \
    if (k < 0 || k >= (int64_t)n) { \
        iecf_fail(f, IECF_FAULT_MUX); \
        return in[0]; \
    } \
    return in[k]; \
} \
/* A STRING(n)'s initial value, which fits: the rest of it zeros. */ \
static inline void iecf_##W##sinit(C *d, uint32_t n, const C *s) { \
    uint32_t i = 0u; \
    for (; s[i] != 0 && i < n; i++) d[i] = s[i]; \
    for (; i <= n; i++) d[i] = 0; \
} \
/* d := s for d a STRING(n): longer than n, cut to n and flagged \
 * IECF_FAULT_STRING_LENGTH (g: &ctx->flag). s may be d. */ \
static inline C *iecf_##W##scopy(iecf_fault_t *g, C *d, uint32_t n, const C *s) { \
    uint32_t i = 0u; \
    for (; s[i] != 0 && i < n; i++) d[i] = s[i]; \
    if (s[i] != 0) iecf_flag(g, IECF_FAULT_STRING_LENGTH); \
    d[i] = 0; \
    return d; \
} \
/* a then b into d, n characters at most. */ \
static inline const C *iecf_##W##concat(iecf_fault_t *g, C *d, uint32_t n, const C *a, const C *b) { \
    uint32_t i = 0u; \
    for (; *a != 0 && i < n; a++) d[i++] = *a; \
    for (; *b != 0 && i < n; b++) d[i++] = *b; \
    if (*a != 0 || *b != 0) iecf_flag(g, IECF_FAULT_STRING_LENGTH); \
    d[i] = 0; \
    return d; \
} \
/* LEFT, RIGHT, MID, DELETE, INSERT, REPLACE (Ed.3 Table 34), positions \
 * from 1. A length or position the standard calls an error - negative, a \
 * position not in the string, a length past its end - gives what Siemens \
 * gives (S7-1200 System Manual 8.2.4, its ENO tables) and is flagged \
 * IECF_FAULT_STRING_POSITION (g: &ctx->flag); the task goes on (decided \
 * 2026-11-06, docs/dependencies.md): \
 *   LEFT, RIGHT: L < 0 empty; L past the end all of IN. \
 *   MID: L < 0 or P < 1 empty; from P to the end if L runs past it. \
 *   DELETE: L < 0 or P < 1 empty; P past the end IN unchanged; L past \
 *     the end deletes to the end. \
 *   INSERT: P < 0 empty; P past the end appends IN2. \
 *   REPLACE: L < 0 or P < 1 empty; P past the end appends IN2; L past the \
 *     end replaces to the end. */ \
static inline uint32_t iecf_##W##put(C *d, uint32_t i, uint32_t n, const C *s, uint32_t k) { \
    uint32_t j; \
    for (j = 0u; j < k && s[j] != 0 && i < n; j++) d[i++] = s[j]; \
    return i; \
} \
static inline const C *iecf_##W##bad(iecf_fault_t *g, C *d) { \
    iecf_flag(g, IECF_FAULT_STRING_POSITION); \
    d[0] = 0; \
    return d; \
} \
static inline const C *iecf_##W##left(iecf_fault_t *g, C *d, uint32_t n, const C *in, int64_t l) { \
    uint32_t m = iecf_##W##slen(in), k; \
    if (l < 0) return iecf_##W##bad(g, d); \
    k = iecf_upto(g, l, 0u, m); \
    d[iecf_##W##put(d, 0u, n, in, k)] = 0; \
    return d; \
} \
static inline const C *iecf_##W##right(iecf_fault_t *g, C *d, uint32_t n, const C *in, int64_t l) { \
    uint32_t m = iecf_##W##slen(in), k; \
    if (l < 0) return iecf_##W##bad(g, d); \
    k = iecf_upto(g, l, 0u, m); \
    d[iecf_##W##put(d, 0u, n, in + (m - k), k)] = 0; \
    return d; \
} \
static inline const C *iecf_##W##mid(iecf_fault_t *g, C *d, uint32_t n, const C *in, int64_t l, int64_t p) { \
    uint32_t m = iecf_##W##slen(in), k; \
    if (l < 0 || p < 1) return iecf_##W##bad(g, d); \
    if (l == 0) { /* no character taken */ \
        d[0] = 0; \
        return d; \
    } \
    if ((uint64_t)p > (uint64_t)m) return iecf_##W##bad(g, d); \
    k = iecf_upto(g, l, (uint32_t)(p - 1), m); \
    d[iecf_##W##put(d, 0u, n, in + (p - 1), k)] = 0; \
    return d; \
} \
static inline const C *iecf_##W##delete(iecf_fault_t *g, C *d, uint32_t n, const C *in, int64_t l, int64_t p) { \
    uint32_t m = iecf_##W##slen(in), k, i; \
    if (l < 0 || p < 1) return iecf_##W##bad(g, d); \
    if ((uint64_t)p > (uint64_t)m) { \
        if (l != 0) iecf_flag(g, IECF_FAULT_STRING_POSITION); \
        d[iecf_##W##put(d, 0u, n, in, m)] = 0; \
        return d; \
    } \
    k = iecf_upto(g, l, (uint32_t)(p - 1), m); \
    i = iecf_##W##put(d, 0u, n, in, (uint32_t)(p - 1)); \
    d[iecf_##W##put(d, i, n, in + (p - 1) + k, m)] = 0; \
    return d; \
} \
static inline const C *iecf_##W##insert(iecf_fault_t *g, C *d, uint32_t n, const C *a, const C *b, int64_t p) { \
    uint32_t m = iecf_##W##slen(a), at, i; \
    if (p < 0) return iecf_##W##bad(g, d); \
    at = (uint32_t)p; \
    if ((uint64_t)p > (uint64_t)m) { \
        iecf_flag(g, IECF_FAULT_STRING_POSITION); \
        at = m; \
    } \
    i = iecf_##W##put(d, 0u, n, a, at); \
    i = iecf_##W##put(d, i, n, b, UINT32_MAX); \
    i = iecf_##W##put(d, i, n, a + at, m); \
    if ((uint64_t)m + iecf_##W##slen(b) > n) iecf_flag(g, IECF_FAULT_STRING_LENGTH); \
    d[i] = 0; \
    return d; \
} \
static inline const C *iecf_##W##replace(iecf_fault_t *g, C *d, uint32_t n, const C *a, const C *b, int64_t l, int64_t p) { \
    uint32_t m = iecf_##W##slen(a), at, k, i; \
    if (l < 0 || p < 1) return iecf_##W##bad(g, d); \
    if ((uint64_t)p > (uint64_t)m) { \
        iecf_flag(g, IECF_FAULT_STRING_POSITION); \
        at = m; \
        k = 0u; \
    } else { \
        at = (uint32_t)(p - 1); \
        k = iecf_upto(g, l, at, m); \
    } \
    i = iecf_##W##put(d, 0u, n, a, at); \
    i = iecf_##W##put(d, i, n, b, UINT32_MAX); \
    i = iecf_##W##put(d, i, n, a + at + k, m); \
    if ((uint64_t)m - k + iecf_##W##slen(b) > n) iecf_flag(g, IECF_FAULT_STRING_LENGTH); \
    d[i] = 0; \
    return d; \
} \
/* FIND: where b first starts in a, from 1; 0 if nowhere, or b is empty. */ \
static inline uint32_t iecf_##W##find(const C *a, const C *b) { \
    uint32_t i, j; \
    if (*b == 0) return 0u; \
    for (i = 0u; a[i] != 0; i++) { \
        for (j = 0u; b[j] != 0 && a[i + j] == b[j]; j++) { \
        } \
        if (b[j] == 0) return i + 1u; \
    } \
    return 0u; \
} \

IECF_STRINGS(, char, unsigned char, IEC_CHAR)
IECF_STRINGS(w, IEC_WCHAR, IEC_WCHAR, IEC_WCHAR)

/* ---- WSTRING and WCHAR (docs/dependencies.md) ---------------------------
 * A WSTRING(n) is IEC_WCHAR[n + 1], UTF-16 units then a 0, made by the
 * same functions as STRING's, their names with a w: iecf_wslen,
 * iecf_wconcat ... A WCHAR is one unit; a character above U+FFFF is two
 * in a WSTRING, a surrogate pair, and LEN counts both (decided, florin,
 * 2026-11-10). A STRING is Latin-1, U+0000-U+00FF: those carry across
 * as they are, both ways (Ed.3 Table 27, rows 1, 3, 6, 8). Any other
 * character is skipped, as Beckhoff's WSTRING_TO_STRING2 and
 * WCHAR_TO_CHAR do, and flagged CONVERSION (decided, florin, 2026-11-10):
 * a surrogate pair, one character, skipped whole. */
static inline int iecf_w_high(IEC_WCHAR u) { return u >= 0xD800u && u <= 0xDBFFu; }
static inline int iecf_w_low(IEC_WCHAR u) { return u >= 0xDC00u && u <= 0xDFFFu; }
/* A WSTRING(n) at an address: its n + 1 units back to back from byte p
 * of the image, each little-endian, as any part there (docs/c-output.md
 * §4), at any byte. Read into d, a C copy; a statement that stores into
 * it changes that copy and writes it all back, so units after its 0
 * stay as they were, as a STRING's bytes there. */
static inline IEC_WCHAR *iecf_wimg_get(IEC_WCHAR *d, uint32_t n, const uint8_t *p) {
    uint32_t i;
    for (i = 0u; i <= n; i++) d[i] = iecf_get_w(p, 2u * i);
    return d;
}
static inline void iecf_wimg_put(uint8_t *p, uint32_t n, const IEC_WCHAR *s) {
    uint32_t i;
    for (i = 0u; i <= n; i++) iecf_set_w(p, 2u * i, s[i]);
}
/* A WSTRING(n) through a reference (Ed.3 6.4.4.10): in C memory its own
 * units, `d` untouched; in the image its n + 1 units copied into d. */
static inline IEC_WCHAR *iecf_wref_get(IEC_WCHAR *d, uint32_t n, const iecf_image_t *im, iecf_ref_t r) {
    if (r.area == 0u) return (IEC_WCHAR *)r.p;
    return iecf_wimg_get(d, n, iecf_ref_area(im, r) + r.off);
}
/* `s`'s n + 1 units into the WSTRING(n) `r` references; `s` may be what
 * iecf_wref_get gave for it. */
static inline void iecf_wref_put(const iecf_image_t *im, iecf_ref_t r, uint32_t n, const IEC_WCHAR *s) {
    uint32_t i;
    IEC_WCHAR *d;
    if (r.area != 0u) {
        iecf_wimg_put(iecf_ref_area(im, r) + r.off, n, s);
        return;
    }
    d = (IEC_WCHAR *)r.p;
    if (d == s) return;
    for (i = 0u; i <= n; i++) d[i] = s[i];
}
/* WSTRING_TO_STRING into d, n characters at most (the input's own n). */
static inline const char *iecf_wstr_str(iecf_fault_t *g, char *d, uint32_t n, const IEC_WCHAR *s) {
    uint32_t i = 0u;
    for (; *s != 0u; s++) {
        if (*s > 0xFFu) {
            iecf_flag(g, IECF_FAULT_CONVERSION);
            if (iecf_w_high(s[0]) && iecf_w_low(s[1])) s++;
            continue;
        }
        if (i < n) d[i++] = (char)*s;
    }
    d[i] = 0;
    return d;
}
/* STRING_TO_WSTRING into d, n units at most (the input's own n). */
static inline const IEC_WCHAR *iecf_str_wstr(IEC_WCHAR *d, uint32_t n, const char *s) {
    uint32_t i = 0u;
    for (; s[i] != 0 && i < n; i++) d[i] = (IEC_WCHAR)(unsigned char)s[i];
    d[i] = 0u;
    return d;
}
/* WCHAR_TO_CHAR: a unit above U+00FF gives '$00', flagged. */
static inline IEC_CHAR iecf_wchar_char(iecf_fault_t *g, IEC_WCHAR c) {
    if (c > 0xFFu) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return (IEC_CHAR)c;
}
/* WSTRING_TO_<T>, the text read as STRING_TO_<T> reads it: the WSTRING
 * made a STRING in `d`, n + 1 bytes. A character a STRING cannot hold is
 * no part of any literal, so it is not skipped but read as one that ends
 * the text: '?' in its place, which no literal takes. */
static inline const char *iecf_wstr_text(char *d, uint32_t n, const IEC_WCHAR *s) {
    uint32_t i = 0u;
    for (; s[i] != 0u && i < n; i++) d[i] = s[i] > 0x7Fu ? '?' : (char)s[i];
    d[i] = 0;
    return d;
}

/* ---- conversions to STRING (docs/dependencies.md) -----------------------
 * Each writes into d, which holds at least the longest text of its type,
 * and returns d. */
static inline const char *iecf_u_str(char *d, uint64_t v) {
    char t[21];
    uint32_t n = 0u, i = 0u;
    do {
        t[n++] = (char)('0' + (int)(v % 10u));
        v /= 10u;
    } while (v != 0u);
    while (n > 0u) d[i++] = t[--n];
    d[i] = 0;
    return d;
}
static inline const char *iecf_s_str(char *d, int64_t v) {
    if (v >= 0) return iecf_u_str(d, (uint64_t)v);
    d[0] = '-';
    iecf_u_str(d + 1, (uint64_t)0u - (uint64_t)v);
    return d;
}
static inline const char *iecf_bool_str(char *d, IEC_BOOL v) {
    const char *t = v ? "TRUE" : "FALSE";
    uint32_t i = 0u;
    for (; t[i] != 0; i++) d[i] = t[i];
    d[i] = 0;
    return d;
}

/* ---- decimal digits of a double, exactly --------------------------------
 * iecf_digits: the first `prec` (1..17) significant decimal digits of a
 * finite v > 0, rounded half to even on its exact value, into `d` as
 * '0'..'9', and *e10 so that v is about d[0].d[1]d[2]... x 10^*e10. No
 * floating point is used: v = f x 2^e exactly, which is the integer
 * f x 2^e, or f x 5^-e over 10^-e, held in base 10^9 (at most 86 limbs:
 * 5^1074 x 2^53 has 767 digits). The same digits on every target. */
#define IECF_DIGIT_LIMBS 90u
static inline void iecf_mul_small(uint32_t *l, uint32_t *n, uint32_t m) {
    uint64_t carry = 0u;
    uint32_t i;
    for (i = 0u; i < *n; i++) {
        uint64_t x = (uint64_t)l[i] * m + carry;
        l[i] = (uint32_t)(x % 1000000000u);
        carry = x / 1000000000u;
    }
    while (carry != 0u && *n < IECF_DIGIT_LIMBS) {
        l[(*n)++] = (uint32_t)(carry % 1000000000u);
        carry /= 1000000000u;
    }
}
static inline uint32_t iecf_digit_at(const uint32_t *l, uint32_t p) {
    uint32_t x = l[p / 9u], k;
    for (k = p % 9u; k > 0u; k--) x /= 10u;
    return x % 10u;
}
static inline void iecf_digits(double v, uint32_t prec, char *d, int32_t *e10) {
    uint32_t l[IECF_DIGIT_LIMBS], n = 0u, top, total, i, k;
    uint64_t bits, f;
    int32_t e2, scale = 0;
    int up = 0;
    iecf_bytes(&bits, &v, sizeof bits);
    f = bits & ((UINT64_C(1) << 52) - 1u);
    e2 = (int32_t)((bits >> 52) & 0x7FFu);
    if (e2 == 0) {
        e2 = -1074;
    } else {
        f |= UINT64_C(1) << 52;
        e2 -= 1075;
    }
    while (f != 0u) {
        l[n++] = (uint32_t)(f % 1000000000u);
        f /= 1000000000u;
    }
    for (; e2 >= 29; e2 -= 29) iecf_mul_small(l, &n, (uint32_t)1u << 29);
    if (e2 > 0) {
        iecf_mul_small(l, &n, (uint32_t)1u << e2);
        e2 = 0;
    }
    for (; e2 < 0; e2 += 12) {
        int32_t k5 = e2 < -12 ? 12 : -e2, j;
        uint32_t m = 1u;
        for (j = 0; j < k5; j++) m *= 5u;
        iecf_mul_small(l, &n, m);
        scale += k5;
    }
    for (top = l[n - 1u], total = 9u * (n - 1u); top != 0u; top /= 10u) total++;
    /* The digits from the most significant; the rest decides rounding. */
    for (i = 0u; i < prec; i++) d[i] = (char)('0' + (i < total ? iecf_digit_at(l, total - 1u - i) : 0u));
    if (prec < total) {
        uint32_t next = iecf_digit_at(l, total - 1u - prec);
        int rest = 0;
        for (k = 0u; k + 1u < total - prec && !rest; k++) rest = iecf_digit_at(l, k) != 0u;
        up = next > 5u || (next == 5u && (rest || ((d[prec - 1u] - '0') & 1)));
    }
    *e10 = (int32_t)total - 1 - scale;
    for (i = prec; up && i > 0u; i--) {
        if (d[i - 1u] == '9') {
            d[i - 1u] = '0';
        } else {
            d[i - 1u]++;
            up = 0;
        }
    }
    if (up) { /* 9...9 rounded up: 10...0 */
        d[0] = '1';
        (*e10)++;
    }
}

/* REAL_TO_STRING, LREAL_TO_STRING: C's %g, prec significant digits (7 for
 * a REAL, 15 for an LREAL - CODESYS and TwinCAT round a REAL's mantissa
 * to 6 places), correctly rounded, trailing zeros dropped: '2.5', '1',
 * '1.677722e+07', '1e-05'; 'inf', '-inf', 'nan'. 24 bytes hold any. */
static inline uint32_t iecf_cpy(char *d, uint32_t i, const char *t) {
    for (; *t != 0; t++) d[i++] = *t;
    d[i] = 0;
    return i;
}
static inline const char *iecf_real_str(char *d, double v, uint32_t prec) {
    char g[17];
    int32_t x;
    uint32_t i = 0u, k, last;
    uint64_t bits;
    iecf_bytes(&bits, &v, sizeof bits);
    if ((bits >> 63) != 0u) d[i++] = '-';
    if (v != v) {
        iecf_cpy(d, 0u, "nan");
        return d;
    }
    if (v > DBL_MAX || v < -DBL_MAX) {
        iecf_cpy(d, i, "inf");
        return d;
    }
    if (v == 0.0) {
        iecf_cpy(d, i, "0");
        return d;
    }
    iecf_digits(v < 0.0 ? -v : v, prec, g, &x);
    for (last = prec; last > 1u && g[last - 1u] == '0'; last--) {
    }
    if (x < -4 || x >= (int32_t)prec) {
        uint32_t e = (uint32_t)(x < 0 ? -x : x);
        d[i++] = g[0];
        if (last > 1u) {
            d[i++] = '.';
            for (k = 1u; k < last; k++) d[i++] = g[k];
        }
        d[i++] = 'e';
        d[i++] = x < 0 ? '-' : '+';
        if (e >= 100u) d[i++] = (char)('0' + (int)(e / 100u));
        d[i++] = (char)('0' + (int)(e / 10u % 10u));
        d[i++] = (char)('0' + (int)(e % 10u));
        d[i] = 0;
    } else if (x >= 0) {
        for (k = 0u; k <= (uint32_t)x; k++) d[i++] = k < last ? g[k] : '0';
        if (last > (uint32_t)x + 1u) {
            d[i++] = '.';
            for (; k < last; k++) d[i++] = g[k];
        }
        d[i] = 0;
    } else {
        d[i++] = '0';
        d[i++] = '.';
        for (k = 1u; k < (uint32_t)-x; k++) d[i++] = '0';
        for (k = 0u; k < last; k++) d[i++] = g[k];
        d[i] = 0;
    }
    return d;
}
/* TIME_TO_STRING, LTIME_TO_STRING: the literal, lower-case units, the
 * ones that are not 0 ('T#1d2h3m4s500ms', 'T#0ms', 'T#-1s500ms'); TIME in
 * ms, LTIME in ns ('LTIME#1s5us'). 40 bytes hold any. */
static inline const char *iecf_time_str(char *d, int64_t v, int ltime) {
    static const char *const unit[7] = {"d", "h", "m", "s", "ms", "us", "ns"};
    static const uint64_t ns[7] = {UINT64_C(86400000000000), UINT64_C(3600000000000), UINT64_C(60000000000),
                                   UINT64_C(1000000000),     UINT64_C(1000000),       UINT64_C(1000), UINT64_C(1)};
    uint64_t m = v < 0 ? (uint64_t)0u - (uint64_t)v : (uint64_t)v;
    uint32_t i = iecf_cpy(d, 0u, ltime ? "LTIME#" : "T#"), u, units = ltime ? 7u : 5u;
    char t[21];
    if (v < 0) d[i++] = '-';
    if (m == 0u) {
        iecf_cpy(d, i, ltime ? "0ns" : "0ms");
        return d;
    }
    for (u = 0u; u < units; u++) {
        uint64_t per = ltime ? ns[u] : ns[u] / UINT64_C(1000000), q = m / per;
        if (q != 0u) {
            i = iecf_cpy(d, i, iecf_u_str(t, q));
            i = iecf_cpy(d, i, unit[u]);
            m -= q * per;
        }
    }
    return d;
}

/* ---- conversions from STRING (docs/dependencies.md) ---------------------
 * The text is read as CODESYS and TwinCAT read it: spaces before it
 * skipped, then a literal of the type - a type prefix (`INT#`) allowed, a
 * sign, a based integer (`16#FF`, `2#1111_1111`), underscores between
 * digits, a REAL's fraction and exponent, a duration's `T#` - and what
 * follows the literal ignored ('123abc' is 123). No literal at all, or a
 * value its type cannot hold, gives 0 and is flagged
 * IECF_FAULT_CONVERSION (g: &ctx->flag), as Siemens' ENO=FALSE; the
 * task goes on (decided 2026-11-06). */
static inline int iecf_is_digit(char c, uint32_t base) {
    uint32_t v = c >= '0' && c <= '9' ? (uint32_t)(c - '0')
               : c >= 'a' && c <= 'f' ? (uint32_t)(c - 'a') + 10u
               : c >= 'A' && c <= 'F' ? (uint32_t)(c - 'A') + 10u : 99u;
    return v < base ? (int)v : -1;
}
static inline int iecf_is_letter(char c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'; }
/* Spaces, then a type prefix `NAME#` if there is one. */
static inline const char *iecf_lit_start(const char *s) {
    const char *t;
    while (*s == ' ' || *s == '\t') s++;
    for (t = s; iecf_is_letter(*t); t++) {
    }
    return (t != s && *t == '#') ? t + 1 : s;
}
/* An unsigned integer literal's magnitude; *neg its sign. 0 if no digit. */
static inline int iecf_lit_int(const char *s, int *neg, uint64_t *mag) {
    uint32_t base = 10u, any = 0u;
    int d;
    s = iecf_lit_start(s);
    *neg = *s == '-';
    if (*s == '-' || *s == '+') s++;
    if ((s[0] == '2' || s[0] == '8') && s[1] == '#') {
        base = (uint32_t)(s[0] - '0');
        s += 2;
    } else if (s[0] == '1' && s[1] == '6' && s[2] == '#') {
        base = 16u;
        s += 3;
    }
    *mag = 0u;
    for (; (d = iecf_is_digit(*s, base)) >= 0 || (*s == '_' && any); s++) {
        if (d < 0) continue;
        if (*mag > (UINT64_MAX - (uint64_t)d) / base) return -1; /* too big for any type */
        *mag = *mag * base + (uint64_t)d;
        any = 1u;
    }
    return any ? 1 : 0;
}
static inline uint64_t iecf_str_u(iecf_fault_t *g, const char *s, uint64_t hi) {
    int neg;
    uint64_t m;
    if (iecf_lit_int(s, &neg, &m) != 1 || m > hi || (neg && m != 0u)) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return m;
}
static inline int64_t iecf_str_s(iecf_fault_t *g, const char *s, int64_t lo, int64_t hi) {
    int neg;
    uint64_t m;
    if (iecf_lit_int(s, &neg, &m) != 1 || (neg ? m > (uint64_t)0u - (uint64_t)lo : m > (uint64_t)hi)) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    return neg ? (int64_t)((uint64_t)0u - m) : (int64_t)m;
}
static inline IEC_BOOL iecf_str_bool(iecf_fault_t *g, const char *s) {
    static const char *const word[2] = {"FALSE", "TRUE"};
    uint32_t w, i;
    s = iecf_lit_start(s);
    if (*s == '0' || *s == '1') return (IEC_BOOL)(*s == '1');
    for (w = 0u; w < 2u; w++) {
        for (i = 0u; word[w][i] != 0 && (s[i] == word[w][i] || s[i] == (char)(word[w][i] + ('a' - 'A'))); i++) {
        }
        if (word[w][i] == 0) return (IEC_BOOL)w;
    }
    iecf_flag(g, IECF_FAULT_CONVERSION);
    return 0u;
}

/* STRING_TO_REAL, STRING_TO_LREAL: the decimal text rounded once, half
 * to even, to `bits` (24 or 53) significant bits, exactly - a big
 * integer, no floating point until the end - so a REAL is not rounded
 * twice through an LREAL. The value is returned as a double, exact for
 * either type. Beyond the type's range, or below its least value but not
 * 0: 0, flagged. */
#define IECF_REAL_LIMBS 100u
#define IECF_REAL_DIGITS 800
static inline void iecf_big_mul(uint32_t *l, uint32_t *n, uint32_t m, uint32_t add) {
    uint64_t c = add;
    uint32_t i;
    for (i = 0u; i < *n; i++) {
        uint64_t x = (uint64_t)l[i] * m + c;
        l[i] = (uint32_t)x;
        c = x >> 32;
    }
    if (c != 0u && *n < IECF_REAL_LIMBS) l[(*n)++] = (uint32_t)c;
}
static inline uint32_t iecf_big_div(uint32_t *l, uint32_t *n, uint32_t m) {
    uint64_t r = 0u;
    uint32_t i;
    for (i = *n; i > 0u; i--) {
        uint64_t x = (r << 32) | l[i - 1u];
        l[i - 1u] = (uint32_t)(x / m);
        r = x % m;
    }
    while (*n > 0u && l[*n - 1u] == 0u) (*n)--;
    return (uint32_t)r;
}
static inline uint32_t iecf_big_bits(const uint32_t *l, uint32_t n) {
    uint32_t b = 32u * (n - 1u), t = l[n - 1u];
    for (; t != 0u; t >>= 1) b++;
    return b;
}
static inline uint32_t iecf_big_bit(const uint32_t *l, uint32_t i) { return (l[i / 32u] >> (i % 32u)) & 1u; }
static inline double iecf_str_real(iecf_fault_t *g, const char *s, uint32_t bits) {
    uint32_t l[IECF_REAL_LIMBS], n = 1u, len, keep, i;
    int32_t ndig = 0, point = -1, e10 = 0, x, e2, emin = bits == 24u ? -126 : -1022, emax = bits == 24u ? 127 : 1023;
    int neg, any = 0, dropped = 0, sticky = 0, up;
    uint64_t q = 0u;
    double v;
    s = iecf_lit_start(s);
    neg = *s == '-';
    if (*s == '-' || *s == '+') s++;
    l[0] = 0u;
    /* digits: leading zeros skipped, the first 800 kept, the rest as sticky */
    for (; (*s >= '0' && *s <= '9') || (*s == '_' && any) || (*s == '.' && point < 0); s++) {
        if (*s == '_') continue;
        if (*s == '.') {
            point = ndig;
            continue;
        }
        any = 1;
        if (*s == '0' && ndig == 0) {
            if (point >= 0) e10--;
            continue;
        }
        if (ndig < IECF_REAL_DIGITS) {
            iecf_big_mul(l, &n, 10u, (uint32_t)(*s - '0'));
            ndig++;
        } else {
            if (*s != '0') dropped = 1;
            if (point < 0) e10++;
        }
    }
    if (!any) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0.0;
    }
    if (point >= 0) e10 -= ndig - point;
    if ((*s == 'e' || *s == 'E') && ((s[1] >= '0' && s[1] <= '9') || ((s[1] == '+' || s[1] == '-') && s[2] >= '0' && s[2] <= '9'))) {
        int eneg = s[1] == '-';
        int32_t ex = 0;
        for (s += (s[1] == '+' || s[1] == '-') ? 2 : 1; *s >= '0' && *s <= '9'; s++) {
            if (ex < 100000) ex = ex * 10 + (*s - '0');
        }
        e10 += eneg ? -ex : ex;
    }
    if (ndig == 0) return neg ? -0.0 : 0.0;
    if (dropped) { /* one more digit, not 0, keeps the rounding exact */
        iecf_big_mul(l, &n, 10u, 1u);
        ndig++;
        e10--;
    }
    x = ndig + e10; /* the value is 0.ddd x 10^x */
    if (x > 310 || x < -330) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0.0;
    }
    e2 = 0;
    if (e10 >= 0) {
        for (; e10 > 0; e10--) iecf_big_mul(l, &n, 10u, 0u);
    } else {
        /* D / 10^m as (D x 2^sh / 5^m) x 2^-(sh + m), at least 66 bits */
        int32_t m = -e10, need = 66 + (m * 2322 + 999) / 1000 - (int32_t)iecf_big_bits(l, n), j;
        for (j = need; j > 0; j -= 16) iecf_big_mul(l, &n, (uint32_t)1u << (j > 16 ? 16 : j), 0u);
        e2 = -(need > 0 ? need : 0) - m;
        for (j = m; j > 0; j -= 13) {
            uint32_t d = 1u, k;
            for (k = 0u; k < (uint32_t)(j > 13 ? 13 : j); k++) d *= 5u;
            if (iecf_big_div(l, &n, d) != 0u) sticky = 1;
        }
    }
    len = iecf_big_bits(l, n);
    x = (int32_t)len - 1 + e2; /* 2^x <= value < 2^(x+1) */
    keep = bits;
    if (x < emin) {
        if (emin - x > (int32_t)bits) { /* below half the least value */
            iecf_flag(g, IECF_FAULT_CONVERSION);
            return 0.0;
        }
        keep = bits - (uint32_t)(emin - x);
    }
    /* the top `keep` bits, the next one, and whether any below is set */
    for (i = 0u; i < keep; i++) q = (q << 1) | (i < len ? iecf_big_bit(l, len - 1u - i) : 0u);
    up = 0;
    if (len > keep) {
        uint32_t r = iecf_big_bit(l, len - 1u - keep);
        for (i = 0u; i + 1u < len - keep && !sticky; i++) sticky = (int)iecf_big_bit(l, i);
        up = r && (sticky || (q & 1u));
    }
    q += (uint64_t)up;
    e2 = (int32_t)len - (int32_t)keep + e2; /* value = q x 2^e2 */
    if (q == 0u || x + (q >> keep != 0u ? 1 : 0) > emax) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0.0;
    }
    v = (double)q; /* exact: q < 2^54 */
    for (; e2 >= 60; e2 -= 60) v *= 0x1p60;
    for (; e2 <= -60; e2 += 60) v /= 0x1p60;
    if (e2 > 0) v *= (double)(UINT64_C(1) << e2);
    if (e2 < 0) v /= (double)(UINT64_C(1) << -e2);
    return neg ? -v : v;
}

/* STRING_TO_TIME, STRING_TO_LTIME: a duration literal, `T#`, `TIME#`,
 * `LT#` or `LTIME#` first (any case), units d h m s ms us ns, a fraction
 * on any; TIME keeps whole milliseconds (the rest dropped, toward 0). */
static inline int64_t iecf_str_time(iecf_fault_t *g, const char *s, int ltime) {
    static const char *const unit[7] = {"ms", "us", "ns", "d", "h", "m", "s"};
    static const uint64_t ns[7] = {UINT64_C(1000000), UINT64_C(1000), UINT64_C(1), UINT64_C(86400000000000),
                                   UINT64_C(3600000000000), UINT64_C(60000000000), UINT64_C(1000000000)};
    uint64_t total = 0u, lim = ltime ? (uint64_t)INT64_MAX : (uint64_t)INT32_MAX * UINT64_C(1000000) + UINT64_C(999999);
    int neg = 0, any = 0, bad = 0;
    const char *t;
    while (*s == ' ' || *s == '\t') s++;
    for (t = s; iecf_is_letter(*t); t++) {
    }
    if (*t != '#' || t == s) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    s = t + 1;
    if (*s == '-' || *s == '+') neg = *s++ == '-';
    if (neg) lim += ltime ? 1u : UINT64_C(1000000); /* the most negative */
    for (;;) {
        uint64_t whole = 0u, frac = 0u, scale = 1u;
        uint32_t u, k;
        int digits = 0;
        for (; (*s >= '0' && *s <= '9') || (*s == '_' && digits); s++) {
            if (*s == '_') continue;
            if (whole > (UINT64_MAX - 9u) / 10u) bad = 1;
            else whole = whole * 10u + (uint64_t)(*s - '0');
            digits = 1;
        }
        if (!digits) break;
        if (*s == '.') {
            for (s++; *s >= '0' && *s <= '9'; s++) {
                if (scale < UINT64_C(1000000000000000000)) {
                    frac = frac * 10u + (uint64_t)(*s - '0');
                    scale *= 10u;
                }
            }
        }
        for (u = 0u; u < 7u; u++) {
            for (k = 0u; unit[u][k] != 0 && (s[k] == unit[u][k] || s[k] == (char)(unit[u][k] - ('a' - 'A'))); k++) {
            }
            if (unit[u][k] == 0) break;
        }
        if (u == 7u) break;
        s += u < 3u ? 2 : 1;
        if (whole > lim / ns[u]) bad = 1;
        else total += whole * ns[u];
        /* the fraction: frac / scale of the unit, in ns, truncated */
        {
            uint64_t per = ns[u], part = 0u, f = frac, sc = scale;
            while (sc > 1u && per % 10u == 0u) {
                per /= 10u;
                sc /= 10u;
            }
            part = f / sc * per + (f % sc) * per / sc;
            total += part;
        }
        if (total > lim) bad = 1;
        any = 1;
    }
    if (!any || bad) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    if (!ltime) total /= UINT64_C(1000000);
    return neg ? (int64_t)((uint64_t)0u - total) : (int64_t)total;
}

/* ---- DATE, TOD and DT (docs/dependencies.md) -----------------------------
 * DATE and DT are seconds since 1970-01-01 00:00 (a DATE at midnight), a
 * TOD milliseconds since midnight, each a uint32_t. A TOD wraps at
 * midnight, unflagged: TOD#23:00:00 + T#2h is TOD#01:00:00. A DATE or DT
 * outside 1970-01-01 .. 2106-02-07-06:28:15, or a difference outside
 * TIME, wraps and is flagged IECF_FAULT_OVERFLOW (g: &ctx->flag), as an
 * integer's overflow is. A DT holds whole seconds: milliseconds coming to
 * it are dropped, rounding down, unflagged (decided 2026-11-07, florin). */
#define IECF_DAY_S 86400
#define IECF_DAY_MS 86400000

/* Any number of ms as a time of day: wrapped at midnight. */
static inline IEC_TOD iecf_tod_ms(int64_t ms) {
    int64_t r = ms % IECF_DAY_MS;
    return (IEC_TOD)(r < 0 ? r + IECF_DAY_MS : r);
}
/* a / b rounded down, b > 0. */
static inline int64_t iecf_floor_div(int64_t a, int64_t b) {
    int64_t q = a / b;
    return (a % b != 0 && a < 0) ? q - 1 : q;
}
/* Seconds as a DATE or DT: outside 0 .. 2^32 - 1 wrapped and flagged. */
static inline uint32_t iecf_dt_s(iecf_fault_t *g, int64_t s) {
    if (s < 0 || s > (int64_t)UINT32_MAX) iecf_flag(g, IECF_FAULT_OVERFLOW);
    return (uint32_t)(uint64_t)s;
}
/* Milliseconds as a TIME: outside it wrapped and flagged. */
static inline IEC_TIME iecf_ms_time(iecf_fault_t *g, int64_t ms) {
    if (ms < INT32_MIN || ms > INT32_MAX) iecf_flag(g, IECF_FAULT_OVERFLOW);
    return iecf_to_i32((uint32_t)(uint64_t)ms);
}
/* Ed.3 Table 30: ADD_TOD_TIME, SUB_TOD_TIME, SUB_TOD_TOD, ADD_DT_TIME,
 * SUB_DT_TIME, SUB_DT_DT, SUB_DATE_DATE (as SUB_DT_DT: a DATE is a DT at
 * midnight), CONCAT_DATE_TOD, DT_TO_TOD, DT_TO_DATE, and DAY_OF_WEEK
 * (Table 36: 0 Sunday .. 6 Saturday). */
static inline IEC_TOD iecf_add_TOD(IEC_TOD x, IEC_TIME t) { return iecf_tod_ms((int64_t)x + t); }
static inline IEC_TOD iecf_sub_TOD(IEC_TOD x, IEC_TIME t) { return iecf_tod_ms((int64_t)x - t); }
static inline IEC_TIME iecf_sub_TOD_TOD(iecf_fault_t *g, IEC_TOD a, IEC_TOD b) {
    return iecf_ms_time(g, (int64_t)a - (int64_t)b);
}
static inline IEC_DT iecf_add_DT(iecf_fault_t *g, IEC_DT x, IEC_TIME t) {
    return iecf_dt_s(g, (int64_t)x + iecf_floor_div(t, 1000));
}
static inline IEC_DT iecf_sub_DT(iecf_fault_t *g, IEC_DT x, IEC_TIME t) {
    return iecf_dt_s(g, (int64_t)x + iecf_floor_div(-(int64_t)t, 1000));
}
static inline IEC_TIME iecf_sub_DT_DT(iecf_fault_t *g, IEC_DT a, IEC_DT b) {
    return iecf_ms_time(g, ((int64_t)a - (int64_t)b) * 1000);
}
static inline IEC_DT iecf_concat_date_tod(iecf_fault_t *g, IEC_DATE d, IEC_TOD t) {
    return iecf_dt_s(g, (int64_t)d + (int64_t)(t / 1000u));
}
static inline IEC_TOD iecf_dt_tod(IEC_DT x) { return (IEC_TOD)(x % (uint32_t)IECF_DAY_S * 1000u); }
static inline IEC_DATE iecf_dt_date(IEC_DT x) { return x - x % (uint32_t)IECF_DAY_S; }
static inline IEC_USINT iecf_day_of_week(IEC_DATE d) { return (IEC_USINT)((d / (uint32_t)IECF_DAY_S + 4u) % 7u); }

/* The proleptic Gregorian calendar (H. Hinnant's days_from_civil and
 * civil_from_days), integers only. */
static inline int64_t iecf_days(int64_t y, uint32_t m, uint32_t d) {
    int64_t era, yoe, doy, doe;
    y -= m <= 2u;
    era = (y >= 0 ? y : y - 399) / 400;
    yoe = y - era * 400;
    doy = (153 * (int64_t)(m > 2u ? m - 3u : m + 9u) + 2) / 5 + (int64_t)d - 1;
    doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return era * 146097 + doe - 719468;
}
static inline void iecf_civil(int64_t days, uint32_t *y, uint32_t *m, uint32_t *d) {
    /* days from 1677 on (LDATE's first, -106751): z stays positive. */
    uint32_t z = (uint32_t)(days + 719468), era = z / 146097u, doe = z - era * 146097u;
    uint32_t yoe = (doe - doe / 1460u + doe / 36524u - doe / 146096u) / 365u;
    uint32_t doy = doe - (365u * yoe + yoe / 4u - yoe / 100u), mp = (5u * doy + 2u) / 153u;
    *d = doy - (153u * mp + 2u) / 5u + 1u;
    *m = mp < 10u ? mp + 3u : mp - 9u;
    *y = yoe + era * 400u + (*m <= 2u ? 1u : 0u);
}

/* The days in month m (1..12) of year y. */
static inline uint32_t iecf_month_len(uint32_t y, uint32_t m) {
    if (m == 2u) return ((y % 4u == 0u && y % 100u != 0u) || y % 400u == 0u) ? 29u : 28u;
    return (m == 4u || m == 6u || m == 9u || m == 11u) ? 30u : 31u;
}

/* Ed.3 Table 36: CONCAT_DATE, CONCAT_TOD, CONCAT_DT from integer parts,
 * SPLIT_DATE, SPLIT_TOD, SPLIT_DT into them. A part out of its range - a
 * month not 1..12, a day not in its month, an hour not 0..23, a minute or
 * second not 0..59, a millisecond not 0..999 - or a date or DT outside
 * 1970-01-01 .. 2106-02-07-06:28:15 gives the type's default value
 * (D#1970-01-01, TOD#00:00:00, DT#1970-01-01-00:00:00), flagged
 * IECF_FAULT_CONVERSION, as CODESYS's DateConcat and PLCnext's
 * CONCAT_LDATE give it (decided 2026-11-08, florin). A DT's millisecond
 * is checked, then dropped, as a DT holds whole seconds. A ULINT part
 * above INT64_MAX comes as -1 (iecf_part), out of every range. */
static inline int64_t iecf_part(uint64_t u) { return u > (uint64_t)INT64_MAX ? -1 : (int64_t)u; }
static inline int iecf_ymd_ok(int64_t y, int64_t m, int64_t d) {
    return y >= 1970 && y <= 2106 && m >= 1 && m <= 12 && d >= 1 && d <= (int64_t)iecf_month_len((uint32_t)y, (uint32_t)m);
}
static inline int iecf_hms_ok(int64_t h, int64_t mi, int64_t s, int64_t ms) {
    return h >= 0 && h <= 23 && mi >= 0 && mi <= 59 && s >= 0 && s <= 59 && ms >= 0 && ms <= 999;
}
static inline IEC_DT iecf_concat_DT(iecf_fault_t *g, int64_t y, int64_t m, int64_t d, int64_t h, int64_t mi,
                                    int64_t s, int64_t ms) {
    int64_t t;
    if (!iecf_ymd_ok(y, m, d) || !iecf_hms_ok(h, mi, s, ms)) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    t = iecf_days(y, (uint32_t)m, (uint32_t)d) * IECF_DAY_S + h * 3600 + mi * 60 + s;
    if (t > (int64_t)UINT32_MAX) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return (IEC_DT)t;
}
static inline IEC_DATE iecf_concat_DATE(iecf_fault_t *g, int64_t y, int64_t m, int64_t d) {
    return iecf_concat_DT(g, y, m, d, 0, 0, 0, 0);
}
static inline IEC_TOD iecf_concat_TOD(iecf_fault_t *g, int64_t h, int64_t mi, int64_t s, int64_t ms) {
    if (!iecf_hms_ok(h, mi, s, ms)) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return (IEC_TOD)(((h * 60 + mi) * 60 + s) * 1000 + ms);
}
static inline void iecf_split_DATE(IEC_DATE x, uint32_t *y, uint32_t *m, uint32_t *d) {
    iecf_civil(x / (uint32_t)IECF_DAY_S, y, m, d);
}
static inline void iecf_split_TOD(IEC_TOD x, uint32_t *h, uint32_t *mi, uint32_t *s, uint32_t *ms) {
    *ms = x % 1000u;
    *s = x / 1000u % 60u;
    *mi = x / 60000u % 60u;
    *h = x / 3600000u;
}
static inline void iecf_split_DT(IEC_DT x, uint32_t *y, uint32_t *m, uint32_t *d, uint32_t *h, uint32_t *mi,
                                 uint32_t *s, uint32_t *ms) {
    iecf_split_DATE(x, y, m, d);
    iecf_split_TOD(x % (uint32_t)IECF_DAY_S * 1000u, h, mi, s, ms);
}
/* `v` in at least `w` digits at d[i]; the next index. */
static inline uint32_t iecf_pad(char *d, uint32_t i, uint32_t v, uint32_t w) {
    char t[11];
    uint32_t n = 0u;
    do {
        t[n++] = (char)('0' + v % 10u);
        v /= 10u;
    } while (v != 0u);
    for (; n < w; w--) d[i++] = '0';
    while (n != 0u) d[i++] = t[--n];
    d[i] = 0;
    return i;
}
static inline uint32_t iecf_day_part(char *d, uint32_t i, int64_t days) {
    uint32_t y, m, day;
    iecf_civil(days, &y, &m, &day);
    i = iecf_pad(d, i, y, 4u);
    d[i++] = '-';
    i = iecf_pad(d, i, m, 2u);
    d[i++] = '-';
    return iecf_pad(d, i, day, 2u);
}
static inline uint32_t iecf_date_part(char *d, uint32_t i, uint32_t s) {
    return iecf_day_part(d, i, (int64_t)(s / (uint32_t)IECF_DAY_S));
}
static inline uint32_t iecf_tod_part(char *d, uint32_t i, uint32_t ms) {
    i = iecf_pad(d, i, ms / 3600000u, 2u);
    d[i++] = ':';
    i = iecf_pad(d, i, ms / 60000u % 60u, 2u);
    d[i++] = ':';
    i = iecf_pad(d, i, ms / 1000u % 60u, 2u);
    if (ms % 1000u != 0u) {
        d[i++] = '.';
        i = iecf_pad(d, i, ms % 1000u, 3u);
    }
    return i;
}
/* DATE_TO_STRING 'D#2026-10-03' (13 bytes), TOD_TO_STRING 'TOD#12:34:56'
 * with '.123' when the ms are not 0 (19 bytes hold any uint32_t),
 * DT_TO_STRING 'DT#2026-10-03-12:34:56' (23 bytes). */
static inline const char *iecf_date_str(char *d, IEC_DATE v) {
    iecf_date_part(d, iecf_cpy(d, 0u, "D#"), v);
    return d;
}
static inline const char *iecf_tod_str(char *d, IEC_TOD v) {
    iecf_tod_part(d, iecf_cpy(d, 0u, "TOD#"), v);
    return d;
}
static inline const char *iecf_dt_str(char *d, IEC_DT v) {
    uint32_t i = iecf_date_part(d, iecf_cpy(d, 0u, "DT#"), v);
    d[i++] = '-';
    iecf_tod_part(d, i, v % (uint32_t)IECF_DAY_S * 1000u);
    return d;
}

/* STRING_TO_DATE, _TOD, _DT: a literal of the type, its prefix first (any
 * case; `D#`, `DATE#`, `TOD#`, `DT#` ...), as STRING_TO_TIME reads one; a
 * TOD keeps whole ms, a DT whole seconds (the rest dropped). No literal,
 * no such day or time, or one outside the type: 0, flagged. */
static inline const char *iecf_num(const char *s, uint32_t *v, uint32_t most) {
    uint32_t n = 0u;
    *v = 0u;
    for (; *s >= '0' && *s <= '9' && n < most; s++, n++) *v = *v * 10u + (uint32_t)(*s - '0');
    return n == 0u ? NULL : s;
}
static inline const char *iecf_read_date(const char *s, int64_t *days) {
    uint32_t y, m, d;
    if ((s = iecf_num(s, &y, 9u)) == NULL || *s++ != '-') return NULL;
    if ((s = iecf_num(s, &m, 2u)) == NULL || *s++ != '-') return NULL;
    if ((s = iecf_num(s, &d, 2u)) == NULL || m < 1u || m > 12u || d < 1u) return NULL;
    if (d > iecf_month_len(y, m)) return NULL;
    *days = iecf_days((int64_t)y, m, d);
    return s;
}
/* A time of day in nanoseconds since midnight, digits past the ninth
 * dropped. */
static inline const char *iecf_read_tod_ns(const char *s, int64_t *ns) {
    uint32_t h, m, sec, k;
    int64_t f = 0;
    if ((s = iecf_num(s, &h, 2u)) == NULL || *s++ != ':') return NULL;
    if ((s = iecf_num(s, &m, 2u)) == NULL || *s++ != ':') return NULL;
    if ((s = iecf_num(s, &sec, 2u)) == NULL || h > 23u || m > 59u || sec > 59u) return NULL;
    k = 0u;
    if (*s == '.') {
        for (s++; *s >= '0' && *s <= '9'; s++, k++) {
            if (k < 9u) f = f * 10 + (int64_t)(*s - '0');
        }
    }
    for (; k < 9u; k++) f *= 10;
    *ns = (int64_t)((h * 60u + m) * 60u + sec) * INT64_C(1000000000) + f;
    return s;
}
static inline const char *iecf_read_tod(const char *s, uint32_t *ms) {
    int64_t ns = 0;
    if ((s = iecf_read_tod_ns(s, &ns)) == NULL) return NULL;
    *ms = (uint32_t)(ns / INT64_C(1000000));
    return s;
}
static inline const char *iecf_lit_body(const char *s) {
    const char *t;
    while (*s == ' ' || *s == '\t') s++;
    for (t = s; iecf_is_letter(*t); t++) {
    }
    return (t != s && *t == '#') ? t + 1 : NULL;
}
static inline IEC_DATE iecf_str_date(iecf_fault_t *g, const char *s) {
    int64_t days = 0;
    if ((s = iecf_lit_body(s)) == NULL || iecf_read_date(s, &days) == NULL || days < 0 ||
        days * IECF_DAY_S > (int64_t)UINT32_MAX) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return (IEC_DATE)(days * IECF_DAY_S);
}
static inline IEC_TOD iecf_str_tod(iecf_fault_t *g, const char *s) {
    uint32_t ms = 0u;
    if ((s = iecf_lit_body(s)) == NULL || iecf_read_tod(s, &ms) == NULL) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return ms;
}
static inline IEC_DT iecf_str_dt(iecf_fault_t *g, const char *s) {
    int64_t days = 0, v;
    uint32_t ms = 0u;
    if ((s = iecf_lit_body(s)) == NULL || (s = iecf_read_date(s, &days)) == NULL || *s++ != '-' ||
        iecf_read_tod(s, &ms) == NULL || days < 0 || (v = days * IECF_DAY_S + (int64_t)(ms / 1000u)) > (int64_t)UINT32_MAX) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return (IEC_DT)v;
}

/* ---- LDATE, LTOD, LDT (Ed.3 Table 10, notes n-p) ---------------------------
 * Each a signed 64-bit count of nanoseconds: LDATE and LDT since
 * 1970-01-01, 1677-09-21-00:12:43.145224192 to 2262-04-11-23:47:16.854775807
 * (an LDATE at midnight, LD#1677-09-22 to LD#2262-04-11), LTOD since
 * midnight. Decided 2026-11-08 (florin; docs/dependencies.md):
 * - an LTOD wraps at midnight, unflagged, as a TOD;
 * - an LDT result outside the range, or a difference outside LTIME, wraps
 *   and is flagged IECF_FAULT_OVERFLOW (iecf_add_LINT, iecf_sub_LINT), as
 *   a DT's;
 * - a long value made a short one drops what the short type is too coarse
 *   for, rounding down, unflagged (LDT_TO_DT the sub-seconds, LTOD_TO_TOD
 *   and LTIME_TO_TIME the sub-milliseconds); one outside the short type
 *   gives its default value (D#1970-01-01, DT#1970-01-01-00:00:00, T#0s),
 *   flagged IECF_FAULT_CONVERSION. */
#define IECF_NS_MS INT64_C(1000000)
#define IECF_NS_S INT64_C(1000000000)
#define IECF_DAY_NS (INT64_C(86400) * IECF_NS_S)
static inline int64_t iecf_floor_mod(int64_t a, int64_t b) {
    int64_t r = a % b;
    return r < 0 ? r + b : r;
}
static inline IEC_LTOD iecf_ltod_ns(int64_t ns) { return iecf_floor_mod(ns, IECF_DAY_NS); }
static inline IEC_LTOD iecf_add_LTOD(IEC_LTOD x, IEC_LTIME t) {
    return iecf_ltod_ns(iecf_ltod_ns(x) + iecf_ltod_ns(t));
}
static inline IEC_LTOD iecf_sub_LTOD(IEC_LTOD x, IEC_LTIME t) {
    return iecf_ltod_ns(iecf_ltod_ns(x) - iecf_ltod_ns(t));
}
/* Seconds and the nanoseconds past them (0..10^9-1) as an LDT, if it
 * holds them. */
static inline int iecf_ldt_of(int64_t s, int64_t sub, int64_t *out) {
    if (s > INT64_C(9223372036) || (s == INT64_C(9223372036) && sub > INT64_C(854775807))) return 0;
    if (s < INT64_C(-9223372037) || (s == INT64_C(-9223372037) && sub < INT64_C(145224192))) return 0;
    *out = s < 0 ? (s + 1) * IECF_NS_S + (sub - IECF_NS_S) : s * IECF_NS_S + sub;
    return 1;
}
/* Conversions (Ed.3 Table 22, Figure 11). */
static inline IEC_LTIME iecf_time_ltime(IEC_TIME t) { return (int64_t)t * IECF_NS_MS; }
static inline IEC_TIME iecf_ltime_time(iecf_fault_t *g, IEC_LTIME t) {
    int64_t ms = iecf_floor_div(t, IECF_NS_MS);
    if (ms < INT32_MIN || ms > INT32_MAX) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    return (IEC_TIME)ms;
}
static inline IEC_LDT iecf_dt_ldt(IEC_DT x) { return (int64_t)x * IECF_NS_S; }
static inline IEC_LTOD iecf_tod_ltod(IEC_TOD x) { return (int64_t)x * IECF_NS_MS; }
static inline IEC_LDATE iecf_dt_ldate(IEC_DT x) { return (int64_t)iecf_dt_date(x) * IECF_NS_S; }
static inline IEC_LTOD iecf_dt_ltod(IEC_DT x) { return (int64_t)(x % (uint32_t)IECF_DAY_S) * IECF_NS_S; }
static inline IEC_DT iecf_ldt_dt(iecf_fault_t *g, IEC_LDT x) {
    int64_t s = iecf_floor_div(x, IECF_NS_S);
    if (s < 0 || s > (int64_t)UINT32_MAX) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return (IEC_DT)s;
}
static inline IEC_DATE iecf_ldt_date(iecf_fault_t *g, IEC_LDT x) {
    int64_t s = iecf_floor_div(x, IECF_DAY_NS) * IECF_DAY_S;
    if (s < 0 || s > (int64_t)UINT32_MAX) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0u;
    }
    return (IEC_DATE)s;
}
/* An LDT on 1677-09-21 has no midnight an LDATE holds. */
static inline IEC_LDATE iecf_ldt_ldate(iecf_fault_t *g, IEC_LDT x) {
    int64_t d = iecf_floor_div(x, IECF_DAY_NS);
    if (d < -(INT64_MAX / IECF_DAY_NS)) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    return d * IECF_DAY_NS;
}
static inline IEC_LTOD iecf_ldt_ltod(IEC_LDT x) { return iecf_ltod_ns(x); }
static inline IEC_TOD iecf_ldt_tod(IEC_LDT x) { return (IEC_TOD)(iecf_ltod_ns(x) / IECF_NS_MS); }
/* CONCAT_DATE_LTOD, CONCAT_LTOD, CONCAT_LDT, SPLIT_LTOD, SPLIT_LDT (Ed.3
 * Table 36), as their short forms: a part out of range, or an LDT
 * outside the range, gives the default value, flagged
 * IECF_FAULT_CONVERSION. The parts count milliseconds: a SPLIT drops
 * what is finer, rounding down. */
static inline IEC_LDT iecf_concat_date_ltod(IEC_DATE d, IEC_LTOD t) {
    return (int64_t)d * IECF_NS_S + iecf_ltod_ns(t);
}
static inline IEC_LTOD iecf_concat_LTOD(iecf_fault_t *g, int64_t h, int64_t mi, int64_t s, int64_t ms) {
    return (int64_t)iecf_concat_TOD(g, h, mi, s, ms) * IECF_NS_MS;
}
static inline IEC_LDT iecf_concat_LDT(iecf_fault_t *g, int64_t y, int64_t m, int64_t d, int64_t h, int64_t mi,
                                      int64_t s, int64_t ms) {
    int64_t v = 0;
    if (y < 1677 || y > 2262 || m < 1 || m > 12 || d < 1 || d > (int64_t)iecf_month_len((uint32_t)y, (uint32_t)m) ||
        !iecf_hms_ok(h, mi, s, ms) ||
        !iecf_ldt_of(iecf_days(y, (uint32_t)m, (uint32_t)d) * IECF_DAY_S + h * 3600 + mi * 60 + s, ms * IECF_NS_MS,
                     &v)) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    return v;
}
static inline void iecf_split_LTOD(IEC_LTOD x, uint32_t *h, uint32_t *mi, uint32_t *s, uint32_t *ms) {
    iecf_split_TOD(iecf_ldt_tod(x), h, mi, s, ms);
}
static inline void iecf_split_LDT(IEC_LDT x, uint32_t *y, uint32_t *m, uint32_t *d, uint32_t *h, uint32_t *mi,
                                  uint32_t *s, uint32_t *ms) {
    iecf_civil(iecf_floor_div(x, IECF_DAY_NS), y, m, d);
    iecf_split_LTOD(x, h, mi, s, ms);
}
/* LDATE_TO_STRING 'LD#2026-10-03' (14 bytes), LTOD_TO_STRING
 * 'LTOD#12:34:56' with the fraction in groups of three digits, as many as
 * are not 0 ('.5' as '.500', '.000001' ...; 24 bytes), LDT_TO_STRING
 * 'LDT#2026-10-03-12:34:56.000000001' (34 bytes). */
static inline uint32_t iecf_ltod_part(char *d, uint32_t i, int64_t ns) {
    int64_t f = ns % IECF_NS_S;
    uint32_t w = 9u;
    i = iecf_tod_part(d, i, (uint32_t)(ns / IECF_NS_S * 1000));
    if (f != 0) {
        while (f % 1000 == 0) {
            f /= 1000;
            w -= 3u;
        }
        d[i++] = '.';
        i = iecf_pad(d, i, (uint32_t)f, w);
    }
    return i;
}
static inline const char *iecf_ldate_str(char *d, IEC_LDATE v) {
    iecf_day_part(d, iecf_cpy(d, 0u, "LD#"), iecf_floor_div(v, IECF_DAY_NS));
    return d;
}
static inline const char *iecf_ltod_str(char *d, IEC_LTOD v) {
    iecf_ltod_part(d, iecf_cpy(d, 0u, "LTOD#"), iecf_ltod_ns(v));
    return d;
}
static inline const char *iecf_ldt_str(char *d, IEC_LDT v) {
    uint32_t i = iecf_day_part(d, iecf_cpy(d, 0u, "LDT#"), iecf_floor_div(v, IECF_DAY_NS));
    d[i++] = '-';
    iecf_ltod_part(d, i, iecf_ltod_ns(v));
    return d;
}
/* STRING_TO_LDATE, _LTOD, _LDT: as STRING_TO_DATE, to the nanosecond. */
static inline IEC_LDT iecf_str_ldt(iecf_fault_t *g, const char *s) {
    int64_t days = 0, ns = 0, v = 0;
    if ((s = iecf_lit_body(s)) == NULL || (s = iecf_read_date(s, &days)) == NULL || *s++ != '-' ||
        iecf_read_tod_ns(s, &ns) == NULL || days < -106752 || days > 106751 ||
        !iecf_ldt_of(days * IECF_DAY_S + ns / IECF_NS_S, ns % IECF_NS_S, &v)) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    return v;
}
static inline IEC_LDATE iecf_str_ldate(iecf_fault_t *g, const char *s) {
    int64_t days = 0;
    if ((s = iecf_lit_body(s)) == NULL || iecf_read_date(s, &days) == NULL || days < -106751 || days > 106751) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    return days * IECF_DAY_NS;
}
static inline IEC_LTOD iecf_str_ltod(iecf_fault_t *g, const char *s) {
    int64_t ns = 0;
    if ((s = iecf_lit_body(s)) == NULL || iecf_read_tod_ns(s, &ns) == NULL) {
        iecf_flag(g, IECF_FAULT_CONVERSION);
        return 0;
    }
    return ns;
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

/* ---- SFC (Ed.3 6.7) -------------------------------------------------------
 * A step: X its flag, T its elapsed time (Table 54), kept when the step
 * is left and set to T#0s when it is entered. T counts from the scan the
 * step is entered at, whole milliseconds, and stops at TIME's largest.
 * An initial step's time starts at the first scan. */
#define IECF_STEP_FRESH INT64_MIN
typedef struct {
    IEC_BOOL X;
    IEC_TIME T;
    int64_t since_;
} iecf_step_t;
static inline void iecf_step_init(iecf_step_t *s, IEC_BOOL initial) {
    s->X = initial;
    s->T = 0;
    s->since_ = IECF_STEP_FRESH;
}
static inline void iecf_step_time(iecf_step_t *s, int64_t now) {
    uint64_t ms;
    if (!s->X) return;
    if (s->since_ == IECF_STEP_FRESH) s->since_ = now;
    ms = now > s->since_ ? ((uint64_t)now - (uint64_t)s->since_) / UINT64_C(1000000) : 0u;
    s->T = ms > (uint64_t)INT32_MAX ? INT32_MAX : (IEC_TIME)ms;
}
static inline void iecf_step_leave(iecf_step_t *s) { s->X = 0; }
/* Entering a step that is still active is an "unsafe" SFC (Ed.3 6.7.5,
 * Figure 26 a): it stays active, once, and is flagged (decided
 * 2026-11-07, docs/dependencies.md). */
static inline void iecf_step_enter(iecf_fault_t *g, iecf_step_t *s, int64_t now) {
    if (s->X) {
        iecf_flag(g, IECF_FAULT_SFC);
        return;
    }
    s->X = 1;
    s->T = 0;
    s->since_ = now;
}

/* An action's control: the functional equivalent of ACTION_CONTROL with
 * "final scan" logic (Ed.3 Figure 22 a, Figure 23 a; Table 60 feature 1).
 * `q` holds the qualifiers of its active associations, `t` the duration
 * of its active time-related one. Q is the action flag; the action runs
 * while A is TRUE: while Q is, one more scan after Q falls, and once for
 * a P1 (rising) or P0 (falling) association, with Q FALSE then.
 * The final scan and P0 follow a falling edge that happened: the
 * figure's F_TRIGs, whose M starts FALSE (Table 44), would also fire on
 * the first scan, which no vendor does and NOTE 3 does not mean. */
#define IECF_Q_N 0x001u
#define IECF_Q_R 0x002u
#define IECF_Q_S 0x004u
#define IECF_Q_L 0x008u
#define IECF_Q_D 0x010u
#define IECF_Q_P 0x020u
#define IECF_Q_P1 0x040u
#define IECF_Q_P0 0x080u
#define IECF_Q_SD 0x100u
#define IECF_Q_DS 0x200u
#define IECF_Q_SL 0x400u
typedef struct {
    iecf_RS_t s_ff, sd_ff, ds_ff, sl_ff;
    iecf_TON_t l_tmr, d_tmr, sd_tmr, ds_tmr, sl_tmr;
    iecf_R_TRIG_t p_trig, p1_trig;
    IEC_BOOL q_, p0_; /* Q and P0 of the scan before */
    IEC_BOOL Q, A;
} iecf_action_t;
static inline void iecf_action_init(iecf_action_t *a) {
    iecf_RS_init(&a->s_ff);
    iecf_RS_init(&a->sd_ff);
    iecf_RS_init(&a->ds_ff);
    iecf_RS_init(&a->sl_ff);
    iecf_TON_init(&a->l_tmr);
    iecf_TON_init(&a->d_tmr);
    iecf_TON_init(&a->sd_tmr);
    iecf_TON_init(&a->ds_tmr);
    iecf_TON_init(&a->sl_tmr);
    iecf_R_TRIG_init(&a->p_trig);
    iecf_R_TRIG_init(&a->p1_trig);
    a->q_ = a->p0_ = a->Q = a->A = 0;
}
static inline void iecf_ton_in(iecf_TON_t *fb, IEC_BOOL in, IEC_TIME pt, int64_t now) {
    fb->IN = in;
    fb->PT = pt;
    iecf_TON(fb, now);
}
static inline void iecf_rs_in(iecf_RS_t *fb, IEC_BOOL s, IEC_BOOL r1) {
    fb->S = s;
    fb->R1 = r1;
    iecf_RS(fb);
}
static inline void iecf_action(iecf_fault_t *g, iecf_action_t *a, unsigned q, IEC_TIME t, int64_t now) {
    IEC_BOOL r = (q & IECF_Q_R) != 0u, p0 = (q & IECF_Q_P0) != 0u, Q;
    /* Ed.3 6.7.4.6 d: SD while its SL_FF is set, or SL while its SD_FF is. */
    if (((q & IECF_Q_SD) != 0u && a->sl_ff.Q1) || ((q & IECF_Q_SL) != 0u && a->sd_ff.Q1)) iecf_flag(g, IECF_FAULT_SFC);
    iecf_rs_in(&a->s_ff, (q & IECF_Q_S) != 0u, r);
    iecf_ton_in(&a->l_tmr, (q & IECF_Q_L) != 0u, t, now);
    iecf_ton_in(&a->d_tmr, (q & IECF_Q_D) != 0u, t, now);
    a->p_trig.CLK = (q & IECF_Q_P) != 0u;
    iecf_R_TRIG(&a->p_trig);
    iecf_rs_in(&a->sd_ff, (q & IECF_Q_SD) != 0u, r);
    iecf_ton_in(&a->sd_tmr, a->sd_ff.Q1, t, now);
    iecf_ton_in(&a->ds_tmr, (q & IECF_Q_DS) != 0u, t, now);
    iecf_rs_in(&a->ds_ff, a->ds_tmr.Q, r);
    iecf_rs_in(&a->sl_ff, (q & IECF_Q_SL) != 0u, r);
    iecf_ton_in(&a->sl_tmr, a->sl_ff.Q1, t, now);
    Q = (IEC_BOOL)(!r && ((q & IECF_Q_N) != 0u || a->s_ff.Q1 || ((q & IECF_Q_L) != 0u && !a->l_tmr.Q) || a->d_tmr.Q ||
                          a->p_trig.Q || a->sd_tmr.Q || a->ds_ff.Q1 || (a->sl_ff.Q1 && !a->sl_tmr.Q)));
    a->p1_trig.CLK = (q & IECF_Q_P1) != 0u;
    iecf_R_TRIG(&a->p1_trig);
    a->A = (IEC_BOOL)(Q || (a->q_ && !Q) || a->p1_trig.Q || (a->p0_ && !p0));
    a->Q = Q;
    a->q_ = Q;
    a->p0_ = p0;
}

#endif /* IECF_H */
