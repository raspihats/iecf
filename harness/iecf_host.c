/* iecf_host.c - runs iecf's C for the harness (harness/README.md).
 *
 * Generic: compiled with -DIECF_PROJECT=<name> and -include <name>.h,
 * the files `iecf build` wrote for one case. The platform side of
 * docs/c-output.md, as small as it can be: the process image, the
 * context, and a scheduler of the project's tasks.
 *
 * Reads the cycle script from stdin as reference_host.c does and writes the
 * same trace: one line after initialisation and one per tick, the Q and M
 * bytes in hex. A tick is the greatest common divisor of the tasks'
 * intervals, as the reference compiler's common_ticktime__; a task runs on the ticks its
 * interval divides, the tasks in order of priority; one with SINGLE on its
 * rising edges, and at its interval only while SINGLE is FALSE. It first checks the
 * symbol table holds together (docs/c-output.md §6). A fault adds
 * "fault CODE POU LINE" to the line, and no task runs after it; a flagged
 * infinite or NaN real adds "flag CODE POU LINE" and changes nothing.
 *
 * A script whose first line after `ticks N` is `vars 0 0 00` also prints,
 * after each line, every leaf of every variable the symbol table lists -
 * array elements, fields, block members - one per line: `  PATH VALUE`, in the form reference_host.c prints the reference compiler's
 * (`iecf-harness compare`): integers in decimal, a REAL in %.9g and an
 * LREAL in %.17g (each exact), a TIME and a TOD in ms, a DATE and a DT
 * in seconds, a STRING quoted.
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
static int show_vars;

/* One elementary value at `p` (bit `bit` of it for a BOOL at a bit in the
 * image; a BOOL in a whole byte, IECF_NO_BIT, TRUE for any value but 0). */
static void print_value(uint32_t elem, const uint8_t *p, int image, unsigned bit) {
    int64_t s = 0;
    uint64_t u = 0;
    switch (elem) {
    case IECF_T_BOOL:
        printf("%u", image && bit != IECF_NO_BIT ? (unsigned)((p[0] >> bit) & 1u) : (unsigned)(p[0] != 0));
        return;
    case IECF_T_SINT: { int8_t v; memcpy(&v, p, 1); s = v; break; }
    case IECF_T_INT: { int16_t v; memcpy(&v, p, 2); s = v; break; }
    case IECF_T_DINT: { int32_t v; memcpy(&v, p, 4); s = v; break; }
    case IECF_T_LINT: { int64_t v; memcpy(&v, p, 8); s = v; break; }
    case IECF_T_USINT: case IECF_T_BYTE: case IECF_T_CHAR: u = p[0]; printf("%llu", (unsigned long long)u); return;
    case IECF_T_UINT: case IECF_T_WORD: case IECF_T_WCHAR: { uint16_t v; memcpy(&v, p, 2); printf("%llu", (unsigned long long)v); return; }
    case IECF_T_UDINT: case IECF_T_DWORD: { uint32_t v; memcpy(&v, p, 4); printf("%llu", (unsigned long long)v); return; }
    case IECF_T_ULINT: case IECF_T_LWORD: { uint64_t v; memcpy(&v, p, 8); printf("%llu", (unsigned long long)v); return; }
    case IECF_T_REAL: { float v; memcpy(&v, p, 4); printf("%.9g", (double)v); return; }
    case IECF_T_LREAL: { double v; memcpy(&v, p, 8); printf("%.17g", v); return; }
    case IECF_T_TIME: { int32_t v; memcpy(&v, p, 4); printf("%ldms", (long)v); return; }
    case IECF_T_TOD: { uint32_t v; memcpy(&v, p, 4); printf("%lums", (unsigned long)v); return; }
    case IECF_T_DATE: case IECF_T_DT: { uint32_t v; memcpy(&v, p, 4); printf("%lus", (unsigned long)v); return; }
    case IECF_T_STRING: {
        const char *c = (const char *)p;
        putchar('\'');
        for (; *c; c++) {
            unsigned char ch = (unsigned char)*c;
            if (ch >= 32 && ch < 127 && ch != '\'' && ch != '$') putchar(ch); else printf("$%02X", ch);
        }
        putchar('\'');
        return;
    }
    case IECF_T_WSTRING: { /* UTF-16 units, "$hhhh" for all but printable ASCII */
        uint16_t u16;
        putchar('"');
        for (;; p += 2) {
            memcpy(&u16, p, 2);
            if (u16 == 0) break;
            if (u16 >= 32 && u16 < 127 && u16 != '"' && u16 != '$') putchar(u16); else printf("$%04X", (unsigned)u16);
        }
        putchar('"');
        return;
    }
    default: printf("?"); return;
    }
    printf("%lld", (long long)s);
}

