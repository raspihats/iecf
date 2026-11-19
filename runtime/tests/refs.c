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
    /* Pointers (the pointers extension): checked in their variable. */
    {
        iecf_fault_t pf = {IECF_FAULT_NONE, 0u, 0u}, pg = {IECF_FAULT_NONE, 0u, 0u};
        uint8_t pq[4] = {0u, 0u, 0u, 0u};
        iecf_image_t pim = {NULL, NULL, NULL, 0u, 0u, 0u};
        IEC_BYTE whole[4];
        IEC_BYTE bytes[8] = {1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u};
        IEC_REAL r = 1.0f;
        IEC_BOOL b = 0u;
        iecf_ptr_t p = iecf_ptr_in(bytes, sizeof bytes, &bytes[2]);
        iecf_ptr_t q = iecf_ptr_add(p, 3), n = iecf_ptr_null();
        IEC_DWORD d;
        CHECK(iecf_pget_BYTE(&pf, &pim, p) == 3u && iecf_pget_BYTE(&pf, &pim, q) == 6u && pf.code == IECF_FAULT_NONE);
        iecf_pset_BYTE(&pf, &pim, iecf_ptr_sub(q, 1), 40u);
        CHECK(bytes[4] == 40u && iecf_ptr_diff(&pf, &pg, q, p) == 3 && iecf_ptr_cmp(&pf, p, q) < 0);
        CHECK(iecf_ptr_eq(iecf_ptr_add(p, 3), q) && !iecf_ptr_eq(p, q) && iecf_ptr_eq(n, iecf_ptr_null()) && !iecf_ptr_eq(n, p));
        /* Four bytes as a DWORD, in the platform's order. */
        d = iecf_pget_DWORD(&pf, &pim, iecf_ptr_in(bytes, sizeof bytes, &bytes[0]));
        CHECK(pf.code == IECF_FAULT_NONE && (d == 0x04030201u || d == 0x01020304u));
        /* Past the end, before the start, NULL: faults, nothing written. */
        CHECK(iecf_pget_DWORD(&pf, &pim, q) == 0u && pf.code == IECF_FAULT_INDEX);
        pf.code = IECF_FAULT_NONE;
        iecf_pset_BYTE(&pf, &pim, iecf_ptr_sub(p, 3), 9u);
        CHECK(pf.code == IECF_FAULT_INDEX && bytes[0] == 1u);
        pf.code = IECF_FAULT_NONE;
        CHECK(iecf_pget_BYTE(&pf, &pim, n) == 0u && pf.code == IECF_FAULT_NULL);
        pf.code = IECF_FAULT_NONE;
        /* Two variables: compared or subtracted, a fault. */
        CHECK(iecf_ptr_cmp(&pf, p, iecf_ptr_in(&r, sizeof r, &r)) == 0 && pf.code == IECF_FAULT_POINTER);
        pf.code = IECF_FAULT_NONE;
        /* A REAL's bits through a DWORD pointer, and a BOOL's byte. */
        CHECK(iecf_pget_DWORD(&pf, &pim, iecf_ptr_in(&r, sizeof r, &r)) == 0x3F800000u);
        iecf_pset_BYTE(&pf, &pim, iecf_ptr_in(&b, sizeof b, &b), 7u);
        CHECK(iecf_pget_BOOL(&pf, &pim, iecf_ptr_in(&b, sizeof b, &b)) == 1u && pf.code == IECF_FAULT_NONE);
        CHECK(pg.code == IECF_FAULT_NONE);
        /* An image area: found through the image at each use. */
        pim.i = NULL;
        pim.q = pq;
        pim.m = NULL;
        pim.i_size = 0u;
        pim.q_size = 4u;
        pim.m_size = 0u;
        iecf_pset_BYTE(&pf, &pim, iecf_ptr_image((uint8_t)'Q', 2u), 0x5Au);
        CHECK(pq[2] == 0x5Au && pf.code == IECF_FAULT_NONE);
        CHECK(iecf_pget_BYTE(&pf, &pim, iecf_ptr_image((uint8_t)'Q', 4u)) == 0u && pf.code == IECF_FAULT_INDEX);
        pf.code = IECF_FAULT_NONE;
        CHECK(!iecf_ptr_eq(iecf_ptr_image((uint8_t)'Q', 0u), iecf_ptr_image((uint8_t)'I', 0u)));
        /* A whole value, read and written; out of its variable nothing. */
        iecf_ptr_read(&pf, &pim, iecf_ptr_in(bytes, sizeof bytes, &bytes[4]), whole, sizeof whole);
        CHECK(whole[0] == 40u && whole[3] == 8u && pf.code == IECF_FAULT_NONE);
        iecf_ptr_write(&pf, &pim, iecf_ptr_in(bytes, sizeof bytes, &bytes[6]), whole, sizeof whole);
        CHECK(pf.code == IECF_FAULT_INDEX && bytes[6] == 7u);
        pf.code = IECF_FAULT_NONE;
    }
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
