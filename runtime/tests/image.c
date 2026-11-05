/* iecf.h's process image access, against values worked out by hand.
 * cc -std=c99 -pedantic -Wall -Wextra -Werror -I runtime runtime/tests/image.c && ./a.out */
#include "iecf.h"

static int failures;
#define CHECK(cond) \
    do { \
        if (!(cond)) failures++; \
    } while (0)

int main(void) {
    uint8_t m[16] = {0};

    /* %MX1.3 := TRUE sets bit 3 of byte 1, and %MB1 sees it. */
    iecf_set_x(m, 1, 3, 1);
    CHECK(m[1] == 0x08);
    CHECK(iecf_get_x(m, 1, 3) == 1 && iecf_get_x(m, 1, 2) == 0);
    iecf_set_x(m, 1, 3, 0);
    CHECK(m[1] == 0);

    /* %MW2 := 16#1234: low byte first; %MW3 overlaps it (bytewise). */
    iecf_set_w(m, 2, 0x1234u);
    CHECK(m[2] == 0x34 && m[3] == 0x12);
    CHECK(iecf_get_b(m, 3) == 0x12);
    CHECK(iecf_get_w(m, 3) == 0x0012u);

    /* %MD4, %ML8. */
    iecf_set_d(m, 4, 0xA1B2C3D4u);
    CHECK(m[4] == 0xD4 && m[7] == 0xA1);
    CHECK(iecf_get_d(m, 4) == 0xA1B2C3D4u);
    iecf_set_l(m, 8, 0x0102030405060708ull);
    CHECK(m[8] == 0x08 && m[15] == 0x01);
    CHECK(iecf_get_l(m, 8) == 0x0102030405060708ull);

    /* A REAL by its IEEE bits: 1.0 is 16#3F80_0000. */
    iecf_set_real(m, 4, 1.0f);
    CHECK(iecf_get_d(m, 4) == 0x3F800000u);
    CHECK(iecf_get_real(m, 4) == 1.0f);
    iecf_set_lreal(m, 8, -2.5);
    CHECK(iecf_get_l(m, 8) == 0xC004000000000000ull);
    CHECK(iecf_get_lreal(m, 8) == -2.5);

    /* An INT read as its two's complement value. */
    iecf_set_w(m, 0, 0xFFFEu);
    CHECK(iecf_to_i16(iecf_get_w(m, 0)) == -2);
    CHECK(iecf_to_i16(0x8000u) == INT16_MIN && iecf_to_i16(0x7FFFu) == INT16_MAX);
    CHECK(iecf_to_i32(0x80000000u) == INT32_MIN);
    CHECK(iecf_to_i64(0xFFFFFFFFFFFFFFFFull) == -1);

    return failures;
}