/* The leaves under one variable: its elements (row-major, the first
 * MAX_ELEMENTS of an array, as src/vars.rs writes the reference compiler's), its fields,
 * a block's members, each as PATH[i,j].FIELD. An enumeration prints its
 * value's number. */
#define MAX_ELEMENTS 64

/* The size of type t laid out in the image, IECF_SYM_PACKED: its parts
 * back to back, each elementary one its C size (a BOOL a byte); a
 * structure with relative locations (IECF_KIND_LAYOUT) its bytes. */
static uint64_t packed_size(uint32_t t) {
    const iecf_type_t *ty = &P(_types)[t];
    uint64_t n = 1;
    uint32_t k;
    switch (ty->kind) {
    case IECF_KIND_STRUCT:
        for (n = 0, k = 0; k < ty->member_count; k++) n += packed_size(ty->members[k].type);
        return n;
    case IECF_KIND_ARRAY:
        for (k = 0; k < ty->dims; k++) n *= (uint64_t)(ty->bounds[2 * k + 1] - ty->bounds[2 * k] + 1);
        return n * packed_size(ty->of);
    default:
        return ty->size;
    }
}

static void print_leaf(const char *path, uint32_t t, const uint8_t *p, int image, unsigned bit, int packed);

/* A variable at a `*` address, IECF_SYM_PARTLY: `p` holds the iecf_ref_t
 * VAR_CONFIG gave it, `t` is a reference to its type; its value is in the
 * image where that points. */
static void print_partly(const char *path, uint32_t t, const uint8_t *p) {
    iecf_ref_t r;
    const iecf_type_t *of = &P(_types)[P(_types)[t].of];
    memcpy(&r, p, sizeof r);
    if (r.area == 0u) {
        printf("  %s NULL\n", path);
        return;
    }
    p = (r.area == 'I' ? I : r.area == 'Q' ? Q : M) + r.off;
    print_leaf(path, P(_types)[t].of, p, 1, r.bit, of->kind != IECF_KIND_ELEM && of->kind != IECF_KIND_ENUM);
}

