# Contributing to iecf

## Every change is documented, in the same commit

Every addition to iecf and every fix is documented where its reader will
look, in the commit that makes it - never later, never only in the
commit message:

- **The book** (`docs/`, published at
  <https://raspihats.github.io/iecf/>): the page that describes what
  changed. What a program gives - the behaviour reference
  (`docs/behaviour/`), with a checked example where it shows a value; a
  diagnostic - its page in `docs/errors/` and its line in
  `docs/SUMMARY.md`; an extension - `docs/extensions.md`; the C or the
  runtime header - `docs/c-output.md`; a choice Ed.3 leaves open -
  `docs/dependencies.md`; a test program - `harness/README.md`'s list of
  cases; a measurement - `docs/testing/performance.md`. A fix that
  changes what a page says corrects the page.
- **The README**, when the change alters what iecf is, does or can be
  used for: its state, its numbers (the harness's count of programs,
  the performance figures), its commands.
- **The introduction** (`docs/introduction.md`, the book's first page),
  when it changes what iecf covers.

A summary page goes stale quietly - a count, "not yet", a size - so a
change that moves one of its facts updates it too. The book is built by
CI (`book.yml`) and its examples are run (`iecf-harness book`); a page
that names a commit or a number names the current one.
