//! `vars.c` for the reference host: `harness_vars()`, printing every leaf of
//! every variable of iecf's symbol table (`<p>.symbols.json`) - array
//! elements, structure fields, block members - from the reference compiler's C, in the
//! form iecf_host.c prints iecf's: `  PATH VALUE`. One list of names for
//! both: what one compiler does not have, neither prints.
//!
//! The reference compiler's C names: an instance P of resource R is `R__P` (VARIABLES.csv
//! lists the instances); a global G of domain D, the configuration or a
//! resource, `D__G`. A variable, and a block's member, is a struct whose
//! `.value` holds the value - or points to it, for a located variable,
//! which the generated C declares `__DECLARE_LOCATED` or
//! `__DECLARE_GLOBAL_LOCATED`. A structure's fields and an array's
//! elements (`.table[i][j]`, from 0) are plain; a block instance is its
//! own struct of members.

use crate::json::{J, parse};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// As iecf_host.c's MAX_ELEMENTS: the first elements of a big array.
const MAX_ELEMENTS: u64 = 64;

/// Write `vars.c` into `work`, where iec2c wrote its files: the variables
/// of `symbols` (iecf's JSON), or none.
pub(crate) fn write_reference_vars(work: &Path, symbols: Option<&str>) -> Result<(), String> {
    let mut body = String::new();
    let mut externs = String::new();
    if let Some(text) = symbols {
        let json = parse(text)?;
        let csv = fs::read_to_string(work.join("VARIABLES.csv")).unwrap_or_default();
        let mut c_text = String::new();
        for e in fs::read_dir(work).map_err(|e| e.to_string())?.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "c" || x == "h") {
                c_text.push_str(&fs::read_to_string(&p).unwrap_or_default());
            }
        }
        // The program instances, `R.P` with their POU type, and the
        // configuration's name as the reference compiler writes it.
        let mut programs: Vec<(String, String)> = Vec::new();
        let mut config = String::new();
        let mut section = "";
        for line in csv.lines() {
            if line.starts_with("//") {
                section = line.trim_start_matches('/').trim();
                continue;
            }
            let f: Vec<&str> = line.split(';').collect();
            if section == "Programs" && f.len() >= 3 {
                let (c, rest) = f[1].split_once('.').unwrap_or(("", f[1]));
                config = c.to_owned();
                if let Some((r, p)) = rest.split_once('.') {
                    let _ = writeln!(externs, "extern {}_data__ {r}__{p};", f[2]);
                }
                programs.push((rest.to_owned(), f[2].to_owned()));
            }
        }
        let types = json.get("types").map(J::arr).unwrap_or(&[]);
        for s in json.get("symbols").map(J::arr).unwrap_or(&[]) {
            let (Some(path), Some(t)) = (s.get("path").and_then(J::str), s.get("type").and_then(J::num)) else {
                continue;
            };
            let segs: Vec<&str> = path.split('.').collect();
            let pou = if segs.len() == 3 {
                let key = format!("{}.{}", segs[0], segs[1]);
                programs.iter().find(|(p, _)| *p == key).map(|(_, t)| t.clone())
            } else {
                None
            };
            let (base, located) = if let Some(pty) = pou {
                let located = declared_located(pou_struct(&c_text, &pty), segs[2]);
                (format!("{}__{}.{}", segs[0], segs[1], segs[2]), located)
            } else {
                // A global: its type, as its __DECLARE_GLOBAL names it.
                let (domain, name) = match segs.as_slice() {
                    [g] => (config.as_str(), *g),
                    [r, g] => (*r, *g),
                    _ => continue,
                };
                let Some((mtype, located)) = global_decl(&c_text, domain, name) else { continue };
                let _ = writeln!(externs, "extern __IEC_{mtype}_{} {domain}__{name};", if located { "p" } else { "t" });
                (format!("{domain}__{name}"), located)
            };
            let (c, wrapped) = if located { (format!("(*{base}.value)"), false) } else { (base, true) };
            leaves(types, path, t as usize, &c, wrapped, &mut body);
        }
    }
    let text = format!(
        "/* vars.c - written by iecf-harness from iecf's symbol table (src/vars.rs). */\n\
         #include <stdio.h>\n#include \"POUS.h\"\n\n{externs}\n\
         /* A duration or time of day in ms; a remainder below a ms shown. */\n\
         static void hv_ms(IEC_TIMESPEC t) {{\n\
         \x20   long long ns = (long long)t.tv_sec * 1000000000LL + t.tv_nsec;\n\
         \x20   if (ns % 1000000LL == 0) printf(\"%lldms\", ns / 1000000LL);\n\
         \x20   else printf(\"%lldms+%lldns\", ns / 1000000LL, ns % 1000000LL);\n}}\n\
         static void hv_s(IEC_TIMESPEC t) {{\n\
         \x20   if (t.tv_nsec == 0) printf(\"%lds\", (long)t.tv_sec);\n\
         \x20   else printf(\"%lds+%ldns\", (long)t.tv_sec, (long)t.tv_nsec);\n}}\n\
         static void hv_string(const IEC_STRING *s) {{\n\
         \x20   unsigned k;\n\
         \x20   putchar('\\'');\n\
         \x20   for (k = 0; k < s->len; k++) {{\n\
         \x20       unsigned char c = s->body[k];\n\
         \x20       if (c >= 32 && c < 127 && c != '\\'' && c != '$') putchar(c); else printf(\"$%02X\", c);\n\
         \x20   }}\n\
         \x20   putchar('\\'');\n}}\n\n\
         void harness_vars(void) {{\n{body}}}\n"
    );
    fs::write(work.join("vars.c"), text).map_err(|e| e.to_string())
}

