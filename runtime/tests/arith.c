/* iecf.h's arithmetic, conversions, shifts and FOR at their edges, against
 * values worked out by hand: each fault and flag where it belongs, no undefined
 * behaviour anywhere (CI also runs this on 32-bit wasm).
 * cc -std=c99 -pedantic -Wall -Wextra -Werror -I runtime runtime/tests/arith.c && ./a.out */
#include "iecf.h"

static int failures;
#define CHECK(cond) \
    do { \
        if (!(cond)) failures++; \
    } while (0)

static iecf_fault_t f;
static int keep; /* set by a real converted to an integer it does not fit */
static int kept(void) {
    int k = keep;
    keep = 0;
    return k;
}
static uint16_t take(void) {
    uint16_t c = f.code;
    f.code = IECF_FAULT_NONE;
    return c;
}

/* A double and a float function for IECF_REAL_FN, counting their calls:
 * which one a REAL's call takes, by IECF_REAL_FLOAT_MATH. */
static int doubles, floats;
static double twice_d(double x) {
    doubles++;
    return x * 2.0;
}
static float twice_f(float x) {
    floats++;
    return x * 2.0f;
}
static double sum_d(double x, double y) {
    doubles++;
    return x + y;
}
static float sum_f(float x, float y) {
    floats++;
    return x + y;
}

/* DINT's and LINT's + and - at every pair of edge values, against a
 * reference worked out apart (a DINT in 64 bits; a LINT by its bounds):
 * the value wrapped and the flag, whichever test iecf.h takes - the
 * compiler's, C99's (IECF_NO_CKD) or the bits' (IECF_BIT_TESTS). Their
 * _or forms too. */
static void addsub_edges(void) {
    static const int32_t d[] = {INT32_MIN, INT32_MIN + 1, -65536, -2, -1, 0, 1, 2, 65535, INT32_MAX - 1, INT32_MAX};
    static const int64_t l[] = {INT64_MIN, INT64_MIN + 1, -4294967296, -1, 0, 1, 4294967296, INT64_MAX - 1, INT64_MAX};
    size_t i, j;
    for (i = 0; i < sizeof d / sizeof d[0]; i++)
        for (j = 0; j < sizeof d / sizeof d[0]; j++) {
            int64_t s = (int64_t)d[i] + d[j], t = (int64_t)d[i] - d[j];
            unsigned any = 0u;
            CHECK(iecf_add_DINT(&f, d[i], d[j]) == iecf_to_i32((uint32_t)(uint64_t)s));
            CHECK(take() == (s < INT32_MIN || s > INT32_MAX ? IECF_FAULT_OVERFLOW : IECF_FAULT_NONE));
            CHECK(iecf_sub_DINT(&f, d[i], d[j]) == iecf_to_i32((uint32_t)(uint64_t)t));
            CHECK(take() == (t < INT32_MIN || t > INT32_MAX ? IECF_FAULT_OVERFLOW : IECF_FAULT_NONE));
            (void)iecf_add_DINT_or(&any, d[i], d[j]);
            CHECK(any == (s < INT32_MIN || s > INT32_MAX ? 1u : 0u));
            any = 0u;
            (void)iecf_sub_DINT_or(&any, d[i], d[j]);
            CHECK(any == (t < INT32_MIN || t > INT32_MAX ? 1u : 0u));
        }
    for (i = 0; i < sizeof l / sizeof l[0]; i++)
        for (j = 0; j < sizeof l / sizeof l[0]; j++) {
            int64_t a = l[i], b = l[j];
            int add_over = (b > 0 && a > INT64_MAX - b) || (b < 0 && a < INT64_MIN - b);
            int sub_over = (b < 0 && a > INT64_MAX + b) || (b > 0 && a < INT64_MIN + b);
            CHECK(iecf_add_LINT(&f, a, b) == iecf_to_i64((uint64_t)a + (uint64_t)b));
            CHECK(take() == (add_over ? IECF_FAULT_OVERFLOW : IECF_FAULT_NONE));
            CHECK(iecf_sub_LINT(&f, a, b) == iecf_to_i64((uint64_t)a - (uint64_t)b));
            CHECK(take() == (sub_over ? IECF_FAULT_OVERFLOW : IECF_FAULT_NONE));
        }
}

