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

    return failures != 0;
}
