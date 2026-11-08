#!/usr/bin/env python3
"""Prokop's programs through the reference compiler and through iecf (survey/README.md).

    survey.py capture PROKOP_DIR REFERENCE_DIR WORK   run Prokop's toolchain tests
                                                   with a stand-in iec2c that
                                                   keeps each program it is fed
    survey.py run REFERENCE_DIR WORK               build each with both, run
                                                   each that both build side by side
                                                   (iecf-harness compare); print the
                                                   table survey/results.md holds
"""
import collections
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

IECF = Path(__file__).resolve().parent.parent
SHIM = """#!/bin/sh
# Stand-in iec2c: keep a copy of the input, then run the reference compiler's.
for a in "$@"; do case "$a" in *.st|*.ST) src="$a";; esac; done
if [ -n "$src" ] && [ -f "$src" ]; then cp "$src" "{captured}/$(sha1sum "$src" | cut -c1-12).st"; fi
exec "{reference}/iec2c" "$@"
"""


def capture(prokop: Path, reference: Path, work: Path) -> None:
    shim, captured = work / "reference", work / "captured"
    for d in (shim, captured):
        shutil.rmtree(d, ignore_errors=True)
        d.mkdir(parents=True)
    (shim / "lib").symlink_to(reference / "lib")
    (shim / "iec2c").write_text(SHIM.format(captured=captured, reference=reference))
    (shim / "iec2c").chmod(0o755)
    env = dict(os.environ, PROKOP_MATIEC_DIR=str(shim))
    subprocess.run([sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider"], cwd=prokop / "toolchain", env=env)
    print(f"{len(list(captured.glob('*.st')))} programs in {captured}")


def first_line(text: str, pattern: str) -> str:
    for line in text.splitlines():
        if re.search(pattern, line):
            return line.strip()
    return text.strip().splitlines()[0] if text.strip() else ""


def run(reference: Path, work: Path) -> None:
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo/bin/cargo")
    subprocess.run([cargo, "build", "-q", "-p", "iecf-cli"], cwd=IECF, check=True)
    iecf = IECF / "target/debug/iecf"
    rows = []
    for src in sorted((work / "captured").glob("*.st")):
        with tempfile.TemporaryDirectory() as t:
            m = subprocess.run([str(reference / "iec2c"), "-I", str(reference / "lib"), "-T", t, str(src)],
                               capture_output=True, text=True, timeout=60)
        if m.returncode != 0:
            rows.append((src.stem, "reference fails", first_line(m.stdout + m.stderr, "error")))
            continue
        with tempfile.TemporaryDirectory() as t:
            r = subprocess.run([str(iecf), "build", "-o", t, str(src)], capture_output=True, text=True, timeout=60)
        out = r.stdout + r.stderr
        if r.returncode == 0:
            rows.append((src.stem, "builds", ""))
        else:
            line = first_line(out, r"error\[")
            code = re.search(r"error\[(\w+)\]", line)
            msg = re.sub(r"^.*?error\[\w+\]: ", "", line)
            rows.append((src.stem, code.group(1) if code else "fails", msg))
    # Those both build: run side by side, every variable compared.
    built = [work / "captured" / f"{n}.st" for n, s, _ in rows if s == "builds"]
    same = {}
    if built:
        r = subprocess.run([cargo, "run", "-q", "-p", "iecf-harness", "--", "compare", "--reference", str(reference)]
                           + [str(b) for b in built], cwd=IECF, capture_output=True, text=True)
        for line in r.stdout.splitlines():
            name, _, result = line.partition(": ")
            same[name] = result
    count = collections.Counter(s for _, s, _ in rows)
    ok = sum(1 for _, s, _ in rows if s != "reference fails")
    alike = sum(1 for v in same.values() if v.startswith("same"))
    print(f"{len(rows)} programs; {ok} build with the reference compiler; {count['builds']} of those iecf builds; "
          f"{alike} of those run the same, every variable compared\n")
    print("| program | iecf | first thing it stops at, or how it runs beside the reference compiler |\n|---|---|---|")
    for name, status, msg in sorted(rows, key=lambda r: (r[1] == "builds", same.get(r[0], "").startswith("same"),
                                                          r[1], same.get(r[0], r[2]))):
        text = same.get(name, msg) if status == "builds" else msg
        print(f"| {name} | {status} | {text.replace('|', '/')} |")


if __name__ == "__main__":
    cmd = sys.argv[1:2]
    if cmd == ["capture"] and len(sys.argv) == 5:
        capture(*(Path(a).resolve() for a in sys.argv[2:]))
    elif cmd == ["run"] and len(sys.argv) == 4:
        run(*(Path(a).resolve() for a in sys.argv[2:]))
    else:
        sys.exit(__doc__)
