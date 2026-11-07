# The survey: Prokop's programs through iecf

Prokop is iecf's first user. Its toolchain tests feed matiec's `iec2c`
every program they build; `survey.py capture` runs them with a stand-in
`iec2c` that keeps a copy of each, and `survey.py run` builds each with
matiec and with `iecf build`, says where iecf stops, and runs each that
both build side by side (`iecf-harness compare`), every variable
compared for 40 scans on pseudo-random inputs.

```
survey.py capture ~/Workspace/Repos/prokop ~/Workspace/External/matiec WORK
survey.py run ~/Workspace/External/matiec WORK > table.md
```

The programs are Prokop's, and stay out of this repository: a program
is named by the first 12 hex digits of its SHA-1, which `capture` gives
again from the same Prokop commit. [results.md](https://github.com/raspihats/iecf/blob/main/survey/results.md) is the last
run. Some programs are tests that give matiec a wrong one on purpose;
those are "matiec fails" and do not count.
