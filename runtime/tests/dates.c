/* iecf.h's DATE, TOD and DT helpers, and LDATE, LTOD and LDT's: Ed.3 Table 30's arithmetic,
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
    char d[40];
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
        CHECK(iecf_concat_DATE(&g, y, m, dd) == day * 86400u && take() == 0u);
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

    /* CONCAT_DATE, CONCAT_TOD, CONCAT_DT and the SPLITs (Table 36). */
    CHECK(iecf_concat_DATE(&g, 2026, 11, 7) == D2026_11_07 && take() == 0u);
    CHECK(iecf_concat_DATE(&g, 2024, 2, 29) == 1709164800u && iecf_concat_DATE(&g, 1970, 1, 1) == 0u && take() == 0u);
    CHECK(iecf_concat_TOD(&g, 12, 34, 56, 789) == 45296789u && iecf_concat_TOD(&g, 23, 59, 59, 999) == 86399999u &&
          take() == 0u);
    CHECK(iecf_concat_DT(&g, 2026, 11, 7, 12, 34, 56, 789) == D2026_11_07 + 45296u && take() == 0u);
    CHECK(iecf_concat_DT(&g, 2106, 2, 7, 6, 28, 15, 0) == UINT32_MAX && take() == 0u);
    /* A part out of its range, or outside the type: the default, flagged. */
    CHECK(iecf_concat_DATE(&g, 2026, 13, 1) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DATE(&g, 2023, 2, 29) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DATE(&g, 2026, 4, 31) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DATE(&g, 2026, 1, 0) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DATE(&g, 1969, 12, 31) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DATE(&g, 2106, 2, 8) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DATE(&g, INT64_MAX, 1, 1) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DATE(&g, iecf_part(UINT64_MAX), 1, 1) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_TOD(&g, 24, 0, 0, 0) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_TOD(&g, 12, 60, 0, 0) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_TOD(&g, 12, 0, -1, 0) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_TOD(&g, 12, 0, 0, 1000) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DT(&g, 2106, 2, 7, 6, 28, 16, 0) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_concat_DT(&g, 2026, 11, 7, 12, 0, 0, 1000) == 0u && take() == IECF_FAULT_CONVERSION);
    {
        uint32_t p[7];
        iecf_split_DATE(D2026_11_07 + 45296u, &p[0], &p[1], &p[2]);
        CHECK(p[0] == 2026u && p[1] == 11u && p[2] == 7u);
        iecf_split_TOD(45296789u, &p[0], &p[1], &p[2], &p[3]);
        CHECK(p[0] == 12u && p[1] == 34u && p[2] == 56u && p[3] == 789u);
        iecf_split_TOD(UINT32_MAX, &p[0], &p[1], &p[2], &p[3]);
        CHECK(p[0] == 1193u && p[1] == 2u && p[2] == 47u && p[3] == 295u);
        iecf_split_DT(UINT32_MAX, &p[0], &p[1], &p[2], &p[3], &p[4], &p[5], &p[6]);
        CHECK(p[0] == 2106u && p[1] == 2u && p[2] == 7u && p[3] == 6u && p[4] == 28u && p[5] == 15u && p[6] == 0u);
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

    /* LDATE, LTOD, LDT: signed nanoseconds, 1677-09-21-00:12:43.145224192
     * to 2262-04-11-23:47:16.854775807. */
    {
        const int64_t S = INT64_C(1000000000), LD2026 = (int64_t)D2026_11_07 * S;
        uint32_t ly, lm, ld, lh, lmi, ls, lms;
        /* An LTOD wraps at midnight, unflagged. */
        CHECK(iecf_add_LTOD(23 * 3600 * S, 2 * 3600 * S) == 3600 * S);
        CHECK(iecf_sub_LTOD(3600 * S, 2 * 3600 * S) == 23 * 3600 * S);
        CHECK(iecf_add_LTOD(1, INT64_MIN) >= 0 && iecf_add_LTOD(1, INT64_MIN) < IECF_DAY_NS);
        CHECK(iecf_sub_LTOD(0, INT64_MIN) >= 0 && iecf_sub_LTOD(0, INT64_MAX) < IECF_DAY_NS);
        /* Widening: exact. */
        CHECK(iecf_time_ltime(-1500) == -1500 * INT64_C(1000000) && iecf_time_ltime(INT32_MIN) < 0);
        CHECK(iecf_dt_ldt(UINT32_MAX) == (int64_t)UINT32_MAX * S && iecf_tod_ltod(86399999u) == IECF_DAY_NS - 1000000);
        CHECK(iecf_dt_ldate(D2026_11_07 + 45296u) == LD2026 && iecf_dt_ltod(D2026_11_07 + 45296u) == 45296 * S);
        /* Narrowing: rounding down, unflagged; out of range the default, flagged. */
        CHECK(iecf_ltime_time(&g, -1500000) == -2 && iecf_ltime_time(&g, 1999999) == 1 && take() == 0u);
        CHECK(iecf_ltime_time(&g, (int64_t)INT32_MAX * 1000000 + 999999) == INT32_MAX && take() == 0u);
        CHECK(iecf_ltime_time(&g, ((int64_t)INT32_MAX + 1) * 1000000) == 0 && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_ltime_time(&g, (int64_t)INT32_MIN * 1000000 - 1) == 0 && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_ldt_dt(&g, LD2026 + S - 1) == D2026_11_07 && take() == 0u);
        CHECK(iecf_ldt_dt(&g, ((int64_t)UINT32_MAX + 1) * S) == 0u && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_ldt_dt(&g, -1) == 0u && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_ldt_date(&g, LD2026 + 45296 * S) == D2026_11_07 && take() == 0u);
        CHECK(iecf_ldt_date(&g, -1) == 0u && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_ldt_ldate(&g, -1) == -IECF_DAY_NS && take() == 0u);
        CHECK(iecf_ldt_ldate(&g, INT64_MAX) == INT64_C(106751) * IECF_DAY_NS && take() == 0u);
        CHECK(iecf_ldt_ldate(&g, INT64_MIN) == 0 && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_ldt_ltod(-1) == IECF_DAY_NS - 1 && iecf_ldt_tod(-1) == 86399999u);
        CHECK(iecf_ldt_tod(LD2026 + 45296 * S + 999999) == 45296000u);
        /* CONCAT and SPLIT. */
        CHECK(iecf_concat_date_ltod(D2026_11_07, 45296 * S + 1) == LD2026 + 45296 * S + 1);
        CHECK(iecf_concat_LTOD(&g, 12, 34, 56, 789) == 45296789 * INT64_C(1000000) && take() == 0u);
        CHECK(iecf_concat_LTOD(&g, 24, 0, 0, 0) == 0 && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_concat_LDT(&g, 2026, 11, 7, 0, 0, 0, 0) == LD2026 && take() == 0u);
        CHECK(iecf_concat_LDT(&g, 2262, 4, 11, 23, 47, 16, 854) == INT64_MAX - 775807 && take() == 0u);
        CHECK(iecf_concat_LDT(&g, 2262, 4, 11, 23, 47, 16, 855) == 0 && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_concat_LDT(&g, 1677, 9, 21, 0, 12, 43, 146) == INT64_MIN + 775808 &&
              take() == 0u);
        CHECK(iecf_concat_LDT(&g, 1677, 9, 21, 0, 12, 43, 145) == 0 && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_concat_LDT(&g, 1969, 12, 31, 23, 59, 59, 999) == -1000000 && take() == 0u);
        CHECK(iecf_concat_LDT(&g, 2023, 2, 29, 0, 0, 0, 0) == 0 && take() == IECF_FAULT_CONVERSION);
        iecf_split_LDT(-1, &ly, &lm, &ld, &lh, &lmi, &ls, &lms);
        CHECK(ly == 1969u && lm == 12u && ld == 31u && lh == 23u && lmi == 59u && ls == 59u && lms == 999u);
        iecf_split_LDT(INT64_MIN, &ly, &lm, &ld, &lh, &lmi, &ls, &lms);
        CHECK(ly == 1677u && lm == 9u && ld == 21u && lh == 0u && lmi == 12u && ls == 43u && lms == 145u);
        iecf_split_LTOD(45296 * S + 999999, &lh, &lmi, &ls, &lms);
        CHECK(lh == 12u && lmi == 34u && ls == 56u && lms == 0u);
        /* The text both ways. */
        CHECK(is(iecf_ldate_str(d, LD2026), "LD#2026-11-07"));
        CHECK(is(iecf_ldate_str(d, -IECF_DAY_NS), "LD#1969-12-31"));
        CHECK(is(iecf_ltod_str(d, 45296 * S + 500000000), "LTOD#12:34:56.500"));
        CHECK(is(iecf_ltod_str(d, 45296 * S + 1000), "LTOD#12:34:56.000001"));
        CHECK(is(iecf_ltod_str(d, 45296 * S), "LTOD#12:34:56"));
        CHECK(is(iecf_ldt_str(d, INT64_MAX), "LDT#2262-04-11-23:47:16.854775807"));
        CHECK(is(iecf_ldt_str(d, INT64_MIN), "LDT#1677-09-21-00:12:43.145224192"));
        CHECK(iecf_str_ldt(&g, "LDT#2262-04-11-23:47:16.854775807") == INT64_MAX && take() == 0u);
        CHECK(iecf_str_ldt(&g, "LDT#1677-09-21-00:12:43.145224192") == INT64_MIN && take() == 0u);
        CHECK(iecf_str_ldt(&g, "LDT#2262-04-11-23:47:16.854775808") == 0 && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_str_ldt(&g, "LDT#1969-12-31-23:59:59.999999999") == -1 && take() == 0u);
        CHECK(iecf_str_ldate(&g, "LD#1677-09-22") == INT64_C(-106751) * IECF_DAY_NS && take() == 0u);
        CHECK(iecf_str_ldate(&g, "LDATE#1677-09-21") == 0 && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_str_ltod(&g, "LTOD#23:59:59.999999999") == IECF_DAY_NS - 1 && take() == 0u);
        CHECK(iecf_str_ltod(&g, "LTOD#24:00:00") == 0 && take() == IECF_FAULT_CONVERSION);
    }

    if (failures != 0) return 1;
    return 0;
}
