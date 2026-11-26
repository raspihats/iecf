#!/bin/bash
# What can be checked of CONTRIBUTING.md's rule - every addition and fix
# documented where its reader looks - is checked here, by CI (ci.yml):
#
# - each harness case is named in harness/README.md's list;
# - the README's count of harness programs is the count of cases;
# - each diagnostic code iecf has has its page in docs/errors/, and the
#   page its line in docs/SUMMARY.md;
# - each extension has its section in docs/extensions.md, a pack its
#   place under Packs.
#
# The book's build has no warnings (book.yml): a stray `<name>` outside
# a code span is read as an HTML tag and vanishes from the page.
# Run from the repository's root.
set -u
fail=0
bad() { echo "check-docs: $*"; fail=1; }

cases=$(ls harness/cases/*.st | xargs -n1 basename | sed 's/\.st$//')
for c in $cases; do
    grep -q "\`$c\`" harness/README.md || bad "harness case $c is not in harness/README.md's list"
done
n=$(echo "$cases" | wc -l)
grep -q "runs iecf's C on $n programs" README.md ||
    bad "README.md does not say the harness runs iecf's C on $n programs"

for code in $(grep -rhoE '"[EW][0-9]{4}"' crates --include=*.rs | tr -d '"' | sort -u); do
    [ -f "docs/errors/$code.md" ] || bad "$code has no page docs/errors/$code.md"
done
for page in docs/errors/*.md; do
    code=$(basename "$page" .md)
    grep -q "(errors/$code.md)" docs/SUMMARY.md || bad "$code's page is not in docs/SUMMARY.md"
done

for ext in $(sed -n '/pub fn name(self)/,/^    }/p' crates/iecf-sema/src/lib.rs | grep -oE '=> "[a-z0-9-]+"' | sed 's/=> //; s/"//g'); do
    grep -q "^## $ext:" docs/extensions.md && continue
    # a pack, several at once, is described under Packs
    sed -n '/^## Packs/,/^## [a-z]/p' docs/extensions.md | grep -q "\`$ext\`" ||
        bad "extension $ext has no section in docs/extensions.md"
done

[ $fail = 0 ] && echo "check-docs: ok"
exit $fail
