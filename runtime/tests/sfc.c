/* iecf.h's SFC helpers: a step's flag and elapsed time, an "unsafe"
 * entry flagged, and ACTION_CONTROL with final scan (Ed.3 Figure 23 a)
 * for each qualifier, scan by scan; harness/cases/sfc covers them inside
 * a chart. A scan is 10 ms here.
 * cc -std=c99 -pedantic -Wall -Wextra -Werror -I runtime runtime/tests/sfc.c && ./a.out */
#include "iecf.h"

static int failures;
#define CHECK(cond) \
    do { \
        if (!(cond)) failures++; \
    } while (0)

static iecf_fault_t g;
static uint16_t take(void) {
    uint16_t c = g.code;
    g.code = IECF_FAULT_NONE;
    return c;
}
#define MS INT64_C(1000000)

/* One action driven by `q` for scans 0..n-1 (`q_at`), its Q and A after
 * each into `out` as "QA" pairs. */
static void run(unsigned (*q_at)(int), IEC_TIME t, int n, char *out) {
    iecf_action_t a;
    int k;
    iecf_action_init(&a);
    for (k = 0; k < n; k++) {
        iecf_action(&g, &a, q_at(k), t, (int64_t)k * 10 * MS);
        out[2 * k] = (char)('0' + a.Q);
        out[2 * k + 1] = (char)('0' + a.A);
    }
    out[2 * n] = 0;
}
static int is(const char *a, const char *b) { return iecf_scmp(a, b) == 0; }

/* The association active on scans 1..3 (a step active three scans). */
static unsigned n_(int k) { return k >= 1 && k <= 3 ? IECF_Q_N : 0u; }
static unsigned p_(int k) { return k >= 1 && k <= 3 ? IECF_Q_P : 0u; }
static unsigned p1_(int k) { return k >= 1 && k <= 3 ? IECF_Q_P1 : 0u; }
static unsigned p0_(int k) { return k >= 1 && k <= 3 ? IECF_Q_P0 : 0u; }
static unsigned l_(int k) { return k >= 1 && k <= 3 ? IECF_Q_L : 0u; }
static unsigned d_(int k) { return k >= 1 && k <= 3 ? IECF_Q_D : 0u; }
static unsigned s_(int k) { return k >= 1 && k <= 3 ? IECF_Q_S : (k == 6 ? IECF_Q_R : 0u); }
static unsigned sd_(int k) { return k == 1 ? IECF_Q_SD : (k == 7 ? IECF_Q_R : 0u); }
static unsigned ds_(int k) { return k >= 1 && k <= 3 ? IECF_Q_DS : (k == 7 ? IECF_Q_R : 0u); }
static unsigned dsshort_(int k) { return k == 1 ? IECF_Q_DS : 0u; }
static unsigned sl_(int k) { return k == 1 ? IECF_Q_SL : 0u; }
static unsigned rn_(int k) { return k >= 1 && k <= 3 ? (IECF_Q_N | (k == 2 ? IECF_Q_R : 0u)) : 0u; }

int main(void) {
    char o[64];
    iecf_step_t s;

    /* A step: T from the first scan it is seen active, kept when left. */
    iecf_step_init(&s, 1);
    iecf_step_time(&s, 50 * MS);
    CHECK(s.X == 1 && s.T == 0);
    iecf_step_time(&s, 80 * MS);
    CHECK(s.T == 30);
    iecf_step_leave(&s);
    iecf_step_time(&s, 200 * MS);
    CHECK(s.X == 0 && s.T == 30);
    iecf_step_enter(&g, &s, 200 * MS);
    CHECK(s.X == 1 && s.T == 0 && take() == 0u);
    iecf_step_time(&s, 210 * MS);
    CHECK(s.T == 10);
    iecf_step_enter(&g, &s, 210 * MS); /* entered while active: unsafe */
    CHECK(s.X == 1 && s.T == 10 && take() == IECF_FAULT_SFC);
    iecf_step_time(&s, INT64_C(9000000000000000000));
    CHECK(s.T == INT32_MAX); /* stops at TIME's largest */

    /* QA per scan 0..7: N runs while active, then once more (final scan). */
    run(n_, 0, 6, o);
    CHECK(is(o, "001111110100"));
    /* P: one scan with Q, then its final scan: twice, as CODESYS. */
    run(p_, 0, 6, o);
    CHECK(is(o, "001101000000"));
    /* P1: once, on entry, Q FALSE; P0: once, on leaving. */
    run(p1_, 0, 6, o);
    CHECK(is(o, "000100000000"));
    run(p0_, 0, 6, o);
    CHECK(is(o, "000000000100"));
    /* L 20 ms: Q for two scans, then the final scan. */
    run(l_, 20, 6, o);
    CHECK(is(o, "001111010000"));
    /* D 20 ms: Q from the third scan, while the step still is. */
    run(d_, 20, 6, o);
    CHECK(is(o, "000000110100"));
    /* S stays after the step; R (scan 6) resets it, with a final scan. */
    run(s_, 0, 8, o);
    CHECK(is(o, "0011111111110100"));
    /* SD 20 ms: stored at once, Q 20 ms later, until R. */
    run(sd_, 20, 9, o);
    CHECK(is(o, "000000111111110100"));
    /* DS 20 ms: Q if still active after 20 ms, then stored until R. */
    run(ds_, 20, 9, o);
    CHECK(is(o, "000000111111110100"));
    run(dsshort_, 20, 5, o); /* left before the delay: never */
    CHECK(is(o, "0000000000"));
    /* SL 20 ms: stored, limited to 20 ms. */
    run(sl_, 20, 6, o);
    CHECK(is(o, "001111010000"));
    /* R overrides N while both are active. */
    run(rn_, 0, 6, o);
    CHECK(is(o, "001101110100"));
    CHECK(take() == 0u);

    /* SD while its SL_FF is set: Ed.3 6.7.4.6 d, flagged. */
    {
        iecf_action_t a;
        iecf_action_init(&a);
        iecf_action(&g, &a, IECF_Q_SL, 50, 0);
        CHECK(take() == 0u);
        iecf_action(&g, &a, IECF_Q_SD, 50, 10 * MS);
        CHECK(take() == IECF_FAULT_SFC);
    }

    if (failures != 0) return 1;
    return 0;
}
