//! The harness (harness/README.md): each case's program run for a number
//! of ticks, its Q and M bytes traced, the trace compared with the one in
//! the repository.
//!
//! ```text
//! iecf-harness matiec [--matiec DIR] [--bless] [CASE...]
//! ```
//!
//! `matiec` compiles each case with matiec's iec2c, runs it under
//! `matiec_host.c` and compares with `cases/<case>.trace`; `--bless`
//! writes the trace instead. matiec is found by `--matiec` or
//! `$IECF_MATIEC_DIR`: a built checkout, `iec2c` at its top and `lib/`
//! beside it. The C compiler is `$CC`, or `cc`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::{env, fs, io::Write as _};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("iecf-harness: {e}");
            ExitCode::from(2)
        }
    }
}

struct Options {
    matiec: Option<PathBuf>,
    bless: bool,
    cases: Vec<String>,
}

fn run(args: Vec<String>) -> Result<bool, String> {
    let mut args = args.into_iter();
    let side = args.next().ok_or("usage: iecf-harness matiec [--matiec DIR] [--bless] [CASE...]")?;
    let mut opts = Options { matiec: env::var_os("IECF_MATIEC_DIR").map(PathBuf::from), bless: false, cases: vec![] };
    while let Some(a) = args.next() {
        match a.as_str() {
            "--matiec" => opts.matiec = Some(args.next().ok_or("--matiec needs a directory")?.into()),
            "--bless" => opts.bless = true,
            _ if a.starts_with('-') => return Err(format!("unknown option {a}")),
            _ => opts.cases.push(a),
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cases = find_cases(&root.join("cases"), &opts.cases)?;
    match side.as_str() {
        "matiec" => {
            let dir = opts.matiec.clone().ok_or("matiec not found: give --matiec DIR or set IECF_MATIEC_DIR")?;
            let work = root.join("../target/harness/matiec");
            let mut ok = true;
            for case in &cases {
                match matiec_trace(root, &dir, &work.join(&case.name), case) {
                    Ok(got) => ok &= compare(case, &case.dir.join(format!("{}.trace", case.name)), &got, opts.bless)?,
                    Err(e) => {
                        println!("{}: FAILED: {e}", case.name);
                        ok = false;
                    }
                }
            }
            Ok(ok)
        }
        _ => Err(format!("unknown side {side}: matiec")),
    }
}

struct Case {
    name: String,
    dir: PathBuf,
    script: String,
}

/// Every `cases/<name>.st` with its `<name>.cycles`, or those named.
fn find_cases(dir: &Path, wanted: &[String]) -> Result<Vec<Case>, String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str()?.strip_suffix(".st").map(str::to_owned))
        .collect();
    names.sort();
    if let Some(w) = wanted.iter().find(|w| !names.contains(w)) {
        return Err(format!("no case {w} in {}", dir.display()));
    }
    names
        .into_iter()
        .filter(|n| wanted.is_empty() || wanted.contains(n))
        .map(|name| {
            let cycles = dir.join(format!("{name}.cycles"));
            let text = fs::read_to_string(&cycles).map_err(|e| format!("{}: {e}", cycles.display()))?;
            let script = host_script(&text).map_err(|e| format!("{}: {e}", cycles.display()))?;
            Ok(Case { name, dir: dir.to_owned(), script })
        })
        .collect()
}

