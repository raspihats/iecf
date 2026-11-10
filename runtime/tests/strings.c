/* iecf.h's STRING helpers against Ed.3 Table 34's examples and each edge
 * docs/dependencies.md decides (Siemens' results, flagged), with no
 * undefined behaviour and no read past a string's 0 (CI runs this under
 * the sanitizers and on 32-bit wasm).
 * cc -std=c99 -pedantic -Wall -Wextra -Werror -I runtime runtime/tests/strings.c && ./a.out */
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
static int wis(const IEC_WCHAR *a, const IEC_WCHAR *b) { return iecf_wscmp(a, b) == 0; }

int main(void) {
    char d[32], v[5];
    /* Table 34's examples. */
    CHECK(iecf_slen("ASTRING") == 7u && iecf_slen("") == 0u);
    CHECK(is(iecf_left(&g, d, 31u, "ASTR", 3), "AST") && take() == 0u);
    CHECK(is(iecf_right(&g, d, 31u, "ASTR", 3), "STR") && take() == 0u);
    CHECK(is(iecf_mid(&g, d, 31u, "ASTR", 2, 2), "ST") && take() == 0u);
    CHECK(is(iecf_concat(&g, d, 31u, "AB", "CD"), "ABCD") && take() == 0u);
    CHECK(is(iecf_insert(&g, d, 31u, "ABC", "XY", 2), "ABXYC") && take() == 0u);
    CHECK(is(iecf_delete(&g, d, 31u, "ABXYC", 2, 3), "ABC") && take() == 0u);
    CHECK(is(iecf_replace(&g, d, 31u, "ABCDE", "X", 2, 3), "ABXE") && take() == 0u);
    CHECK(iecf_find("ABCBC", "BC") == 2u && iecf_find("ABC", "X") == 0u && iecf_find("AB", "ABC") == 0u);
    CHECK(iecf_find("ABC", "") == 0u && iecf_find("", "A") == 0u);

    /* Comparison (Ed.3 6.6.2.5.11): 'Z' > 'AZ' > 'ABC'; codes unsigned. */
    CHECK(iecf_scmp("Z", "AZ") > 0 && iecf_scmp("AZ", "ABC") > 0 && iecf_scmp("AB", "ABC") < 0);
    CHECK(iecf_scmp("\351", "z") > 0 && iecf_scmp("", "") == 0);

    /* Too long: cut to fit and flagged. */
    CHECK(is(iecf_scopy(&g, v, 4u, "ABCDEF"), "ABCD") && take() == IECF_FAULT_STRING_LENGTH);
    CHECK(is(iecf_scopy(&g, v, 4u, "ABCD"), "ABCD") && take() == 0u);
    CHECK(is(iecf_scopy(&g, v, 4u, v), "ABCD") && take() == 0u);
    CHECK(is(iecf_concat(&g, v, 4u, "ABC", "DE"), "ABCD") && take() == IECF_FAULT_STRING_LENGTH);
    iecf_sinit(v, 4u, "AB");
    CHECK(is(v, "AB") && v[3] == 0 && v[4] == 0);

    /* LEFT, RIGHT: L past the end all of IN, L < 0 empty; both flagged. */
    CHECK(is(iecf_left(&g, d, 31u, "AB", 5), "AB") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_right(&g, d, 31u, "AB", 5), "AB") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_left(&g, d, 31u, "AB", -1), "") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_left(&g, d, 31u, "AB", 0), "") && take() == 0u);
    CHECK(is(iecf_right(&g, d, 31u, "AB", INT64_MAX), "AB") && take() == IECF_FAULT_STRING_POSITION);

    /* MID: to the end if L runs past it; P < 1 or past the end empty. */
    CHECK(is(iecf_mid(&g, d, 31u, "ABC", 5, 2), "BC") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_mid(&g, d, 31u, "ABC", 1, 0), "") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_mid(&g, d, 31u, "ABC", 1, 4), "") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_mid(&g, d, 31u, "ABC", 0, 2), "") && take() == 0u);

    /* DELETE: P past the end leaves IN; L past it deletes to the end. */
    CHECK(is(iecf_delete(&g, d, 31u, "ABC", 1, 4), "ABC") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_delete(&g, d, 31u, "ABC", 0, 2), "ABC") && take() == 0u);
    CHECK(is(iecf_delete(&g, d, 31u, "ABC", 9, 2), "A") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_delete(&g, d, 31u, "ABC", 1, 0), "") && take() == IECF_FAULT_STRING_POSITION);

    /* INSERT: P = 0 at the front; past the end appends; P < 0 empty. */
    CHECK(is(iecf_insert(&g, d, 31u, "ABC", "X", 0), "XABC") && take() == 0u);
    CHECK(is(iecf_insert(&g, d, 31u, "ABC", "X", 3), "ABCX") && take() == 0u);
    CHECK(is(iecf_insert(&g, d, 31u, "ABC", "X", 7), "ABCX") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_insert(&g, d, 31u, "ABC", "X", -1), "") && take() == IECF_FAULT_STRING_POSITION);

    /* REPLACE: L past the end replaces to the end; P past it appends. */
    CHECK(is(iecf_replace(&g, d, 31u, "ABC", "XY", 5, 2), "AXY") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_replace(&g, d, 31u, "ABC", "XY", 1, 9), "ABCXY") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_replace(&g, d, 31u, "ABC", "XY", -1, 1), "") && take() == IECF_FAULT_STRING_POSITION);
    CHECK(is(iecf_replace(&g, d, 31u, "ABC", "XY", 0, 2), "AXYBC") && take() == 0u);

    /* The buffer is never overrun: results cut at n and flagged. */
    CHECK(is(iecf_insert(&g, v, 4u, "ABC", "XY", 1), "AXYB") && take() == IECF_FAULT_STRING_LENGTH);
    CHECK(is(iecf_replace(&g, v, 4u, "ABC", "XYZ", 1, 2), "AXYZ") && take() == IECF_FAULT_STRING_LENGTH);

    /* To STRING (docs/dependencies.md): integers and bit strings in
     * decimal, BOOL as TRUE/FALSE, reals as %g with 7 or 15 digits, the
     * durations as literals. */
    CHECK(is(iecf_s_str(d, -123456), "-123456") && is(iecf_s_str(d, INT64_MIN), "-9223372036854775808"));
    CHECK(is(iecf_u_str(d, 255u), "255") && is(iecf_u_str(d, UINT64_MAX), "18446744073709551615"));
    CHECK(is(iecf_bool_str(d, 1u), "TRUE") && is(iecf_bool_str(d, 0u), "FALSE"));
    CHECK(is(iecf_real_str(d, (double)2.5f, 7u), "2.5") && is(iecf_real_str(d, (double)0.1f, 7u), "0.1"));
    CHECK(is(iecf_real_str(d, (double)16777216.0f, 7u), "1.677722e+07") && is(iecf_real_str(d, 1.0, 7u), "1"));
    CHECK(is(iecf_real_str(d, 0.1, 15u), "0.1") && is(iecf_real_str(d, 0.123456789012, 15u), "0.123456789012"));
    CHECK(is(iecf_real_str(d, 0.00001, 15u), "1e-05") && is(iecf_real_str(d, -0.0, 7u), "-0"));
    CHECK(is(iecf_real_str(d, 1e300 * 1e300, 7u), "inf") && is(iecf_real_str(d, -1e300 * 1e300, 7u), "-inf"));
    CHECK(is(iecf_real_str(d, 1.7976931348623157e308, 15u), "1.79769313486232e+308"));
    CHECK(is(iecf_time_str(d, 93784500, 0), "T#1d2h3m4s500ms") && is(iecf_time_str(d, 0, 0), "T#0ms"));
    CHECK(is(iecf_time_str(d, -1500, 0), "T#-1s500ms") && is(iecf_time_str(d, INT32_MIN, 0), "T#-24d20h31m23s648ms"));
    CHECK(is(iecf_time_str(d, 1000005000, 1), "LTIME#1s5us") && is(iecf_time_str(d, 0, 1), "LTIME#0ns"));
    {
        char w[48];
        CHECK(is(iecf_time_str(w, INT64_MIN, 1), "LTIME#-106751d23h47m16s854ms775us808ns"));
    }

    /* From STRING: spaces, a type prefix, a sign, based literals and
     * underscores read; what follows the literal ignored; no literal, or
     * out of range, 0 and flagged IECF_FAULT_CONVERSION. */
    CHECK(iecf_str_s(&g, " 123abc", -32768, 32767) == 123 && take() == 0u);
    CHECK(iecf_str_s(&g, "INT#-32768", -32768, 32767) == -32768 && take() == 0u);
    CHECK(iecf_str_s(&g, "-16#7F", -128, 127) == -127 && take() == 0u);
    CHECK(iecf_str_u(&g, "2#1111_1111", 255u) == 255u && take() == 0u);
    CHECK(iecf_str_u(&g, "16#FFFFFFFFFFFFFFFF", UINT64_MAX) == UINT64_MAX && take() == 0u);
    CHECK(iecf_str_s(&g, "abc", -100, 100) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_s(&g, "", -100, 100) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_s(&g, "128", -128, 127) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_u(&g, "-1", 255u) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_u(&g, "99999999999999999999", UINT64_MAX) == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_bool(&g, "true") == 1u && iecf_str_bool(&g, " FALSE") == 0u && iecf_str_bool(&g, "1") == 1u);
    CHECK(take() == 0u && iecf_str_bool(&g, "yes") == 0u && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_real(&g, "2.5", 53u) == 2.5 && iecf_str_real(&g, " 1.5E+2xyz", 53u) == 150.0);
    CHECK(iecf_str_real(&g, "REAL#-1_000.25", 24u) == -1000.25 && iecf_str_real(&g, "+.5", 53u) == 0.5);
    CHECK(iecf_str_real(&g, "0.1", 53u) == 0.1 && (float)iecf_str_real(&g, "0.1", 24u) == 0.1f && take() == 0u);
    CHECK(iecf_str_real(&g, "1e23", 53u) == 1e23 && iecf_str_real(&g, "4.9406564584124654e-324", 53u) == 5e-324);
    CHECK(iecf_str_real(&g, "3.4028235e38", 24u) == (double)3.4028235e38f && take() == 0u);
    CHECK(iecf_str_real(&g, "3.5e38", 24u) == 0.0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_real(&g, "1e400", 53u) == 0.0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_real(&g, "1e-400", 53u) == 0.0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_real(&g, "x1", 53u) == 0.0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_time(&g, "T#1d2h3m4s500ms", 0) == 93784500 && iecf_str_time(&g, "t#1.5s", 0) == 1500);
    CHECK(iecf_str_time(&g, "LTIME#1ms2us3ns", 1) == 1002003 && iecf_str_time(&g, "T#-24d20h31m23s648ms", 0) == INT32_MIN);
    CHECK(take() == 0u && iecf_str_time(&g, "T#24d20h31m23s648ms", 0) == 0 && take() == IECF_FAULT_CONVERSION);
    CHECK(iecf_str_time(&g, "1s", 0) == 0 && take() == IECF_FAULT_CONVERSION);

    /* SEL, MIN, MAX, LIMIT, MUX: by the characters' codes, as compared. */
    {
        static const char *const ins[] = {"a", "b", "c"};
        CHECK(is(iecf_ssel(0u, "x", "y"), "x") && is(iecf_ssel(1u, "x", "y"), "y"));
        CHECK(is(iecf_smax("ab", "abc"), "abc") && is(iecf_smin("Z", "AZ"), "AZ") && is(iecf_smax("a", "a"), "a"));
        CHECK(is(iecf_slimit("b", "a", "d"), "b") && is(iecf_slimit("b", "z", "d"), "d") && is(iecf_slimit("b", "c", "d"), "c"));
        CHECK(is(iecf_smux(&g, 2, 3u, ins), "c") && take() == 0u);
        CHECK(is(iecf_smux(&g, 3, 3u, ins), "a") && take() == IECF_FAULT_MUX);
    }

    /* WSTRING: the same functions on UTF-16 units. */
    {
        static const IEC_WCHAR astr[] = {'A', 'S', 'T', 'R', 0}, st[] = {'S', 'T', 0}, as[] = {'A', 'S', 0}, ab[] = {'A', 'B', 0};
        static const IEC_WCHAR smile[] = {'a', 0x263Au, 0xD83Du, 0xDE00u, 0xE9u, 0}, empty[] = {0};
        static const IEC_WCHAR big[] = {0x263Au, 0}, small[] = {'z', 0};
        IEC_WCHAR wd[16], wv[3];
        char nd[16];
        CHECK(iecf_wslen(astr) == 4u && iecf_wslen(smile) == 5u && iecf_wslen(empty) == 0u);
        CHECK(wis(iecf_wmid(&g, wd, 15u, astr, 2, 2), st) && take() == 0u);
        CHECK(iecf_wfind(astr, st) == 2u && iecf_wfind(astr, empty) == 0u);
        CHECK(wis(iecf_wscopy(&g, wv, 2u, astr), as) && take() == IECF_FAULT_STRING_LENGTH);
        CHECK(iecf_wscmp(big, small) > 0 && iecf_wscmp(empty, empty) == 0);
        CHECK(iecf_wschar(&g, smile, 2) == 0x263Au && take() == 0u);
        CHECK(iecf_wschar(&g, smile, 6) == 0u && take() == IECF_FAULT_STRING_POSITION);
        /* To STRING: Latin-1 kept, the rest skipped (a surrogate pair whole), flagged. */
        CHECK(is(iecf_wstr_str(&g, nd, 5u, smile), "a\351") && take() == IECF_FAULT_CONVERSION);
        CHECK(is(iecf_wstr_str(&g, nd, 5u, astr), "ASTR") && take() == 0u);
        CHECK(wis(iecf_str_wstr(wd, 15u, "AB"), ab) && iecf_str_wstr(wd, 15u, "\351")[0] == 0xE9u);
        CHECK(iecf_wchar_char(&g, 0xE9u) == 0xE9u && take() == 0u);
        CHECK(iecf_wchar_char(&g, 0x263Au) == 0u && take() == IECF_FAULT_CONVERSION);
        CHECK(iecf_wstring_char(&g, empty) == 0u && take() == IECF_FAULT_CONVERSION);
        CHECK(is(iecf_wstr_text(nd, 15u, smile), "a???\77"));
    }

    return failures != 0;
}