/* `packed`: an array or structure at an address, IECF_SYM_PACKED. */
static void print_leaf(const char *path, uint32_t t, const uint8_t *p, int image, unsigned bit, int packed) {
    const iecf_type_t *ty = &P(_types)[t];
    char sub[512];
    uint32_t k;
    switch (ty->kind) {
    case IECF_KIND_ELEM:
    case IECF_KIND_ENUM:
        if (ty->elem == IECF_T_LTIME) return;
        printf("  %s ", path);
        print_value(ty->elem, p, image, packed ? IECF_NO_BIT : bit);
        printf("\n");
        return;
    /* A reference: whether it references anything; where is the run's. */
    case IECF_KIND_REF: {
        iecf_ref_t r;
        memcpy(&r, p, sizeof r);
        printf("  %s %s\n", path, iecf_ref_is_null(r) ? "NULL" : "REF");
        return;
    }
    /* Its bytes wherever it is: each member at its byte, read as the
       image is, an array or structure in it back to back. */
    case IECF_KIND_LAYOUT:
        for (k = 0; k < ty->member_count; k++) {
            const iecf_member_t *m = &ty->members[k];
            const iecf_type_t *mt = &P(_types)[m->type];
            snprintf(sub, sizeof sub, "%s.%s", path, m->name);
            if (mt->kind == IECF_KIND_ELEM || mt->kind == IECF_KIND_ENUM) {
                if (mt->elem == IECF_T_LTIME) continue;
                printf("  %s ", sub);
                print_value(mt->elem, p + m->offset, 1, m->bit);
                printf("\n");
            } else {
                print_leaf(sub, m->type, p + m->offset, 1, IECF_NO_BIT, 1);
            }
        }
        return;
    case IECF_KIND_STRUCT:
    case IECF_KIND_BLOCK: {
        uint64_t at = 0;
        for (k = 0; k < ty->member_count; k++) {
            snprintf(sub, sizeof sub, "%s.%s", path, ty->members[k].name);
            if (ty->members[k].flags & IECF_SYM_PARTLY)
                print_partly(sub, ty->members[k].type, p + ty->members[k].offset);
            else
                print_leaf(sub, ty->members[k].type, p + (packed ? at : ty->members[k].offset), image, 0, packed);
            at += packed_size(ty->members[k].type);
        }
        return;
    }
    case IECF_KIND_ARRAY: {
        uint64_t n = 1, e;
        uint32_t d;
        for (d = 0; d < ty->dims; d++) n *= (uint64_t)(ty->bounds[2 * d + 1] - ty->bounds[2 * d] + 1);
        for (e = 0; e < n && e < MAX_ELEMENTS; e++) {
            uint64_t rest = e;
            int64_t idx[8];
            size_t len;
            for (d = ty->dims; d-- > 0;) {
                uint64_t w = (uint64_t)(ty->bounds[2 * d + 1] - ty->bounds[2 * d] + 1);
                idx[d] = ty->bounds[2 * d] + (int64_t)(rest % w);
                rest /= w;
            }
            len = (size_t)snprintf(sub, sizeof sub, "%s[", path);
            for (d = 0; d < ty->dims && len < sizeof sub; d++)
                len += (size_t)snprintf(sub + len, sizeof sub - len, d ? ",%lld" : "%lld", (long long)idx[d]);
            if (len < sizeof sub) snprintf(sub + len, sizeof sub - len, "]");
            print_leaf(sub, ty->of, p + e * (packed ? packed_size(ty->of) : P(_types)[ty->of].size), image, 0, packed);
        }
        return;
    }
    default:
        return;
    }
}

static void print_vars(void) {
    uint32_t k;
    for (k = 0; k < P(_symbol_count); k++) {
        const iecf_symbol_t *sy = &P(_symbols)[k];
        const uint8_t *p;
        switch (sy->area) {
        case 0: p = (const uint8_t *)&ctx + sy->offset; break;
        case 'I': p = I + sy->offset; break;
        case 'Q': p = Q + sy->offset; break;
        default: p = M + sy->offset; break;
        }
        if (sy->flags & IECF_SYM_PARTLY)
            print_partly(sy->path, sy->type, p);
        else
            print_leaf(sy->path, sy->type, p, sy->area != 0, sy->bit, (sy->flags & IECF_SYM_PACKED) != 0);
    }
}

/* The harness's hierarchical addresses (docs/c-output.md §4): a test
 * mapping, the levels but a bit address's last folded from byte 256,
 * each level a hex digit (%IW2.5.7.1 is byte 256 + 0x2571 % 1024), the
 * bit that last level. A level of 99 is an address the host has not. */
iecf_ref_t iecf_address(const void *c, const iecf_address_t *a) {
    uint32_t k, n = a->levels, byte = 0;
    (void)c;
    if (a->size == 'X') n--;
    for (k = 0; k < a->levels; k++)
        if (a->level[k] == 99u) return iecf_ref_null();
    for (k = 0; k < n; k++) byte = byte * 16u + a->level[k];
    byte = 256u + byte % 1024u;
    return iecf_ref_image((uint8_t)a->area, byte, a->size == 'X' ? (uint8_t)(a->level[n] % 8u) : IECF_NO_BIT);
}

#ifdef IECF_HOST_STEPS
/* A debug build's hook (docs/c-output.md §9): each statement as it
 * runs, `step POU LINE INSTANCE`, the instance a block's symbol path
 * where the symbol table has one at `self` (a program's own is `-`). */
