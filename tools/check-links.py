#!/usr/bin/env python3
"""Every link inside the built book's pages leads somewhere: a file that
exists and, with a #fragment, an element of that id. Run after
`mdbook build`: tools/check-links.py target/book"""
import os
import re
import sys

root = sys.argv[1] if len(sys.argv) > 1 else "target/book"
bad = []
for d, _, files in os.walk(root):
    for f in files:
        if not f.endswith(".html"):
            continue
        page = os.path.join(d, f)
        text = open(page, encoding="utf-8").read()
        main = text.split("<main>")[1].split("</main>")[0] if "<main>" in text else ""
        for href in re.findall(r'href="([^"]+)"', main):
            if href.startswith(("http:", "https:", "mailto:")):
                continue
            path, _, frag = href.partition("#")
            target = os.path.normpath(os.path.join(d, path)) if path else page
            if not os.path.exists(target):
                bad.append(f"{page}: {href}: no such page")
            elif frag and target.endswith(".html") and f'id="{frag}"' not in open(target, encoding="utf-8").read():
                bad.append(f"{page}: {href}: no such anchor")
for b in bad:
    print(b)
sys.exit(1 if bad else 0)
