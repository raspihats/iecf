#!/bin/bash
# iecf's benchmark (docs/testing/performance.md): one program built by
# iecf and by the reference compiler, each run TICKS cycles by the
# harness's hosts, the best of five timed; the traces compared.
#
# usage: REFERENCE=DIR tools/bench/run.sh PROGRAM TICKS [SCRIPT]
#   PROGRAM  a file of tools/bench without .st, or a path to a .st
#   TICKS    cycles to run
#   SCRIPT   a host script to run instead of `ticks TICKS` (inputs set
#            each cycle, harness/README.md's host form)
#   REFERENCE  the reference compiler's directory (its iec2c, lib/);
#            unset, iecf alone. IECF: another iecf to time beside
#            (an older build), IECF_ARGS: its options (oo.st takes
#            `--extension codesys`), CC and CFLAGS (-O2) the C compiler.
# Run from the repository's root, after `cargo build --release`.
set -u
root=$(pwd)
prog=$1; ticks=$2; script=${3:-}
st=$prog; [ -f "$st" ] || st=$root/tools/bench/$prog.st
st=$(cd "$(dirname "$st")" && pwd)/$(basename "$st")
name=$(basename "$st" .st)
cc=${CC:-cc}; cflags=${CFLAGS:--O2}
work=${TMPDIR:-/tmp}/iecf-bench/$name
rm -rf "$work"; mkdir -p "$work"
ms() { echo $(( $(date +%s%N) / 1000000 )); }

# iecf's C and the harness's host: a build of `bin` into `dir`.
build_iecf() {
    local label=$1 bin=$2 dir=$work/$1
    mkdir -p "$dir"
    local t0; t0=$(ms)
    # shellcheck disable=SC2086
    "$bin" build -o "$dir" ${IECF_ARGS:-} "$st" > "$dir/build.log" 2>&1 || { echo "$label: iecf build failed"; tail -3 "$dir/build.log"; return 1; }
    local t1; t1=$(ms)
    local c; c=$(cd "$dir" && ls ./*.c | head -1); c=${c#./}; local p=${c%.c}
    $cc -std=c99 $cflags -w -I "$dir" -c "$dir/$c" -o "$dir/project.o" &&
        $cc -std=c99 $cflags -w -I "$dir" -DIECF_PROJECT="$p" -DIECF_PROJECT_UP="${p^^}" -include "$dir/$p.h" \
            "$root/harness/iecf_host.c" "$dir/project.o" -lm -o "$dir/host" || return 1
    echo "$label: to C $((t1 - t0)) ms, C compiled in $(( $(ms) - t1 )) ms"
}

# The reference compiler's C and its host, as the harness builds them.
build_reference() {
    local dir=$work/reference
    mkdir -p "$dir"; cd "$dir" || return 1
    local t0; t0=$(ms)
    "$REFERENCE/iec2c" -r -I "$REFERENCE/lib" "$st" > build.log 2>&1 || { echo "reference: build failed"; tail -3 build.log; cd "$root"; return 1; }
    local t1; t1=$(ms)
    echo "void harness_vars(void) {}" > vars.c
    # shellcheck disable=SC2046
    $cc $cflags -w -I "$dir" -include "$root/harness/reference_fixes.h" -I "$REFERENCE/lib/C" \
        "$root/harness/reference_host.c" $(ls ./*.c | grep -v '^./POUS.c$') -lm -o host || { cd "$root"; return 1; }
    echo "reference: to C $((t1 - t0)) ms, C compiled in $(( $(ms) - t1 )) ms"
    cd "$root" || return 1
}

hosts=()
build_iecf iecf "$root/target/release/iecf" && hosts+=(iecf)
[ -n "${IECF:-}" ] && build_iecf iecf-other "$IECF" && hosts+=(iecf-other)
[ -n "${REFERENCE:-}" ] && build_reference && hosts+=(reference)

for h in "${hosts[@]}"; do
    best=999999999
    for _ in 1 2 3 4 5; do
        t0=$(date +%s%N)
        if [ -n "$script" ]; then "$work/$h/host" < "$script" > "$work/$h.trace"
        else printf 'ticks %s\n' "$ticks" | "$work/$h/host" > "$work/$h.trace"; fi
        t1=$(date +%s%N)
        us=$(( (t1 - t0) / 1000 )); [ $us -lt $best ] && best=$us
    done
    echo "$h: $(( best / ticks )) us a cycle ($best us for $ticks), last: $(tail -1 "$work/$h.trace" | cut -c1-80)"
done
# The same Q and M bytes each cycle; iecf's flags and faults are its own.
if [ ${#hosts[@]} -gt 1 ]; then
    strip() { sed -E 's/ (flag|fault) .*$//' "$1"; }
    if cmp -s <(strip "$work/${hosts[0]}.trace") <(strip "$work/${hosts[-1]}.trace"); then echo "traces: the same"
    else echo "traces: DIFFER"; fi
fi
