/* iecf.h's references (Ed.3 6.4.4.10), against values worked out by hand:
 * the same variable reached in C memory and in the process image.
 * cc -std=c99 -pedantic -Wall -Wextra -Werror -I runtime runtime/tests/refs.c && ./a.out */
#include "iecf.h"

static int failures;
#define CHECK(cond) \
    do { \
        if (!(cond)) failures++; \
    } while (0)

typedef struct {
    IEC_INT a;
    IEC_BYTE b;
} pair_t;

int main(void) {
    uint8_t i[4] = {0}, q[4] = {0}, m[24] = {0};
    iecf_image_t im;
    iecf_fault_t f = {0, 0, 0};
    IEC_INT x = 7, arr[3] = {1, 2, 3};
    IEC_BOOL flag = 0;
    pair_t pr = {0, 0};
    double nowhere = 0.0;
    iecf_ref_t r, n = iecf_ref_null();

    im.i = i;
    im.q = q;
    im.m = m;
    im.i_size = im.q_size = 4u;
    im.m_size = 24u;

    /* In C memory: read and written as the variable itself. */
    r = iecf_ref_c(&x);
    CHECK(iecf_rget_INT(&im, r) == 7);
    iecf_rset_INT(&im, r, -3);
    CHECK(x == -3);
    r = iecf_ref_c(&flag);
    iecf_rset_BOOL(&im, r, 1);
    CHECK(flag == 1 && iecf_rget_BOOL(&im, r) == 1);

    /* In the image: %MW2 little-endian, the sign from its bits. */
    r = iecf_ref_image((uint8_t)'M', 2u, IECF_NO_BIT);
    iecf_rset_INT(&im, r, -2);
    CHECK(m[2] == 0xFE && m[3] == 0xFF);
    CHECK(iecf_rget_INT(&im, r) == -2);
    /* A BOOL at %MX5.3: that bit only. */
    r = iecf_ref_image((uint8_t)'M', 5u, 3u);
    m[5] = 0x01;
    iecf_rset_BOOL(&im, r, 1);
    CHECK(m[5] == 0x09 && iecf_rget_BOOL(&im, r) == 1);
    /* A BOOL taking %MB6 whole: written as 1, any value but 0 TRUE. */
    r = iecf_ref_image((uint8_t)'M', 6u, IECF_NO_BIT);
    m[6] = 0x40;
    CHECK(iecf_rget_BOOL(&im, r) == 1);
    iecf_rset_BOOL(&im, r, 0);
    CHECK(m[6] == 0);
    /* %IB1 and %QD0: each area its own. */
    i[1] = 0x81;
    CHECK(iecf_rget_SINT(&im, iecf_ref_image((uint8_t)'I', 1u, IECF_NO_BIT)) == -127);
    iecf_rset_UDINT(&im, iecf_ref_image((uint8_t)'Q', 0u, IECF_NO_BIT), 0x01020304u);
    CHECK(q[0] == 0x04 && q[3] == 0x01);
    /* REAL by its IEEE bits. */
    iecf_rset_REAL(&im, iecf_ref_image((uint8_t)'M', 8u, IECF_NO_BIT), 1.0f);
    CHECK(iecf_get_d(m, 8) == 0x3F800000u);

    /* A part: element 2 is 2 C elements on, 2 packed INTs (4 bytes) on. */
    r = iecf_ref_at(iecf_ref_c(arr), 2u, sizeof(IEC_INT), 2u);
    CHECK(iecf_rget_INT(&im, r) == 3);
    r = iecf_ref_at(iecf_ref_image((uint8_t)'M', 10u, IECF_NO_BIT), 2u, sizeof(IEC_INT), 2u);
    CHECK(r.off == 14u && r.area == 'M');
    /* A field: by offsetof in C, by its packed offset in the image. */
    r = iecf_ref_at(iecf_ref_c(&pr), 1u, offsetof(pair_t, b), 2u);
    iecf_rset_BYTE(&im, r, 0xAB);
    CHECK(pr.b == 0xAB);
    r = iecf_ref_at(iecf_ref_image((uint8_t)'M', 10u, IECF_NO_BIT), 1u, offsetof(pair_t, b), 2u);
    CHECK(r.off == 12u);
    /* A STRING's bytes start where the reference points, in either place. */
    CHECK(iecf_ref_ptr(&im, iecf_ref_image((uint8_t)'M', 13u, IECF_NO_BIT)) == (void *)(m + 13));
    CHECK(iecf_ref_ptr(&im, iecf_ref_c(&x)) == (void *)&x);

    /* A WSTRING[2] through a reference: in C memory its own units, read
     * in place and written back unless it is them; at %MW13, an odd
     * byte, its units little-endian, copied out and back. */
    {
        IEC_WCHAR w[3] = {0x263Au, 0x62u, 0u}, d[3] = {0u, 0u, 0u}, s[3] = {0x41u, 0u, 0u};
        const IEC_WCHAR *g;
        r = iecf_ref_c(w);
        g = iecf_wref_get(d, 2u, &im, r);
        CHECK(g == w && d[0] == 0u);
        iecf_wref_put(&im, r, 2u, g);
        CHECK(w[0] == 0x263Au && w[1] == 0x62u);
        iecf_wref_put(&im, r, 2u, s);
        CHECK(w[0] == 0x41u && w[1] == 0u);
        r = iecf_ref_image((uint8_t)'M', 13u, IECF_NO_BIT);
        s[0] = 0x263Au;
        s[1] = 0x62u;
        s[2] = 0u;
        iecf_wref_put(&im, r, 2u, s);
        CHECK(m[13] == 0x3A && m[14] == 0x26 && m[15] == 0x62);
        g = iecf_wref_get(d, 2u, &im, iecf_ref_image((uint8_t)'M', 13u, IECF_NO_BIT));
        CHECK(g == d && d[0] == 0x263Au && d[1] == 0x62u);
    }

    /* Equal when they reference the same variable. */
    CHECK(iecf_ref_eq(iecf_ref_c(&x), iecf_ref_c(&x)) == 1);
    CHECK(iecf_ref_eq(iecf_ref_c(&x), iecf_ref_c(&flag)) == 0);
    CHECK(iecf_ref_eq(iecf_ref_image((uint8_t)'M', 5u, 3u), iecf_ref_image((uint8_t)'M', 5u, 3u)) == 1);
    CHECK(iecf_ref_eq(iecf_ref_image((uint8_t)'M', 5u, 3u), iecf_ref_image((uint8_t)'M', 5u, 2u)) == 0);
    CHECK(iecf_ref_eq(iecf_ref_image((uint8_t)'M', 0u, IECF_NO_BIT), iecf_ref_image((uint8_t)'Q', 0u, IECF_NO_BIT)) == 0);
    CHECK(iecf_ref_eq(n, iecf_ref_null()) == 1 && iecf_ref_is_null(n));
    CHECK(!iecf_ref_is_null(iecf_ref_image((uint8_t)'I', 0u, IECF_NO_BIT)));

    /* `^` of a reference: itself; of NULL, a fault and `nowhere`. */
    r = iecf_deref(&f, iecf_ref_c(&x), &nowhere);
    CHECK(r.p == (void *)&x && f.code == IECF_FAULT_NONE);
    r = iecf_deref(&f, n, &nowhere);
    CHECK(r.p == (void *)&nowhere && r.area == 0u && f.code == IECF_FAULT_NULL);

    return failures != 0;
}
