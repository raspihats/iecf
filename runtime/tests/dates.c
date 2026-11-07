/* iecf.h's DATE, TOD and DT helpers: Ed.3 Table 30's arithmetic, what
 * docs/dependencies.md decides at the edges (a TOD wraps at midnight
 * unflagged; a DATE or DT, or a difference, out of range wraps and is
 * flagged; a DT drops milliseconds), the text both ways, and the calendar
 * against a day-by-day count (CI runs this under the sanitizers and on
 * 32-bit wasm).
 * cc -std=c99 -pedantic -Wall -Wextra -Werror -I runtime runtime/tests/dates.c && ./a.out */
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
static int is(const char *a, const char *b) { return iecf_scmp(a, b) == 0; }

#define H 3600000
#define D2026_11_07 1794009600u /* 2026-11-07 00:00:00 */

int main(void) {
    char d[32];
    uint32_t day, y = 1970u, m = 1u, dd = 1u;

    /* TOD wraps at midnight, unflagged. */
    CHECK(iecf_add_TOD(23u * H, 2 * H) == 1u * H && take() == 0u);
    CHECK(iecf_sub_TOD(1u * H, 2 * H) == 23u * H && take() == 0u);
    CHECK(iecf_add_TOD(0u, INT32_MIN) == iecf_tod_ms((int64_t)INT32_MIN) && iecf_add_TOD(0u, INT32_MIN) < 86400000u);
    CHECK(iecf_sub_TOD(5u, INT32_MIN) < 86400000u && take() == 0u);
    CHECK(iecf_sub_TOD_TOD(&g, 1u * H, 23u * H) == -22 * H && take() == 0u);

    /* DT: whole seconds, milliseconds dropped rounding down. */
    CHECK(iecf_add_DT(&g, 100u, 1500) == 101u && iecf_add_DT(&g, 100u, -1500) == 98u && take() == 0u);
    CHECK(iecf_sub_DT(&g, 100u, 1500) == 98u && iecf_sub_DT(&g, 100u, -1500) == 101u && take() == 0u);
    CHECK(iecf_sub_DT(&g, 100u, INT32_MIN) == 100u + 2147483u && take() == 0u);
    /* Out of range: wrapped and flagged. */
    CHECK(iecf_sub_DT(&g, 0u, 1000) == UINT32_MAX && take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_add_DT(&g, UINT32_MAX, 1000) == 0u && take() == IECF_FAULT_OVERFLOW);
    /* A difference: in TIME's range, or wrapped and flagged. */
    CHECK(iecf_sub_DT_DT(&g, 1000u, 10u) == 990000 && take() == 0u);
    CHECK(iecf_sub_DT_DT(&g, D2026_11_07 + 86400u * 24u, D2026_11_07) == 24 * 24 * H && take() == 0u);
    CHECK(iecf_sub_DT_DT(&g, D2026_11_07 + 86400u * 25u, D2026_11_07) == iecf_to_i32(25u * 24u * H) &&
          take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_sub_DT_DT(&g, 0u, UINT32_MAX) == iecf_to_i32((uint32_t)(uint64_t)(-(int64_t)UINT32_MAX * 1000)) &&
          take() == IECF_FAULT_OVERFLOW);

    /* The conversions and DAY_OF_WEEK (2026-11-07 is a Saturday). */
    CHECK(iecf_dt_tod(D2026_11_07 + 45296u) == 45296000u && iecf_dt_date(D2026_11_07 + 45296u) == D2026_11_07);
    CHECK(iecf_concat_date_tod(&g, D2026_11_07, 45296789u) == D2026_11_07 + 45296u && take() == 0u);
    CHECK(iecf_concat_date_tod(&g, 4294915200u, 86399999u) == (uint32_t)(4294915200u + 86399u) &&
          take() == IECF_FAULT_OVERFLOW);
    CHECK(iecf_day_of_week(D2026_11_07) == 6u && iecf_day_of_week(0u) == 4u && iecf_day_of_week(D2026_11_07 + 86400u) == 0u);

    /* The calendar: every day from 1970 to 2106, against a count. */
    for (day = 0u; day <= 49710u; day++) {
        uint32_t cy, cm, cd, len;
        iecf_civil(day, &cy, &cm, &cd);
        CHECK(cy == y && cm == m && cd == dd && iecf_days((int64_t)y, m, dd) == (int64_t)day);
        len = m == 2u ? ((y % 4u == 0u && y % 100u != 0u) || y % 400u == 0u ? 29u : 28u)
            : (m == 4u || m == 6u || m == 9u || m == 11u) ? 30u : 31u;
        if (++dd > len) {
            dd = 1u;
            if (++m > 12u) {
                m = 1u;
                y++;
            }
        }
    }

    /* To STRING (docs/dependencies.md). */
    CHECK(is(iecf_date_str(d, D2026_11_07), "D#2026-11-07") && is(iecf_date_str(d, 0u), "D#1970-01-01"));
    CHECK(is(iecf_tod_str(d, 45296000u), "TOD#12:34:56") && is(iecf_tod_str(d, 45296007u), "TOD#12:34:56.007"));
    CHECK(is(iecf_tod_str(d, 0u), "TOD#00:00:00") && is(iecf_tod_str(d, UINT32_MAX), "TOD#1193:02:47.295"));
    CHECK(is(iecf_dt_str(d, D2026_11_07 + 45296u), "DT#2026-11-07-12:34:56"));
    CHECK(is(iecf_dt_str(d, UINT32_MAX), "DT#2106-02-07-06:28:15"));

    /* From STRING: the literal, its prefix any case; the rest ignored. */
    CHECK(iecf_str_date(&g, "D#2026-11-07") == D2026_11_07 && iecf_str_date(&g, " date#2026-11-07x") == D2026_11_07);
    CHECK(iecf_str_tod(&g, "TOD#12:34:56.0079") == 45296007u && iecf_str_tod(&g, "tod#1:2:3") == 3723000u);
    CHECK(iecf_str_tod(&g, "TIME_OF_DAY#12:34:56.5") == 45296500u && take() == 0u);
    CHECK(iecf_str_dt(&g, "DT#2026-11-07-12:34:56.999") == D2026_11_07 + 45296u && take() == 0u);
    CHECK(iecf_str_dt(&g, "DT#2106-02-07-06:28:15") == UINT32_MAX && take() == 0u);
    CHECK(iecf_str_date(&g, "D#2024-02-29") == 1709164800u && take() == 0u);
    /* None, no such day or time, or outside the type: 0, flagged. */
    CHECK(iecf_str_date(&g, "2026-11-07") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_date(&g, "D#2023-02-29") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_date(&g, "D#1969-12-31") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_date(&g, "D#2106-02-08") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_tod(&g, "TOD#24:00:00") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_tod(&g, "TOD#12:60:00") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_tod(&g, "TOD#12:00") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_dt(&g, "DT#2106-02-07-06:28:16") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_dt(&g, "DT#2026-11-07") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_dt(&g, "DT#99999999999-01-01-00:00:00") == 0u && take() == IECF_FAULT_CONVERSION);

    if (failures != 0) return 1;
    return 0;
}