/// A `.cycles` file to the hosts' form. The file:
///
/// ```text
/// # a comment
/// ticks 6                 # how many ticks to run
/// 0 %IB0 16#03            # at tick 0, before it runs, %IB0 := 16#03
/// 3 %IX1.2 TRUE
/// 4 %IW2 -5
/// ```
///
/// The hosts' form: `ticks N`, then `set TICK OFFSET HEXBYTES`, the bytes
/// little-endian and the address numbers counting bytes, as
/// docs/c-output.md §4 has them. A bit is set within the byte as the
/// script has left it.
fn host_script(text: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut image = vec![0u8; 4096];
    let mut last = 0u64;
    let mut ticks = None;
    for (n, line) in text.lines().enumerate() {
        // A comment is a `#` at the start or after a space; `16#FF` keeps its own.
        let line = if line.trim_start().starts_with('#') { "" } else { line.split(" #").next().unwrap_or("").trim() };
        if line.is_empty() {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let err = |m: &str| format!("line {}: {m}", n + 1);
        match words.as_slice() {
            ["ticks", count] => ticks = Some(count.parse::<u64>().map_err(|_| err("ticks: not a number"))?),
            [tick, addr, value] => {
                let tick: u64 = tick.parse().map_err(|_| err("not a tick number"))?;
                if tick < last {
                    return Err(err("ticks out of order"));
                }
                last = tick;
                let (size, byte, bit) =
                    parse_address(addr).ok_or_else(|| err("not an input address (%IX0.0, %IB0, %IW0, %ID0, %IL0)"))?;
                if byte + size > image.len() {
                    return Err(err("address beyond 4096 bytes"));
                }
                let v = parse_value(value).ok_or_else(|| err("not a value (TRUE, 12, -3, 16#FF, 2#101)"))?;
                match bit {
                    Some(b) => {
                        if v > 1 {
                            return Err(err("a bit takes 0, 1, TRUE or FALSE"));
                        }
                        image[byte] = (image[byte] & !(1 << b)) | ((v as u8) << b);
                    }
                    None => {
                        if size < 8 && v >> (8 * size) != 0 && (v as i64) >> (8 * size - 1) != -1 {
                            return Err(err("value wider than the address"));
                        }
                        image[byte..byte + size].copy_from_slice(&v.to_le_bytes()[..size]);
                    }
                }
                let hex: String = image[byte..byte + size].iter().fold(String::new(), |mut s, b| {
                    let _ = write!(s, "{b:02x}");
                    s
                });
                let _ = writeln!(out, "set {tick} {byte} {hex}");
            }
            _ => return Err(err("expected 'ticks N' or 'TICK ADDRESS VALUE'")),
        }
    }
    let ticks = ticks.ok_or("no 'ticks N' line")?;
    if last >= ticks && !out.is_empty() {
        return Err(format!("an input set at tick {last}, but only {ticks} ticks run"));
    }
    Ok(format!("ticks {ticks}\n{out}"))
}

/// `%IX3.5` → (1, 3, Some(5)); `%IW2` → (2, 2, None).
fn parse_address(a: &str) -> Option<(usize, usize, Option<u8>)> {
    let rest = a.strip_prefix("%I")?;
    let (size, num) = match rest.chars().next()? {
        'X' => (1, &rest[1..]),
        'B' => (1, &rest[1..]),
        'W' => (2, &rest[1..]),
        'D' => (4, &rest[1..]),
        'L' => (8, &rest[1..]),
        _ => return None,
    };
    if rest.starts_with('X') {
        let (byte, bit) = num.split_once('.')?;
        let bit: u8 = bit.parse().ok().filter(|b| *b < 8)?;
        Some((1, byte.parse().ok()?, Some(bit)))
    } else {
        Some((size, num.parse().ok()?, None))
    }
}

fn parse_value(v: &str) -> Option<u64> {
    let v = v.replace('_', "");
    match v.as_str() {
        "TRUE" => return Some(1),
        "FALSE" => return Some(0),
        _ => {}
    }
    for (prefix, radix) in [("16#", 16), ("8#", 8), ("2#", 2)] {
        if let Some(d) = v.strip_prefix(prefix) {
            return u64::from_str_radix(d, radix).ok();
        }
    }
    match v.strip_prefix('-') {
        Some(d) => d.parse::<i64>().ok().map(|n| (-n) as u64),
        None => v.parse().ok(),
    }
}

/// Compile the case with iec2c and the host, run it, return its trace.
fn matiec_trace(root: &Path, matiec: &Path, work: &Path, case: &Case) -> Result<String, String> {
    let _ = fs::remove_dir_all(work);
    fs::create_dir_all(work).map_err(|e| format!("{}: {e}", work.display()))?;
    let st = fs::canonicalize(case.dir.join(format!("{}.st", case.name))).map_err(|e| e.to_string())?;
    let iec2c = matiec.join("iec2c");
    let lib = matiec.join("lib");
    let out = Command::new(&iec2c)
        .arg("-I")
        .arg(&lib)
        .arg(&st)
        .current_dir(work)
        .output()
        .map_err(|e| format!("{}: {e}", iec2c.display()))?;
    if !out.status.success() {
        return Err(format!("iec2c\n{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)));
    }
    // Every .c iec2c wrote but POUS.c, which the others include.
    let mut sources: Vec<PathBuf> = fs::read_dir(work)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "c") && !p.ends_with("POUS.c"))
        .collect();
    sources.sort();
    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
    let exe = work.join("host");
    let host = fs::canonicalize(root.join("matiec_host.c")).map_err(|e| e.to_string())?;
    let fixes = fs::canonicalize(root.join("matiec_fixes.h")).map_err(|e| e.to_string())?;
    let out = Command::new(&cc)
        // matiec's C and lib/C are not warning-clean; the host is GNU C.
        .args(["-w", "-O0", "-I", "."])
        .arg("-include")
        .arg(&fixes)
        .arg("-I")
        .arg(lib.join("C"))
        .arg(&host)
        .args(&sources)
        .args(["-lm", "-o"])
        .arg(&exe)
        .current_dir(work)
        .output()
        .map_err(|e| format!("{cc}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{cc}\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    run_host(&exe, &case.script)
}

fn run_host(exe: &Path, script: &str) -> Result<String, String> {
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    child.stdin.take().ok_or("no stdin")?.write_all(script.as_bytes()).map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("the host\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}

/// Compare with the trace on disk, or write it with `--bless`.
fn compare(case: &Case, path: &Path, got: &str, bless: bool) -> Result<bool, String> {
    if bless {
        fs::write(path, got).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}: blessed", case.name);
        return Ok(true);
    }
    let want =
        fs::read_to_string(path).map_err(|e| format!("{}: {e} (run with --bless to write it)", path.display()))?;
    if want == got {
        println!("{}: ok", case.name);
        return Ok(true);
    }
    println!("{}: DIFFERS from {}", case.name, path.display());
    for (w, g) in want.lines().zip(got.lines()).filter(|(w, g)| w != g).take(5) {
        println!("  want {w}\n  got  {g}");
    }
    let (w, g) = (want.lines().count(), got.lines().count());
    if w != g {
        println!("  {w} lines wanted, {g} got");
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts() {
        let s = host_script("ticks 4\n0 %IB0 16#03 # three\n1 %IX0.7 TRUE\n2 %IW2 -2\n3 %ID4 2#101\n").unwrap();
        assert_eq!(s, "ticks 4\nset 0 0 03\nset 1 0 83\nset 2 2 feff\nset 3 4 05000000\n");
        assert!(host_script("ticks 2\n2 %IB0 1\n").is_err());
        assert!(host_script("ticks 3\n1 %IB0 256\n").is_err());
        assert!(host_script("ticks 3\n1 %IB0 -128\n").is_ok());
        assert!(host_script("ticks 3\n2 %IB0 1\n1 %IB0 1\n").is_err());
        assert!(host_script("ticks 3\n1 %QB0 1\n").is_err());
    }

    /// Every case is IEC 61131-3 as iecf reads it, with no extension and
    /// no error (a deprecated form warns): a case only matiec accepts
    /// could never be run on iecf's side.
    #[test]
    fn cases_check_clean() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("cases");
        let cases = find_cases(&dir, &[]).unwrap();
        assert!(cases.len() >= 10);
        for case in cases {
            let name = format!("{}.st", case.name);
            let text = fs::read_to_string(dir.join(&name)).unwrap();
            let report = iecf_driver::check([(name, text)], &iecf_driver::Options::default());
            assert!(!report.has_errors(), "{}", report.to_short());
        }
    }
}