/// The reference compiler's type of global `name` of `domain`, and whether it is
/// located: `__DECLARE_GLOBAL(TYPE,DOMAIN,NAME)` or `..._LOCATED(...)`.
fn global_decl(c: &str, domain: &str, name: &str) -> Option<(String, bool)> {
    for (head, located) in [("__DECLARE_GLOBAL(", false), ("__DECLARE_GLOBAL_LOCATED(", true)] {
        for l in c.lines() {
            let Some(args) = l.trim().strip_prefix(head) else { continue };
            let parts: Vec<&str> = args.trim_end_matches(')').split(',').collect();
            if parts.len() == 3 && parts[1] == domain && parts[2] == name {
                return Some((parts[0].to_owned(), located));
            }
        }
    }
    None
}

/// Whether `__DECLARE_LOCATED(TYPE,NAME)` declares `name` in `st`.
fn declared_located(st: &str, name: &str) -> bool {
    st.lines().any(|l| {
        let l = l.trim();
        l.starts_with("__DECLARE_LOCATED(") && l.trim_end_matches(')').rsplit(',').next() == Some(name)
    })
}

/// The printing of each leaf under `path`, of type `t`, whose C is `c`:
/// `wrapped` when `c` is the reference compiler's struct holding `.value`.
fn leaves(types: &[J], path: &str, t: usize, c: &str, wrapped: bool, out: &mut String) {
    let Some(ty) = types.get(t) else { return };
    let content = if wrapped { format!("{c}.value") } else { c.to_owned() };
    let kind = ty.get("kind").and_then(J::str).unwrap_or("");
    match kind {
        "elementary" | "enum" | "string" => {
            let name = ty.get("name").and_then(J::str).unwrap_or("");
            let print = match (kind, name) {
                ("string", _) => format!("hv_string(&{content});"),
                ("enum", _) => format!("printf(\"%lld\", (long long){content});"),
                // The reference compiler has no L dates and holds an
                // LTIME otherwise: not compared.
                (_, "LTIME" | "LDATE" | "LTOD" | "LDT") => return,
                (_, "BOOL") => format!("printf(\"%u\", (unsigned)({content} != 0));"),
                (_, "SINT" | "INT" | "DINT" | "LINT") => format!("printf(\"%lld\", (long long){content});"),
                (_, "REAL") => format!("printf(\"%.9g\", (double){content});"),
                (_, "LREAL") => format!("printf(\"%.17g\", {content});"),
                (_, "TIME" | "TOD") => format!("hv_ms({content});"),
                (_, "DATE" | "DT") => format!("hv_s({content});"),
                _ => format!("printf(\"%llu\", (unsigned long long){content});"),
            };
            let _ = writeln!(out, "    printf(\"  {path} \");\n    {print}\n    printf(\"\\n\");");
        }
        // A reference: whether it references anything, as iecf_host.c.
        "reference" => {
            let _ = writeln!(out, "    printf(\"  {path} %s\\n\", {content} == NULL ? \"NULL\" : \"REF\");");
        }
        "struct" => {
            for m in ty.get("members").map(J::arr).unwrap_or(&[]) {
                let (Some(n), Some(mt)) = (m.get("name").and_then(J::str), m.get("type").and_then(J::num)) else {
                    continue;
                };
                leaves(types, &format!("{path}.{n}"), mt as usize, &format!("{content}.{n}"), false, out);
            }
        }
        // A block instance is its own struct; each member holds `.value`.
        "block" => {
            for m in ty.get("members").map(J::arr).unwrap_or(&[]) {
                let (Some(n), Some(mt)) = (m.get("name").and_then(J::str), m.get("type").and_then(J::num)) else {
                    continue;
                };
                leaves(types, &format!("{path}.{n}"), mt as usize, &format!("{c}.{n}"), true, out);
            }
        }
        "array" => {
            let dims: Vec<(i64, i64)> = ty
                .get("dims")
                .map(J::arr)
                .unwrap_or(&[])
                .iter()
                .filter_map(|d| Some((d.arr().first()?.num()?, d.arr().get(1)?.num()?)))
                .collect();
            let Some(of) = ty.get("of").and_then(J::num) else { return };
            let n: u64 = dims.iter().map(|(lo, hi)| (hi - lo + 1) as u64).product();
            for e in 0..n.min(MAX_ELEMENTS) {
                // Row-major, the last index fastest, as iecf_host.c.
                let mut rest = e;
                let mut idx = vec![0i64; dims.len()];
                for (d, (lo, hi)) in dims.iter().enumerate().rev() {
                    let w = (hi - lo + 1) as u64;
                    idx[d] = lo + (rest % w) as i64;
                    rest /= w;
                }
                let names: Vec<String> = idx.iter().map(i64::to_string).collect();
                let table: String = idx.iter().zip(&dims).map(|(i, (lo, _))| format!("[{}]", i - lo)).collect();
                let sub = format!("{path}[{}]", names.join(","));
                leaves(types, &sub, of as usize, &format!("{content}.table{table}"), false, out);
            }
        }
        _ => {}
    }
}

/// The text of `typedef struct { ... } <TYPE>_data__;` in POUS.h.
fn pou_struct<'t>(c: &'t str, ty: &str) -> &'t str {
    let end = format!("}} {ty}_data__;");
    let Some(e) = c.find(&end) else { return "" };
    let start = c[..e].rfind("typedef struct").unwrap_or(0);
    &c[start..e]
}