void iecf_debug_step(const void *c, uint16_t pou, uint32_t line, const void *self) {
    const char *where = "-";
    uint32_t k;
    (void)c;
    if (self != NULL) {
        size_t off = (size_t)((const char *)self - (const char *)&ctx);
        for (k = 0; k < P(_symbol_count); k++) {
            const iecf_symbol_t *sy = &P(_symbols)[k];
            /* A block's first member shares its address: the type says. */
            if (sy->area == 0 && sy->offset == off && P(_types)[sy->type].kind == IECF_KIND_BLOCK &&
                strcmp(P(_types)[sy->type].name, P(_pou_names)[pou]) == 0) {
                where = sy->path;
                break;
            }
        }
    }
    printf("  step %s %lu %s\n", P(_pou_names)[pou], (unsigned long)line, where);
}
#endif

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
    if (show_vars) print_vars();
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
    /* Not followed: a structure may reference its own type. */
    if (ty->kind == IECF_KIND_REF) {
        if (ty->of >= P(_type_count) || ty->size != sizeof(iecf_ref_t)) {
            fprintf(stderr, "symbols: %s is no reference\n", ty->name);
            exit(3);
        }
        return;
    }
    if (ty->kind == IECF_KIND_ARRAY) {
        uint64_t n = 1;
        for (k = 0; k < ty->dims; k++) n *= (uint64_t)(ty->bounds[2 * k + 1] - ty->bounds[2 * k] + 1);
        check_type(ty->of, depth + 1);
        if (n * P(_types)[ty->of].size != ty->size) {
            fprintf(stderr, "symbols: %s is not its elements' size\n", ty->name);
            exit(3);
        }
    }
    if (ty->kind == IECF_KIND_LAYOUT)
        for (k = 0; k < ty->member_count; k++) {
            const iecf_member_t *m = &ty->members[k];
            check_type(m->type, depth + 1);
            if (m->offset + (m->bit != IECF_NO_BIT ? 1u : packed_size(m->type)) > ty->size
                || (m->bit != IECF_NO_BIT && (m->bit > 7u || P(_types)[m->type].elem != IECF_T_BOOL))) {
                fprintf(stderr, "symbols: %s.%s outside %s\n", ty->name, m->name, ty->name);
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
        end = (uint64_t)s->offset + ((s->flags & IECF_SYM_PACKED) ? packed_size(s->type) : P(_types)[s->type].size);
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
    int single_was[64] = {0}; /* each task's SINGLE at its last look: FALSE before the first */
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
    have = scanf("%15s %lu %lu %8192s", word, &at, &offset, hex) == 4;
    if (have && strcmp(word, "vars") == 0) {
        show_vars = 1;
        have = scanf("%15s %lu %lu %8192s", word, &at, &offset, hex) == 4;
    }
    P(_init)(&ctx);
    trace("init", q, m);

    for (tick = 0; tick < ticks; tick++) {
        char label[32];
        int64_t now = (int64_t)tick * base;
        while (have && at == tick) {
            size_t b, len = strlen(hex) / 2;
            if (strcmp(word, "warm") == 0) {
                /* A warm restart: every variable but the RETAIN ones. */
                P(_warm)(&ctx);
                have = scanf("%15s %lu %lu %8192s", word, &at, &offset, hex) == 4;
                continue;
            }
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
            /* Ed.3 6.8.2 a, b: on each rising edge of SINGLE; at the
             * interval as long as SINGLE stands at FALSE; with neither, at
             * every tick (cyclic). An interval that is a variable is read
             * now; it should be a multiple of the tick. */
            int64_t iv = t->interval ? t->interval(&ctx) : t->interval_ns;
            int s = t->single ? t->single(&ctx) != 0 : 0;
            int edge = t->single != NULL && s && !single_was[order[k]];
            int periodic = (t->single == NULL && t->interval == NULL && iv == 0) || (iv > 0 && !s && now % iv == 0);
            if (t->single) single_was[order[k]] = s;
            if (edge || periodic) t->run(&ctx, now);
        }
        snprintf(label, sizeof label, "%lu", tick);
        trace(label, q, m);
    }
    return 0;
}
