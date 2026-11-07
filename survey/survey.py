#!/usr/bin/env python3
"""Prokop's programs through matiec and through iecf (survey/README.md).

    survey.py capture PROKOP_DIR MATIEC_DIR WORK   run Prokop's toolchain tests
                                                   with a stand-in iec2c that
                                                   keeps each program it is fed
    survey.py run MATIEC_DIR WORK                  build each with both; print
                                                   the table survey/results.md holds
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
# Stand-in iec2c: keep a copy of the input, then run matiec's.
for a in "$@"; do case "$a" in *.st|*.ST) src="$a";; esac; done
if [ -n "$src" ] && [ -f "$src" ]; then cp "$src" "{captured}/$(sha1sum "$src" | cut -c1-12).st"; fi
exec "{matiec}/iec2c" "$@"
"""


def capture(prokop: Path, matiec: Path, work: Path) -> None:
    shim, captured = work / "matiec", work / "captured"
    for d in (shim, captured):
        shutil.rmtree(d, ignore_errors=True)
        d.mkdir(parents=True)
    (shim / "lib").symlink_to(matiec / "lib")
    (shim / "iec2c").write_text(SHIM.format(captured=captured, matiec=matiec))
    (shim / "iec2c").chmod(0o755)
    env = dict(os.environ, PROKOP_MATIEC_DIR=str(shim))
    subprocess.run([sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider"], cwd=prokop / "toolchain", env=env)
    print(f"{len(list(captured.glob('*.st')))} programs in {captured}")


def first_line(text: str, pattern: str) -> str:
    for line in text.splitlines():
        if re.search(pattern, line):
            return line.strip()
    return text.strip().splitlines()[0] if text.strip() else ""


def run(matiec: Path, work: Path) -> None:
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo/bin/cargo")
    subprocess.run([cargo, "build", "-q", "-p", "iecf-cli"], cwd=IECF, check=True)
    iecf = IECF / "target/debug/iecf"
    rows = []
    for src in sorted((work / "captured").glob("*.st")):
        with tempfile.TemporaryDirectory() as t:
            m = subprocess.run([str(matiec / "iec2c"), "-I", str(matiec / "lib"), "-T", t, str(src)],
                               capture_output=True, text=True, timeout=60)
        if m.returncode != 0:
            rows.append((src.stem, "matiec fails", first_line(m.stdout + m.stderr, "error")))
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
    count = collections.Counter(s for _, s, _ in rows)
    ok = sum(1 for _, s, _ in rows if s != "matiec fails")
    print(f"{len(rows)} programs; {ok} matiec builds; {count['builds']} of those iecf builds\n")
    print("| program | iecf | first thing it stops at |\n|---|---|---|")
    for name, status, msg in sorted(rows, key=lambda r: (r[1] == "builds", r[1], r[2])):
        print(f"| {name} | {status} | {msg.replace('|', '/')} |")


if __name__ == "__main__":
    cmd = sys.argv[1:2]
    if cmd == ["capture"] and len(sys.argv) == 5:
        capture(*(Path(a).resolve() for a in sys.argv[2:]))
    elif cmd == ["run"] and len(sys.argv) == 4:
        run(*(Path(a).resolve() for a in sys.argv[2:]))
    else:
        sys.exit(__doc__)
