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
 * interval divides, the tasks in order of priority. It first checks the
 * symbol table holds together (docs/c-output.md §6). A fault adds
 * "fault CODE POU LINE" to the line, and no task runs after it; a flagged
 * infinite or NaN real adds "flag CODE POU LINE" and changes nothing.
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
    if (ctx.flag.code != IECF_FAULT_NONE)
        printf(" flag %u %s %lu", (unsigned)ctx.flag.code, P(_pou_names)[ctx.flag.pou], (unsigned long)ctx.flag.line);
    printf("\n");
}

/* The symbol table holds together: each variable inside the context or
 * its image area, each member inside its type, each array the size of
 * its elements. Exits 3 on the first that does not. */
static void check_type(uint32_t t, uint32_t depth) {
    const iecf_type_t *ty;
    uint32_t k;
    if (t >= P(_type_count) || depth > 64) {
        fprintf(stderr, "symbols: type %lu out of the table\n", (unsigned long)t);
        exit(3);
    }
    ty = &P(_types)[t];
    if (ty->kind == IECF_KIND_ARRAY) {
        uint64_t n = 1;
        for (k = 0; k < ty->dims; k++) n *= (uint64_t)(ty->bounds[2 * k + 1] - ty->bounds[2 * k] + 1);
        check_type(ty->of, depth + 1);
        if (n * P(_types)[ty->of].size != ty->size) {
            fprintf(stderr, "symbols: %s is not its elements' size\n", ty->name);
            exit(3);
        }
    }
    if (ty->kind == IECF_KIND_STRUCT || ty->kind == IECF_KIND_BLOCK)
        for (k = 0; k < ty->member_count; k++) {
            const iecf_member_t *m = &ty->members[k];
            check_type(m->type, depth + 1);
            if (m->offset + P(_types)[m->type].size > ty->size) {
                fprintf(stderr, "symbols: %s.%s outside %s\n", ty->name, m->name, ty->name);
                exit(3);
            }
        }
}

static void check_symbols(void) {
    uint32_t k;
    for (k = 0; k < P(_symbol_count); k++) {
        const iecf_symbol_t *s = &P(_symbols)[k];
        uint64_t end;
        check_type(s->type, 0);
        end = (uint64_t)s->offset + P(_types)[s->type].size;
        if ((s->area == 0 && end > sizeof ctx) || (s->area != 0 && end > IMAGE_BYTES)) {
            fprintf(stderr, "symbols: %s outside its area\n", s->path);
            exit(3);
        }
    }
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
    check_symbols();
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
