/* iecf.h's arithmetic, conversions, shifts and FOR at their edges, against
 * values worked out by hand: each fault where it belongs, no undefined
 * behaviour anywhere (CI also runs this on 32-bit wasm).
 * cc -std=c99 -pedantic -Wall -Wextra -Werror -I runtime runtime/tests/arith.c && ./a.out */
#include "iecf.h"

static int failures;
#define CHECK(cond) \
    do { \
        if (!(cond)) failures++; \
    } while (0)

static iecf_fault_t f;
static uint16_t take(void) {
    uint16_t c = f.code;
    f.code = IECF_FAULT_NONE;
    return c;
}

int main(void) {
    /* INT: in range, then one past each end. */
    CHECK(iecf_add_INT(&f, 32000, 767) == 32767 && take() == IECF_FAULT_NONE);
    CHECK(iecf_add_INT(&f, 32000, 768) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_sub_INT(&f, -32768, 1) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_INT(&f, 256, 128) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_INT(&f, -256, 128) == -32768 && take() == IECF_FAULT_NONE);
    /* Division truncates toward zero (Ed.3 Table 29 d); by zero faults. */
    CHECK(iecf_div_INT(&f, -7, 3) == -2 && take() == IECF_FAULT_NONE);
    CHECK(iecf_div_INT(&f, 7, 0) == 0 && take() == IECF_FAULT_DIVISION_BY_ZERO);
    CHECK(iecf_div_INT(&f, -32768, -1) == 0 && take() == IECF_FAULT_OVERFLOW);
    /* MOD: IN1 - (IN1/IN2)*IN2, 0 by zero (Table 29 e). */
    CHECK(iecf_mod_INT(-7, 3) == -1 && iecf_mod_INT(7, -3) == 1 && iecf_mod_INT(7, 0) == 0);
    CHECK(iecf_mod_INT(-32768, -1) == 0);
    CHECK(iecf_neg_INT(&f, -32768) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_abs_SINT(&f, -128) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_abs_SINT(&f, -127) == 127 && take() == IECF_FAULT_NONE);

    /* LINT, checked before the operation. */
    CHECK(iecf_add_LINT(&f, INT64_MAX, 1) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_sub_LINT(&f, INT64_MIN, 1) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_LINT(&f, INT64_MIN, -1) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_LINT(&f, -4294967296LL, 2147483648LL) == INT64_MIN && take() == IECF_FAULT_NONE);
    CHECK(iecf_mul_LINT(&f, 4294967296LL, 2147483648LL) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_div_LINT(&f, INT64_MIN, -1) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mod_LINT(INT64_MIN, -1) == 0);
    CHECK(iecf_abs_LINT(&f, INT64_MIN) == 0 && take() == IECF_FAULT_OVERFLOW);

    /* Unsigned: below zero is an overflow too. */
    CHECK(iecf_sub_USINT(&f, 3, 4) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_add_UDINT(&f, 4294967295u, 1u) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_ULINT(&f, UINT64_C(4294967296), UINT64_C(4294967296)) == 0 && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_mul_UINT(&f, 255, 257) == 65535 && take() == IECF_FAULT_NONE);
    CHECK(iecf_mod_UINT(7, 0) == 0);

    /* Conversions keep the value or fault. */
    CHECK(iecf_s_in(&f, -129, INT8_MIN, INT8_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_su_in(&f, -1, UINT16_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_us_in(&f, UINT64_C(32768), INT16_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_us_in(&f, UINT64_C(32767), INT16_MAX) == 32767 && take() == IECF_FAULT_NONE);
    CHECK(iecf_to_i8(0x80) == -128 && iecf_to_i8(0x7F) == 127 && iecf_to_i16(0xFFFF) == -1);

    /* Shifts (Table 30 example) and their edges. */
    CHECK(iecf_shl_BYTE_u(0x19, 3) == 0xC8 && iecf_shr_BYTE_u(0x19, 3) == 0x03);
    CHECK(iecf_rol_BYTE_u(0x19, 3) == 0xC8 && iecf_ror_BYTE_u(0x19, 3) == 0x23);
    CHECK(iecf_shl_WORD_u(0xFFFF, 16) == 0 && iecf_shr_LWORD_u(UINT64_MAX, 64) == 0);
    CHECK(iecf_rol_WORD_u(0x1234, 20) == 0x2341 && iecf_ror_LWORD_u(1, 65) == UINT64_C(0x8000000000000000));
    CHECK(iecf_shl_WORD(&f, 1, -1) == 0 && take() == IECF_FAULT_NEGATIVE);

    /* FOR to the type's last value ends, without overflow. */
    {
        int8_t i;
        int n = 0, done;
        for (i = 120, done = iecf_for_done_SINT(i, 127, 3); !done; done = iecf_for_next_SINT(&i, 127, 3)) n++;
        CHECK(n == 3 && i == 126); /* 120, 123, 126 */
        n = 0;
        for (i = -126, done = iecf_for_done_SINT(i, -128, -1); !done; done = iecf_for_next_SINT(&i, -128, -1)) n++;
        CHECK(n == 3 && i == -128);
    }
    {
        uint8_t u;
        int n = 0, done;
        for (u = 250, done = iecf_for_done_USINT(u, 255, 5); !done; done = iecf_for_next_USINT(&u, 255, 5)) n++;
        CHECK(n == 2 && u == 255);
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
    CHECK(iecf_round_s(&f, 2.5, INT16_MIN, INT16_MAX) == 2 && iecf_round_s(&f, -2.5, INT16_MIN, INT16_MAX) == -2);
    CHECK(iecf_round_s(&f, 1.5, INT16_MIN, INT16_MAX) == 2 && iecf_round_s(&f, -1.6, INT16_MIN, INT16_MAX) == -2);
    CHECK(iecf_round_s(&f, 0.4999999999999999, INT16_MIN, INT16_MAX) == 0 && take() == IECF_FAULT_NONE);
    CHECK(iecf_round_s(&f, 32767.5, INT16_MIN, INT16_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_round_s(&f, 32767.4, INT16_MIN, INT16_MAX) == 32767 && take() == IECF_FAULT_NONE);
    CHECK(iecf_round_s(&f, -9223372036854775808.0, INT64_MIN, INT64_MAX) == INT64_MIN && take() == IECF_FAULT_NONE);
    CHECK(iecf_round_s(&f, 9223372036854775808.0, INT64_MIN, INT64_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_round_s(&f, iecf_bits_lreal(UINT64_C(0x7FF8000000000000)), INT16_MIN, INT16_MAX) == 0 &&
          take() == IECF_FAULT_CONVERSION); /* NaN */
    CHECK(iecf_round_u(&f, -0.5, UINT8_MAX) == 0 && take() == IECF_FAULT_NONE);
    CHECK(iecf_round_u(&f, -0.6, UINT8_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_round_u(&f, 254.5, UINT8_MAX) == 254 && iecf_round_u(&f, 255.5, UINT8_MAX) == 0 &&
          take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_trunc_s(&f, -1.6, INT16_MIN, INT16_MAX) == -1 && iecf_trunc_s(&f, 1.6, INT16_MIN, INT16_MAX) == 1);
    CHECK(iecf_trunc_u(&f, -0.9, UINT16_MAX) == 0 && take() == IECF_FAULT_NONE);
    CHECK(iecf_trunc_u(&f, -1.0, UINT16_MAX) == 0 && take() == IECF_FAULT_CONVERSION);
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
    CHECK(iecf_real_bits(1.0f) == 0x3F800000u && iecf_bits_real(0xC0000000u) == -2.0f);
    CHECK(iecf_lreal_bits(iecf_abs_LREAL(-0.0)) == 0u && iecf_abs_REAL(-2.5f) == 2.5f);

    CHECK(iecf_limit_INT(-100, 300, 100) == 100 && iecf_min_UINT(3, 2) == 2 && iecf_max_SINT(-3, -2) == -2);
    return failures;
}
