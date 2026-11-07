//! `iecf-harness compare [--ticks N] [--seed S] FILE.st...`: each program
//! built with matiec and with iecf, run side by side on the same inputs,
//! and its variables compared tick by tick - every elementary variable of
//! its symbol table, not only the Q and M bytes (survey/README.md).
//!
//! The inputs are pseudo-random, the same for both, the same at every run
//! (a seed from the file's name and `--seed`): every input byte the
//! program addresses changes every fourth tick. The trace lines are those
//! of the hosts with `vars` (iecf_host.c, matiec_host.c); iecf's flags,
//! which matiec has no word for, are not compared, a fault is.
//!
//! One line per program: `same`, `differs at tick T: ...`, or why it did
//! not run. With `--expect-same`, any but `same` fails the run (CI, on
//! the cases harness/compare-cases lists).

use crate::{Built, Case, iecf_trace, matiec_trace};
use std::fs;
use std::path::Path;

/// The input bytes the program addresses, from iecf's header.
fn input_bytes(text: &str, name: &str) -> usize {
    let built = iecf_driver::build([(format!("{name}.st"), text.to_owned())], &iecf_driver::Options::default());
    let Ok(b) = built else { return 0 };
    for (f, t) in &b.files {
        if !f.ends_with(".h") || f == "iecf.h" {
            continue;
        }
        for l in t.lines() {
            if let Some(rest) = l.strip_prefix("#define ")
                && let Some((k, v)) = rest.split_once(' ')
                && k.ends_with("_I_BYTES")
            {
                return v.trim_end_matches('u').parse().unwrap_or(0);
            }
        }
    }
    0
}

fn script(n_in: usize, ticks: u64, seed: u64) -> String {
    let mut s = format!("ticks {ticks}\nvars 0 0 00\n");
    let mut x = seed | 1;
    for t in (0..ticks).step_by(4) {
        if n_in == 0 {
            break;
        }
        let mut hex = String::new();
        for _ in 0..n_in {
            // xorshift64
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            hex.push_str(&format!("{:02x}", (x >> 24) as u8));
        }
        s.push_str(&format!("set {t} 0 {hex}\n"));
    }
    s
}

/// The trace split into ticks: each tick's line without iecf's flag,
/// and its variable lines, sorted.
fn blocks(trace: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for l in trace.lines() {
        if let Some(v) = l.strip_prefix("  ") {
            if let Some(b) = out.last_mut() {
                b.1.push(v.to_owned());
            }
        } else {
            let head = l.split(" flag ").next().unwrap_or(l).to_owned();
            out.push((head, Vec::new()));
        }
    }
    for b in &mut out {
        b.1.sort();
    }
    out
}

fn compare(m: &str, i: &str) -> String {
    let (bm, bi) = (blocks(m), blocks(i));
    let vars = bi.first().map_or(0, |b| b.1.len());
    for (k, (a, b)) in bm.iter().zip(&bi).enumerate() {
        let tick = if k == 0 { "init".to_string() } else { (k - 1).to_string() };
        if b.0.contains(" fault ") {
            return format!("iecf faults at tick {tick}: `{}`", b.0);
        }
        if a.0 != b.0 {
            return format!("differs at tick {tick}: matiec `{}`, iecf `{}`", a.0, b.0);
        }
        // The variables both list. matiec also lists VAR_TEMP, which
        // iecf's symbol table leaves out (it lives only during a call);
        // a variable only iecf lists is a difference.
        let name = |l: &String| l.split(' ').next().unwrap_or("").to_owned();
        let in_m: Vec<String> = a.1.iter().map(name).collect();
        let only_i: Vec<String> = b.1.iter().map(name).filter(|n| !in_m.contains(n)).collect();
        if !only_i.is_empty() {
            return format!("variables only iecf lists: {only_i:?}");
        }
        let in_i: Vec<String> = b.1.iter().map(name).collect();
        let am: Vec<&String> = a.1.iter().filter(|l| in_i.contains(&name(l))).collect();
        if let Some((x, y)) = am.iter().zip(&b.1).find(|(x, y)| ***x != **y) {
            return format!("differs at tick {tick}: matiec `{x}`, iecf `{y}`");
        }
    }
    if bm.len() != bi.len() {
        return format!("traces of {} and {} lines", bm.len(), bi.len());
    }
    format!("same ({} ticks, {vars} variables)", bi.len().saturating_sub(1))
}

/// With `expect_same`, false unless every program runs the same.
pub(crate) fn run(
    root: &Path,
    matiec: &Path,
    files: &[String],
    ticks: u64,
    seed: u64,
    expect_same: bool,
) -> Result<bool, String> {
    let work = root.join("../target/harness/compare");
    let mut all_same = true;
    for f in files {
        let path = Path::new(f);
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("p").to_owned();
        let text = fs::read_to_string(path).map_err(|e| format!("{f}: {e}"))?;
        let dir = work.join(&name);
        let src = dir.join("src");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&src).map_err(|e| e.to_string())?;
        fs::write(src.join(format!("{name}.st")), &text).map_err(|e| e.to_string())?;
        let h = name.bytes().fold(seed ^ 0x9e37_79b9_7f4a_7c15, |a, b| a.rotate_left(5) ^ u64::from(b));
        let case = Case { name: name.clone(), dir: src, script: script(input_bytes(&text, &name), ticks, h) };
        let m = matiec_trace(root, matiec, &dir.join("matiec"), &case);
        let i = iecf_trace(root, &dir.join("iecf"), &case);
        let line = match (m, i) {
            (Err(e), _) => format!("matiec fails: {}", e.lines().next().unwrap_or("")),
            (_, Err(e)) => format!("iecf fails: {}", e.lines().nth(1).or(e.lines().next()).unwrap_or("")),
            (_, Ok(Built::NotYet(why))) => format!("iecf: not yet: {why}"),
            (Ok(m), Ok(Built::Trace(i))) => compare(&m, &i),
        };
        all_same &= line.starts_with("same");
        println!("{name}: {line}");
    }
    Ok(all_same || !expect_same)
}
