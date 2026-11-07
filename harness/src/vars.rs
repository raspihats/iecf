//! `vars.c` for the matiec host: `harness_vars()`, printing each
//! elementary variable of matiec's VARIABLES.csv in the form iecf_host.c
//! prints iecf's from the symbol table - `  PATH VALUE`, the path without
//! the configuration's name, as iecf's symbols have it (R.P.X, G, R.G).
//!
//! The C names are matiec's: an instance P of resource R is `R__P`, its
//! variable X `R__P.X`; a global G of domain D (the configuration or a
//! resource) is `D__G`. Each is a struct whose `.value` is the value, or
//! a pointer to it for a located variable (`__DECLARE_LOCATED`,
//! `__DECLARE_GLOBAL_LOCATED`), which the generated files say.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

const ELEMENTARY: [&str; 21] = [
    "BOOL",
    "SINT",
    "INT",
    "DINT",
    "LINT",
    "USINT",
    "UINT",
    "UDINT",
    "ULINT",
    "BYTE",
    "WORD",
    "DWORD",
    "LWORD",
    "REAL",
    "LREAL",
    "TIME",
    "DATE",
    "TOD",
    "TIME_OF_DAY",
    "DT",
    "STRING",
];

/// Write `vars.c` into `work`, where iec2c wrote its files.
pub(crate) fn write_matiec_vars(work: &Path) -> Result<(), String> {
    let csv = fs::read_to_string(work.join("VARIABLES.csv")).unwrap_or_default();
    // Every C file iec2c wrote, to find which declarations are located.
    let mut c_text = String::new();
    for e in fs::read_dir(work).map_err(|e| e.to_string())?.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "c" || x == "h") {
            c_text.push_str(&fs::read_to_string(&p).unwrap_or_default());
        }
    }
    let mut programs: Vec<(Vec<String>, String)> = Vec::new(); // path after the config, type
    let mut section = "";
    let mut body = String::new();
    let mut externs = String::new();
    for line in csv.lines() {
        if line.starts_with("//") {
            section = line.trim_start_matches('/').trim();
            continue;
        }
        let f: Vec<&str> = line.split(';').collect();
        match section {
            "Programs" if f.len() >= 3 => {
                let segs: Vec<String> = f[1].split('.').skip(1).map(str::to_owned).collect();
                if segs.len() == 2 {
                    let _ = writeln!(externs, "extern {}_data__ {}__{};", f[2], segs[0], segs[1]);
                }
                programs.push((segs, f[2].to_owned()));
            }
            "Variables" if f.len() >= 6 => {
                let (class, path, ty) = (f[1], f[2], f[4]);
                if class == "FB" || class == "EXT" || !ELEMENTARY.contains(&ty) {
                    continue;
                }
                let segs: Vec<&str> = path.split('.').collect();
                // SFC transitions are listed too (`IDLE->ACTIVE`): not variables.
                if segs.iter().any(|g| !g.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')) {
                    continue;
                }
                let config = segs[0];
                let rest = &segs[1..];
                let name = rest.join(".");
                let (c, located) = match rest {
                    // A configuration's global.
                    [g] => {
                        let located = c_text.contains(&format!("__DECLARE_GLOBAL_LOCATED({ty},{config},{g})"));
                        let kind = if located { "p" } else { "t" };
                        let _ = writeln!(externs, "extern __IEC_{ty}_{kind} {config}__{g};");
                        (format!("{config}__{g}"), located)
                    }
                    // A program instance's variable, or a resource's global.
                    [a, b] if !programs.iter().any(|(p, _)| p.len() == 2 && p[0] == *a && p[1] == *b) => {
                        let located = c_text.contains(&format!("__DECLARE_GLOBAL_LOCATED({ty},{a},{b})"));
                        let kind = if located { "p" } else { "t" };
                        let _ = writeln!(externs, "extern __IEC_{ty}_{kind} {a}__{b};");
                        (format!("{a}__{b}"), located)
                    }
                    [r, p, x] => {
                        let Some((_, pty)) = programs.iter().find(|(s, _)| s.len() == 2 && s[0] == *r && s[1] == *p)
                        else {
                            continue;
                        };
                        // Located: declared so in its program's structure.
                        let located = pou_struct(&c_text, pty).contains(&format!("__DECLARE_LOCATED({ty},{x})"));
                        (format!("{r}__{p}.{x}"), located)
                    }
                    _ => continue,
                };
                let v = if located { format!("(*{c}.value)") } else { format!("{c}.value") };
                let print = match ty {
                    "BOOL" => format!("printf(\"%u\", (unsigned)({v} != 0));"),
                    "SINT" | "INT" | "DINT" | "LINT" => format!("printf(\"%lld\", (long long){v});"),
                    "REAL" => format!("printf(\"%.9g\", (double){v});"),
                    "LREAL" => format!("printf(\"%.17g\", {v});"),
                    "TIME" | "TOD" | "TIME_OF_DAY" => format!("hv_ms({v});"),
                    "DATE" | "DT" => format!("hv_s({v});"),
                    "STRING" => format!("hv_string(&{v});"),
                    _ => format!("printf(\"%llu\", (unsigned long long){v});"),
                };
                let _ = writeln!(body, "    printf(\"  {name} \");\n    {print}\n    printf(\"\\n\");");
            }
            _ => {}
        }
    }
    let text = format!(
        "/* vars.c - written by iecf-harness from VARIABLES.csv (src/vars.rs). */\n\
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

/// The text of `typedef struct { ... } <TYPE>_data__;` in POUS.h.
fn pou_struct<'t>(c: &'t str, ty: &str) -> &'t str {
    let end = format!("}} {ty}_data__;");
    let Some(e) = c.find(&end) else { return "" };
    let start = c[..e].rfind("typedef struct").unwrap_or(0);
    &c[start..e]
}