int main(void) {
    addsub_edges();
    /* INT: in range, then one past each end - wrapped and flagged. Here
     * one iecf_fault_t takes both faults and flags. */
    /* A signed integer shifted (integer-shifts): SHR keeps the sign. */
    CHECK(iecf_shr_INT_u(-8, 1u) == -4 && iecf_shr_INT_u(-1, 3u) == -1 && iecf_shr_INT_u(-32768, 15u) == -1);
    CHECK(iecf_shr_INT_u(100, 2u) == 25 && iecf_shr_INT_u(-5, 16u) == -1 && iecf_shr_INT_u(5, 99u) == 0);
    CHECK(iecf_shr_SINT_u(-128, 7u) == -1 && iecf_shr_DINT_u(INT32_MIN, 31u) == -1);
    CHECK(iecf_shr_LINT_u(INT64_MIN, 63u) == -1 && iecf_shr_LINT_u(INT64_MIN, 1u) == INT64_MIN / 2);
    CHECK(iecf_shl_INT_u(16384, 1u) == -32768 && iecf_shl_INT_u(-1, 15u) == -32768 && iecf_shl_INT_u(-1, 16u) == 0);
    CHECK(iecf_shl_SINT_u(-1, 1u) == -2 && iecf_shl_DINT_u(1, 31u) == INT32_MIN);
    CHECK(iecf_rol_INT_u(-32768, 1u) == 1 && iecf_ror_INT_u(1, 1u) == -32768 && iecf_rol_SINT_u(-127, 1u) == 3);
    CHECK(iecf_ror_LINT_u(1, 1u) == INT64_MIN && iecf_rol_DINT_u(-1, 7u) == -1);
    {
        iecf_fault_t f = {IECF_FAULT_NONE, 0u, 0u};
        CHECK(iecf_shr_INT(&f, -8, -1) == 0 && f.code == IECF_FAULT_NEGATIVE);
    }
    CHECK(iecf_add_INT(&f, 32000, 767) == 32767 && take() == IECF_FAULT_NONE);
    CHECK(iecf_add_INT(&f, 32000, 768) == -32768 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_sub_INT(&f, -32768, 1) == 32767 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_INT(&f, 256, 128) == -32768 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_INT(&f, 300, 300) == 24464 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_INT(&f, -256, 128) == -32768 && take() == IECF_FAULT_NONE);
    /* Division truncates toward zero (Ed.3 Table 29 d); by zero faults. */
    CHECK(iecf_div_INT(&f, &f, -7, 3) == -2 && take() == IECF_FAULT_NONE);
    CHECK(iecf_div_INT(&f, &f, 7, 0) == 0 && take() == IECF_FAULT_DIVISION_BY_ZERO);
    CHECK(iecf_div_INT(&f, &f, -32768, -1) == -32768 && take() == IECF_FAULT_OVERFLOW);
    /* MOD: IN1 - (IN1/IN2)*IN2, 0 by zero (Table 29 e). */
    CHECK(iecf_mod_INT(-7, 3) == -1 && iecf_mod_INT(7, -3) == 1 && iecf_mod_INT(7, 0) == 0);
    CHECK(iecf_mod_INT(-32768, -1) == 0);
    CHECK(iecf_neg_INT(&f, -32768) == -32768 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_abs_SINT(&f, -128) == -128 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_abs_SINT(&f, -127) == 127 && take() == IECF_FAULT_NONE);
    CHECK(iecf_add_DINT(&f, INT32_MAX, INT32_MAX) == -2 && take() == IECF_FAULT_OVERFLOW);

    /* LINT, tested before the operation, wrapped in unsigned bits. */
    CHECK(iecf_add_LINT(&f, INT64_MAX, 1) == INT64_MIN && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_sub_LINT(&f, INT64_MIN, 1) == INT64_MAX && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_LINT(&f, INT64_MIN, -1) == INT64_MIN && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_LINT(&f, -4294967296LL, 2147483648LL) == INT64_MIN && take() == IECF_FAULT_NONE);
    CHECK(iecf_mul_LINT(&f, 4294967296LL, 2147483648LL) == INT64_MIN && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_LINT(&f, -3, 5) == -15 && take() == IECF_FAULT_NONE);
    CHECK(iecf_div_LINT(&f, &f, INT64_MIN, -1) == INT64_MIN && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_div_LINT(&f, &f, 7, -1) == -7 && take() == IECF_FAULT_NONE);
    CHECK(iecf_div_LINT(&f, &f, 7, 0) == 0 && take() == IECF_FAULT_DIVISION_BY_ZERO);
    CHECK(iecf_mod_LINT(INT64_MIN, -1) == 0);
    CHECK(iecf_abs_LINT(&f, INT64_MIN) == INT64_MIN && take() == IECF_FAULT_OVERFLOW);

    /* Unsigned: below zero is an overflow too. */
    CHECK(iecf_sub_USINT(&f, 3, 4) == 255 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_add_UDINT(&f, 4294967295u, 1u) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_ULINT(&f, UINT64_C(4294967296), UINT64_C(4294967297)) == UINT64_C(4294967296) && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_UINT(&f, 255, 257) == 65535 && take() == IECF_FAULT_NONE);
    CHECK(iecf_div_UINT(&f, 7, 0) == 0 && take() == IECF_FAULT_DIVISION_BY_ZERO);
    CHECK(iecf_mod_UINT(7, 0) == 0);

    /* Conversions keep the value, or its low bits and a flag. */
    CHECK(iecf_to_i8((uint8_t)iecf_s_in(&f, -129, INT8_MIN, INT8_MAX)) == 127 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_i8((uint8_t)iecf_s_in(&f, 4223, INT8_MIN, INT8_MAX)) == 127 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_i8((uint8_t)iecf_s_in(&f, -100, INT8_MIN, INT8_MAX)) == -100 && take() == IECF_FAULT_NONE);
    CHECK((uint16_t)iecf_su_in(&f, -1, UINT16_MAX) == 65535 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_i16((uint16_t)iecf_us_in(&f, UINT64_C(32768), INT16_MAX)) == -32768 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_i16((uint16_t)iecf_us_in(&f, UINT64_C(32767), INT16_MAX)) == 32767 && take() == IECF_FAULT_NONE);
    CHECK(iecf_to_i64(iecf_us_in(&f, UINT64_MAX, INT64_MAX)) == -1 && take() == IECF_FAULT_CONVERSION);
    CHECK((uint8_t)iecf_u_in(&f, 300u, UINT8_MAX) == 44 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_i8(0x80) == -128 && iecf_to_i8(0x7F) == 127 && iecf_to_i16(0xFFFF) == -1);

    /* Subranges: clamped to the bound passed, and flagged. */
    CHECK(iecf_clamp_s(&f, 150, 0, 100) == 100 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_clamp_s(&f, -5, 0, 100) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_clamp_s(&f, 42, 0, 100) == 42 && take() == IECF_FAULT_NONE);
    CHECK(iecf_clamp_u(&f, 1, 2, 9) == 2 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_for_end_s(&f, 150, 1, 1, 3) == 3 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_for_end_s(&f, -5, 1, 1, 3) == -5 && take() == IECF_FAULT_NONE);
    CHECK(iecf_for_end_s(&f, -5, -1, 1, 3) == 1 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_for_end_u(&f, 9, 3) == 3 && take() == IECF_FAULT_CONVERSION);

    /* Array indices: the offset from the lower bound, out of bounds a fault. */
    CHECK(iecf_index(&f, -1, -1, 1) == 0 && iecf_index(&f, 1, -1, 1) == 2 && take() == IECF_FAULT_NONE);
    CHECK(iecf_index(&f, 2, -1, 1) == 0 && take() == IECF_FAULT_INDEX);
    CHECK(iecf_index(&f, INT64_MIN, -1, 1) == 0 && take() == IECF_FAULT_INDEX);
    CHECK(iecf_index_u(&f, UINT64_MAX, 0, 3) == 0 && take() == IECF_FAULT_INDEX);
    CHECK(iecf_index_u(&f, 3, 1, 4) == 2 && take() == IECF_FAULT_NONE);
    CHECK(iecf_index_u(&f, 0, 1, 4) == 0 && take() == IECF_FAULT_INDEX);

    /* Shifts (Table 30 example) and their edges. */
    CHECK(iecf_shl_BYTE_u(0x19, 3) == 0xC8 && iecf_shr_BYTE_u(0x19, 3) == 0x03);
    CHECK(iecf_rol_BYTE_u(0x19, 3) == 0xC8 && iecf_ror_BYTE_u(0x19, 3) == 0x23);
    CHECK(iecf_shl_WORD_u(0xFFFF, 16) == 0 && iecf_shr_LWORD_u(UINT64_MAX, 64) == 0);
    CHECK(iecf_rol_WORD_u(0x1234, 20) == 0x2341 && iecf_ror_LWORD_u(1, 65) == UINT64_C(0x8000000000000000));
    CHECK(iecf_shl_WORD(&f, 1, -1) == 0 && take() == IECF_FAULT_NEGATIVE);

    /* FOR: after the loop, one step past the end (Ed.3 7.3.3.4.2's
     * examples, CODESYS); to the type's last value the loop still ends,
     * the step past wrapped and flagged. */
    {
        int16_t k;
        int n = 0, done;
        for (k = 1, done = iecf_for_done_INT(k, 10, 1); !done; done = iecf_for_next_INT(&f, &k, 10, 1)) n++;
        CHECK(n == 10 && k == 11 && take() == 0u);
        n = 0;
        for (k = 3, done = iecf_for_done_INT(k, 1, -1); !done; done = iecf_for_next_INT(&f, &k, 1, -1)) n++;
        CHECK(n == 3 && k == 0 && take() == 0u); /* "terminates when ... I reaches 0" */
        n = 0;
        for (k = 5, done = iecf_for_done_INT(k, 1, 1); !done; done = iecf_for_next_INT(&f, &k, 1, 1)) n++;
        CHECK(n == 0 && k == 5); /* not run: the start, as written */
        /* The body moving it outside the TO range ends the loop (Ed.3:
         * "terminated when ... outside the range"): 1 3, 4 6, 7 9, 10 12. */
        n = 0;
        for (k = 1, done = iecf_for_done_INT(k, 10, 1); !done; done = iecf_for_next_INT(&f, &k, 10, 1)) {
            k = (int16_t)(k + 2);
            n++;
        }
        CHECK(n == 4 && k == 13 && take() == 0u);
        n = 0;
        for (k = 10, done = iecf_for_done_INT(k, 1, -1); !done; done = iecf_for_next_INT(&f, &k, 1, -1)) {
            k = (int16_t)(k - 5);
            n++;
        }
        CHECK(n == 2 && k == -2 && take() == 0u); /* 10 5, 4 -1, then one step past */
    }
    {
        int8_t i;
        int n = 0, done;
        for (i = 120, done = iecf_for_done_SINT(i, 127, 3); !done; done = iecf_for_next_SINT(&f, &i, 127, 3)) n++;
        CHECK(n == 3 && i == -127 && take() == IECF_FAULT_OVERFLOW); /* 120, 123, 126; 129 wraps */
        n = 0;
        for (i = -126, done = iecf_for_done_SINT(i, -128, -1); !done; done = iecf_for_next_SINT(&f, &i, -128, -1)) n++;
        CHECK(n == 3 && i == 127 && take() == IECF_FAULT_OVERFLOW);
        n = 0; /* a subrange's variable ends at its last value, inside */
        for (i = 120, done = iecf_for_done_SINT(i, 127, 3); !done; done = iecf_for_next_in_SINT(&i, 127, 3)) n++;
        CHECK(n == 3 && i == 126 && take() == 0u);
    }
    {
        uint8_t u;
        int n = 0, done;
        for (u = 250, done = iecf_for_done_USINT(u, 255, 5); !done; done = iecf_for_next_USINT(&f, &u, 255, 5)) n++;
        CHECK(n == 2 && u == 4 && take() == IECF_FAULT_OVERFLOW); /* 250, 255; 260 wraps */
    }

    /* Counters stop at their type's ends (docs/dependencies.md), never wrap. */
    {
        iecf_CTU_t up;
        iecf_CTD_t down;
        iecf_CTUD_UDINT_t ud;
        int k;
        iecf_CTU_init(&up);
        up.CV = 32766;
        for (k = 0; k < 3; k++) {
            up.CU = 1;
            iecf_CTU(&up);
            up.CU = 0;
            iecf_CTU(&up);
        }
        CHECK(up.CV == 32767);
        iecf_CTD_init(&down);
        down.CV = -32767;
        for (k = 0; k < 3; k++) {
            down.CD = 1;
            iecf_CTD(&down);
            down.CD = 0;
            iecf_CTD(&down);
        }
        CHECK(down.CV == -32768 && down.Q == 1);
        iecf_CTUD_UDINT_init(&ud);
        ud.CD = 1;
        iecf_CTUD_UDINT(&ud);
        CHECK(ud.CV == 0u && ud.QD == 1);
        ud.CD = 0;
        iecf_CTUD_UDINT(&ud);
        ud.CU = 1;
        ud.CD = 1;
        iecf_CTUD_UDINT(&ud); /* both edges at once: neither counts */
        CHECK(ud.CV == 0u);
    }
    /* TON in LTIME: ET in ns, Q at PT. */
    {
        iecf_TON_LTIME_t t;
        iecf_TON_LTIME_init(&t);
        t.IN = 1;
        t.PT = 1500;
        iecf_TON_LTIME(&t, 1000);
        iecf_TON_LTIME(&t, 2499);
        CHECK(t.ET == 1499 && t.Q == 0);
        iecf_TON_LTIME(&t, 2500);
        CHECK(t.ET == 1500 && t.Q == 1);
        t.IN = 0;
        iecf_TON_LTIME(&t, 2600);
        CHECK(t.ET == 0 && t.Q == 0);
    }

    /* Reals to integers: half to even (Ed.3 6.6.2.5.3), TRUNC toward 0. */
    CHECK(iecf_round_s(&f, &keep, 2.5, INT16_MIN, INT16_MAX) == 2 && iecf_round_s(&f, &keep, -2.5, INT16_MIN, INT16_MAX) == -2);
    CHECK(iecf_round_s(&f, &keep, 1.5, INT16_MIN, INT16_MAX) == 2 && iecf_round_s(&f, &keep, -1.6, INT16_MIN, INT16_MAX) == -2);
    CHECK(iecf_round_s(&f, &keep, 0.4999999999999999, INT16_MIN, INT16_MAX) == 0 && take() == IECF_FAULT_NONE);
    CHECK(kept() == 0);
    CHECK(iecf_round_s(&f, &keep, 32767.5, INT16_MIN, INT16_MAX) == 0 && take() == IECF_FAULT_CONVERSION && kept());
    CHECK(iecf_round_s(&f, &keep, 32767.4, INT16_MIN, INT16_MAX) == 32767 && take() == IECF_FAULT_NONE);
    CHECK(iecf_round_s(&f, &keep, -9223372036854775808.0, INT64_MIN, INT64_MAX) == INT64_MIN && take() == IECF_FAULT_NONE);
    CHECK(iecf_round_s(&f, &keep, 9223372036854775808.0, INT64_MIN, INT64_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_round_s(&f, &keep, iecf_bits_lreal(UINT64_C(0x7FF8000000000000)), INT16_MIN, INT16_MAX) == 0 &&
          take() == IECF_FAULT_CONVERSION); /* NaN */
    CHECK(iecf_round_u(&f, &keep, -0.5, UINT8_MAX) == 0 && take() == IECF_FAULT_NONE);
    CHECK(iecf_round_u(&f, &keep, -0.6, UINT8_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_round_u(&f, &keep, 254.5, UINT8_MAX) == 254 && iecf_round_u(&f, &keep, 255.5, UINT8_MAX) == 0 &&
          take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_trunc_s(&f, &keep, -1.6, INT16_MIN, INT16_MAX) == -1 && iecf_trunc_s(&f, &keep, 1.6, INT16_MIN, INT16_MAX) == 1);
    CHECK(iecf_trunc_u(&f, &keep, -0.9, UINT16_MAX) == 0 && take() == IECF_FAULT_NONE);
    CHECK(iecf_trunc_u(&f, &keep, -1.0, UINT16_MAX) == 0 && take() == IECF_FAULT_CONVERSION && kept());
    CHECK(iecf_trunc_u(&f, &keep, 65535.9, UINT16_MAX) == 65535 && take() == IECF_FAULT_NONE && !kept());
    /* An infinite or NaN REAL carries on and is flagged (in g), not faulted. */
    {
        iecf_fault_t g = {0, 0, 0};
        IEC_REAL big = iecf_real_of(&g, 1e300);
        CHECK(big == iecf_bits_real(0x7F800000u) && g.code == IECF_FAULT_OVERFLOW && f.code == IECF_FAULT_NONE);
        g.code = IECF_FAULT_NONE;
        CHECK(iecf_real_of(&g, 0.1) == 0.1f && g.code == IECF_FAULT_NONE);
        CHECK(iecf_mul_REAL(&g, 3e38f, 10.0f) == big && g.code == IECF_FAULT_OVERFLOW);
        g.code = IECF_FAULT_NONE;
        CHECK(iecf_sub_REAL(&g, big, big) != 0.0f && g.code == IECF_FAULT_NAN); /* inf - inf: NaN, unequal to all */
        iecf_noted(&g, 3, 42);
        CHECK(g.pou == 3 && g.line == 42);
        iecf_noted(&g, 4, 43);
        CHECK(g.line == 42); /* the first stays */
        g.code = IECF_FAULT_NONE;
        CHECK(iecf_add_LREAL(&g, 1.0, 2.0) == 3.0 && g.code == IECF_FAULT_NONE);
        CHECK(iecf_div_REAL(&f, &g, 1.0f, 0.0f) == 0.0f && take() == IECF_FAULT_DIVISION_BY_ZERO);
        CHECK(iecf_div_LREAL(&f, &g, 1e300, 1e-300) != 0.0 && g.code == IECF_FAULT_OVERFLOW && take() == IECF_FAULT_NONE);
    }
    /* Finite on the bits: the largest, the smallest subnormal and -0 are;
     * an infinity of either sign and a NaN are not. */
    CHECK(iecf_finite_REAL(FLT_MAX) && iecf_finite_REAL(iecf_bits_real(1u)) && iecf_finite_REAL(-0.0f));
    CHECK(!iecf_finite_REAL(iecf_bits_real(0x7F800000u)) && !iecf_finite_REAL(iecf_bits_real(0xFF800000u)));
    CHECK(!iecf_finite_REAL(iecf_bits_real(0x7FC00000u)) && !iecf_finite_REAL(iecf_bits_real(0xFF800001u)));
    CHECK(iecf_finite_LREAL(DBL_MAX) && iecf_finite_LREAL(-DBL_MAX) && iecf_finite_LREAL(4.9e-324));
    CHECK(!iecf_finite_LREAL(iecf_bits_lreal(UINT64_C(0x7FF0000000000000))));
    CHECK(!iecf_finite_LREAL(iecf_bits_lreal(UINT64_C(0xFFF0000000000000))));
    CHECK(!iecf_finite_LREAL(iecf_bits_lreal(UINT64_C(0x7FF8000000000001))));
    /* A REAL's Table 28 function: one call, of the double function or of
     * the float one; the result the same here, flagged the same. */
    {
        iecf_fault_t g = {0, 0, 0};
        (void)twice_d; /* one pair is called, the other not, by the mode */
        (void)twice_f;
        (void)sum_d;
        (void)sum_f;
        CHECK(IECF_REAL_FN(&g, twice_d, twice_f, 1.25f) == 2.5f && g.code == IECF_FAULT_NONE);
        CHECK(IECF_REAL_FN(&g, twice_d, twice_f, 3e38f) == iecf_bits_real(0x7F800000u) && g.code == IECF_FAULT_OVERFLOW);
        g.code = IECF_FAULT_NONE;
        CHECK(IECF_REAL_FN(&g, twice_d, twice_f, iecf_bits_real(0x7FC00000u)) != 0.0f && g.code == IECF_FAULT_NAN);
        g.code = IECF_FAULT_NONE;
        CHECK(IECF_REAL_FN2(&g, sum_d, sum_f, 1.0f, 0.5f) == 1.5f && g.code == IECF_FAULT_NONE);
        CHECK(IECF_REAL_RAW(twice_d, twice_f, 3e38f) == iecf_bits_real(0x7F800000u) && g.code == IECF_FAULT_NONE);
#if defined(IECF_REAL_FLOAT_MATH) && IECF_REAL_FLOAT_MATH
        CHECK(floats == 5 && doubles == 0);
#else
        CHECK(doubles == 5 && floats == 0);
#endif
    }
    CHECK(iecf_real_bits(1.0f) == 0x3F800000u && iecf_bits_real(0xC0000000u) == -2.0f);
    CHECK(iecf_lreal_bits(iecf_abs_LREAL(-0.0)) == 0u && iecf_abs_REAL(-2.5f) == 2.5f);

    CHECK(iecf_limit_INT(-100, 300, 100) == 100 && iecf_min_UINT(3, 2) == 2 && iecf_max_SINT(-3, -2) == -2);
    CHECK(iecf_sel_INT(0, 4, 9) == 4 && iecf_sel_INT(1, 4, 9) == 9 && iecf_sel_LREAL(1, 0.5, 2.5) == 2.5);

    /* One statement's overflow across a loop: the value wrapped as the
     * helper's, the overflow ORed into the loop's local, taken after. */
    {
        unsigned any = 0u;
        iecf_fault_t g = {IECF_FAULT_NONE, 0u, 0u};
        CHECK(iecf_add_DINT_or(&any, 1, 2) == 3 && any == 0u);
        CHECK(iecf_add_DINT_or(&any, INT32_MAX, 1) == INT32_MIN && any == 1u);
        CHECK(iecf_add_DINT_or(&any, 1, 2) == 3 && any == 1u); /* stays */
        any = 0u;
        CHECK(iecf_mul_INT_or(&any, 300, 300) == 24464 && any == 1u);
        any = 0u;
        CHECK(iecf_neg_SINT_or(&any, -128) == -128 && any == 1u);
        any = 0u;
        CHECK(iecf_sub_UDINT_or(&any, 0u, 1u) == UINT32_MAX && any == 1u);
        any = 0u;
        CHECK(iecf_mul_LINT_or(&any, INT64_C(3037000500), INT64_C(3037000500)) < 0 && any == 1u);
        any = 0u;
        CHECK(iecf_add_ULINT_or(&any, UINT64_MAX, 1u) == 0u && any == 1u);
        iecf_took_any(&g, 0u, 3u, 4u);
        CHECK(g.code == IECF_FAULT_NONE);
        iecf_took_any(&g, 1u, 3u, 4u);
        CHECK(g.code == IECF_FAULT_OVERFLOW && g.pou == 3u && g.line == 4u);
        iecf_took_any(&g, 1u, 5u, 6u);
        CHECK(g.line == 4u); /* the first stays */
        CHECK(take() == IECF_FAULT_NONE);
    }

    /* IS_VALID_BCD (Ed.3 Table 39): each nibble 0-9; A-F anywhere is not. */
    CHECK(iecf_is_bcd(0u) && iecf_is_bcd(0x9999u) && iecf_is_bcd(UINT64_C(0x9999999999999999)));
    CHECK(!iecf_is_bcd(0xAu) && !iecf_is_bcd(0xC0u) && !iecf_is_bcd(0xF000u) && !iecf_is_bcd(UINT64_C(0xB000000000000000)));
    CHECK(!iecf_is_bcd(0x12A4u) && iecf_is_bcd(0x1289u));
    /* BCD to an integer (Table 22 footnote d's 16#0369 = 369); not BCD,
     * a nibble above 9 anywhere, gives 0, flagged. */
    CHECK(iecf_from_bcd(&f, 0x0369u, 4u) == 369u && take() == IECF_FAULT_NONE);
    CHECK(iecf_from_bcd(&f, UINT64_C(0x9999999999999999), 16u) == UINT64_C(9999999999999999) && take() == IECF_FAULT_NONE);
    CHECK(iecf_from_bcd(&f, 0u, 2u) == 0u && take() == IECF_FAULT_NONE);
    CHECK(iecf_from_bcd(&f, 0x1Fu, 2u) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_from_bcd(&f, 0xFFFFu, 4u) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_from_bcd(&f, UINT64_C(0xA000000000000001), 16u) == 0u && take() == IECF_FAULT_CONVERSION);
    /* An integer to BCD (footnote d's 25 = 16#25): more digits than the
     * nibbles, or a negative number, give every nibble F, flagged. */
    CHECK(iecf_to_bcd_u(&f, 25u, 2u) == 0x25u && take() == IECF_FAULT_NONE);
    CHECK(iecf_to_bcd_u(&f, 9999u, 4u) == 0x9999u && take() == IECF_FAULT_NONE);
    CHECK(iecf_to_bcd_u(&f, 10000u, 4u) == 0xFFFFu && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_bcd_u(&f, 100u, 2u) == 0xFFu && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_bcd_u(&f, 100000000u, 8u) == 0xFFFFFFFFu && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_bcd_u(&f, UINT64_C(9999999999999999), 16u) == UINT64_C(0x9999999999999999) && take() == IECF_FAULT_NONE);
    CHECK(iecf_to_bcd_u(&f, UINT64_C(10000000000000000), 16u) == UINT64_MAX && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_bcd_u(&f, UINT64_MAX, 16u) == UINT64_MAX && take() == IECF_FAULT_CONVERSION);
    CHECK(!iecf_is_bcd(iecf_to_bcd_u(&f, 123u, 2u)) && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_bcd_s(&f, 1234, 4u) == 0x1234u && take() == IECF_FAULT_NONE);
    CHECK(iecf_to_bcd_s(&f, -1, 4u) == 0xFFFFu && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_bcd_s(&f, -1, 2u) == 0xFFu && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_bcd_s(&f, INT64_MIN, 16u) == UINT64_MAX && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_to_bcd_s(&f, 0, 2u) == 0u && take() == IECF_FAULT_NONE);
    /* Endianness (Table 37): the bytes in memory, whatever the host. */
    {
        IEC_DWORD d = iecf_endian_DWORD(0x12345678u, 1);
        IEC_DWORD l = iecf_endian_DWORD(0x12345678u, 0);
        IEC_LWORD q = iecf_endian_LWORD(UINT64_C(0x123456789ABCDEF0), 1);
        const unsigned char *b = (const unsigned char *)&d, *c = (const unsigned char *)&l;
        const unsigned char *e = (const unsigned char *)&q;
        unsigned char r[3] = {1u, 2u, 3u};
        CHECK(b[0] == 0x12u && b[1] == 0x34u && b[2] == 0x56u && b[3] == 0x78u);
        CHECK(c[0] == 0x78u && c[1] == 0x56u && c[2] == 0x34u && c[3] == 0x12u);
        CHECK(e[0] == 0x12u && e[3] == 0x78u && e[4] == 0x9Au && e[7] == 0xF0u);
        CHECK(iecf_endian_DWORD(d, 1) == 0x12345678u && iecf_endian_INT(iecf_endian_INT(-2, 0), 0) == -2);
        CHECK(iecf_endian_REAL(iecf_endian_REAL(1.5f, 1), 1) == 1.5f);
        CHECK(iecf_endian_LTIME(iecf_endian_LTIME(-7, 1), 1) == -7 && iecf_endian_DT(iecf_endian_DT(86400u, 0), 0) == 86400u);
        {
            IEC_DT t = iecf_endian_DT(86400u, 1); /* 16#00015180 */
            const unsigned char *p = (const unsigned char *)&t;
            CHECK(p[0] == 0x00u && p[1] == 0x01u && p[2] == 0x51u && p[3] == 0x80u);
        }
        iecf_reverse(r, 3u);
        CHECK(r[0] == 3u && r[1] == 2u && r[2] == 1u);
        iecf_reverse(r, 1u);
        CHECK(r[0] == 3u);
    }
    return failures;
}
