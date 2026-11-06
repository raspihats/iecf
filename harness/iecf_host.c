/* iecf_host.c - runs iecf's C for the harness (harness/README.md).
 *
 * Generic: compiled with -DIECF_PROJECT=<name> and -include <name>.h,
 * the files `iecf build` wrote for one case. The platform side of
 * docs/c-output.md, as small as it can be: the process image, the
 * context, and a scheduler of the project's tasks.
 *
 * Reads the cycle script from stdin as matiec_host.c does and writes the
 * same trace: one line after initialisation and one per tick, the Q and M
 * bytes in hex. A tick is the greatest common divisor of the tasks'
 * intervals, as matiec's common_ticktime__; a task runs on the ticks its
 * interval divides, the tasks in order of priority. A fault adds
 * "fault CODE POU LINE" to the line, and no task runs after it.
 *
 * Harness code: plain C99 with the C library.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CAT_(a, b) a##b
#define CAT(a, b) CAT_(a, b)
#define P(x) CAT(IECF_PROJECT, x)

#define IMAGE_BYTES 4096
static uint8_t I[IMAGE_BYTES], Q[IMAGE_BYTES], M[IMAGE_BYTES];
static P(_t) ctx;

static void trace(const char *label, unsigned q, unsigned m) {
    unsigned k;
    printf("%s Q", label);
    for (k = 0; k < q; k++) printf(" %02x", Q[k]);
    printf(" M");
    for (k = 0; k < m; k++) printf(" %02x", M[k]);
    if (ctx.fault.code != IECF_FAULT_NONE)
        printf(" fault %u %s %lu", (unsigned)ctx.fault.code, P(_pou_names)[ctx.fault.pou], (unsigned long)ctx.fault.line);
    printf("\n");
}

static int64_t gcd(int64_t a, int64_t b) {
    while (b) {
        int64_t t = a % b;
        a = b;
        b = t;
    }
    return a;
}

int main(void) {
    unsigned long ticks = 0, tick, at, offset;
    char word[16], hex[2 * IMAGE_BYTES + 1];
    int have;
    uint32_t order[64], n = P(_task_count), k, j;
    int64_t base = 0;
    unsigned q = CAT(IECF_PROJECT_UP, _Q_BYTES), m = CAT(IECF_PROJECT_UP, _M_BYTES);

    if (scanf("%15s %lu", word, &ticks) != 2 || strcmp(word, "ticks") != 0) {
        fprintf(stderr, "script: expected 'ticks N'\n");
        return 2;
    }
    if (n > 64) {
        fprintf(stderr, "more than 64 tasks\n");
        return 2;
    }
    /* Tasks by priority (a lower number first), stable. */
    for (k = 0; k < n; k++) order[k] = k;
    for (k = 1; k < n; k++)
        for (j = k; j > 0 && P(_tasks)[order[j]].priority < P(_tasks)[order[j - 1]].priority; j--) {
            uint32_t t = order[j];
            order[j] = order[j - 1];
            order[j - 1] = t;
        }
    for (k = 0; k < n; k++) base = gcd(base, P(_tasks)[k].interval_ns);
    if (base == 0) base = 1000000; /* only cyclic tasks: 1 ms */

    ctx.image.i = I;
    ctx.image.q = Q;
    ctx.image.m = M;
    ctx.image.i_size = ctx.image.q_size = ctx.image.m_size = IMAGE_BYTES;
    P(_init)(&ctx);
    trace("init", q, m);

    have = scanf("%15s %lu %lu %8192s", word, &at, &offset, hex) == 4;
    for (tick = 0; tick < ticks; tick++) {
        char label[32];
        int64_t now = (int64_t)tick * base;
        while (have && at == tick) {
            size_t b, len = strlen(hex) / 2;
            if (strcmp(word, "set") != 0 || offset + len > IMAGE_BYTES) {
                fprintf(stderr, "script: bad line at tick %lu\n", at);
                return 2;
            }
            for (b = 0; b < len; b++) {
                unsigned v;
                sscanf(hex + 2 * b, "%2x", &v);
                I[offset + b] = (uint8_t)v;
            }
            have = scanf("%15s %lu %lu %8192s", word, &at, &offset, hex) == 4;
        }
        for (k = 0; k < n; k++) {
            const P(_task_t) *t = &P(_tasks)[order[k]];
            if (t->interval_ns == 0 || now % t->interval_ns == 0) t->run(&ctx, now);
        }
        snprintf(label, sizeof label, "%lu", tick);
        trace(label, q, m);
    }
    return 0;
}
