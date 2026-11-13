//! The symbol table (docs/c-output.md §6): `<p>_symbols`, one entry per
//! top-level variable - a global, a program instance's variable - and
//! `<p>_types`, each type they use described once, as Beckhoff's ADS
//! and CODESYS's symbol configuration describe theirs. A platform finds
//! `R.P.PTS[2].X` by the symbol `R.P.PTS`, then the array's element
//! type, then the structure's member. Offsets and sizes are the C
//! compiler's own, `offsetof` and `sizeof`, right for whatever target
//! built the C.

use crate::derived::{DTy, Kind};
use crate::names::StdFb;
use crate::{Gen, c_type};
use iecf_sema::stdlib::Dir;
use iecf_sema::types::Elem;
use std::collections::HashMap;
use std::fmt::Write as _;

/// A type as the table describes it.
#[derive(Clone, Debug)]
pub(crate) enum SymTy {
    Elem(Elem),
    Enum(String, Elem),
    Struct(String),
    Array(Box<SymTy>, Vec<(i64, i64)>),
    /// `STRING(n)`; `true`: `WSTRING(n)`.
    Str(u32, bool),
    /// The user's FUNCTION_BLOCK, by its upper-cased name.
    Block(String),
    /// A standard function block, by its stem in runtime/iecf.h.
    Std(String),
    /// `REF_TO` the type; `true`: `REFERENCE TO` it (reference-to).
    Ref(bool, Box<SymTy>),
    /// An interface: never in the table (docs/c-output.md §6), nor an
    /// array of them, nor a structure's field of one (`hidden`).
    Itf,
}

impl SymTy {
    pub(crate) fn of(t: &DTy) -> SymTy {
        match &t.kind {
            Kind::Elem(e) => SymTy::Elem(*e),
            Kind::Enum(k, b) => SymTy::Enum(k.clone(), *b),
            Kind::Struct(k) => SymTy::Struct(k.clone()),
            Kind::Array(e, b) => SymTy::Array(Box::new(SymTy::of(e)), b.clone()),
            Kind::Str(n, w) => SymTy::Str(*n, *w),
            Kind::Block(k) => SymTy::Block(k.clone()),
            Kind::Std(fb) => SymTy::Std(fb.stem.clone()),
            // An ARRAY[*] is an in-out, never in the table either.
            Kind::Itf(_) | Kind::Flex(..) => SymTy::Itf,
            Kind::Ref(to, implicit) => SymTy::Ref(
                *implicit,
                Box::new(match &**to {
                    crate::derived::RefTo::Data(d) => SymTy::of(d),
                    crate::derived::RefTo::Block(k) => SymTy::Block(k.clone()),
                    crate::derived::RefTo::Std(fb) => SymTy::Std(fb.stem.clone()),
                }),
            ),
        }
    }

    /// Whether it is left out of the table: an interface, an array of
    /// them, or a reference to one.
    pub(crate) fn hidden(&self) -> bool {
        match self {
            SymTy::Itf => true,
            SymTy::Array(e, _) | SymTy::Ref(_, e) => e.hidden(),
            _ => false,
        }
    }

    /// Its name in the table, which also tells types apart.
    fn name(&self) -> String {
        match self {
            SymTy::Itf => unreachable!("an interface is left out of the table"),
            SymTy::Elem(e) => e.name().to_string(),
            SymTy::Str(n, false) => format!("STRING({n})"),
            SymTy::Str(n, true) => format!("WSTRING({n})"),
            SymTy::Enum(k, _) | SymTy::Struct(k) | SymTy::Block(k) | SymTy::Std(k) => k.clone(),
            SymTy::Ref(false, t) => format!("REF_TO {}", t.name()),
            SymTy::Ref(true, t) => format!("REFERENCE TO {}", t.name()),
            SymTy::Array(e, b) => {
                let dims: Vec<String> = b.iter().map(|(lo, hi)| format!("{lo}..{hi}")).collect();
                format!("ARRAY [{}] OF {}", dims.join(", "), e.name())
            }
        }
    }

    /// Its size, as a C constant expression.
    fn size(&self) -> String {
        match self {
            SymTy::Elem(e) | SymTy::Enum(_, e) => format!("sizeof({})", c_type(*e)),
            SymTy::Struct(k) => format!("sizeof(T_{k})"),
            SymTy::Block(k) => format!("sizeof(B_{k}_t)"),
            SymTy::Std(s) => format!("sizeof(iecf_{s}_t)"),
            SymTy::Str(n, false) => format!("{}u", *n as u64 + 1),
            SymTy::Str(n, true) => format!("{}u", (*n as u64 + 1) * 2),
            SymTy::Ref(..) => "sizeof(iecf_ref_t)".into(),
            SymTy::Itf => unreachable!("an interface is left out of the table"),
            SymTy::Array(e, b) => {
                let n: i128 = b.iter().map(|(lo, hi)| (*hi as i128) - (*lo as i128) + 1).product();
                format!("({n}u * {})", e.size())
            }
        }
    }
}

/// A variable the table lists, or a member of a type it describes.
#[derive(Clone, Debug)]
pub(crate) struct Member {
    /// Upper-cased.
    pub(crate) name: String,
    pub(crate) ty: SymTy,
    pub(crate) at: At,
    pub(crate) flags: u16,
}

#[derive(Clone, Debug)]
pub(crate) enum At {
    /// A field of the containing C structure.
    Field(String),
    /// In the process image: area, byte, and the bit of a BOOL at a bit.
    Image(char, u32, Option<u8>),
    /// In a structure with relative locations: its byte, and the bit of
    /// a BOOL at a bit.
    Laid(u64, Option<u8>),
    /// A field of the context itself, whatever contains the variable: a
    /// hierarchical address's slot, `H_<k>`.
    Ctx(String),
}

pub(crate) const INPUT: u16 = 1;
pub(crate) const OUTPUT: u16 = 2;
pub(crate) const CONSTANT: u16 = 4;
pub(crate) const RETAIN: u16 = 8;
/// An array, structure or STRING at an address: its parts back to back,
/// not at the offsets of the type table (docs/c-output.md §6).
pub(crate) const PACKED: u16 = 16;
/// Declared at a `*` address (Ed.3 6.5.5.4): the member is the
/// `iecf_ref_t` VAR_CONFIG gave the instance, its type a reference to
/// the variable's, and the variable is in the image where it points.
pub(crate) const PARTLY: u16 = 32;

/// A variable at a `*` address, `k`, of type `t` (config.rs): its
/// field `A_<K>`, the address VAR_CONFIG gives it. Never in ctx->retain,
/// RETAIN or not: the image, which keeps it, is the platform's.
pub(crate) fn partly(k: &str, t: &DTy, flags: u16) -> Member {
    Member {
        name: k.to_string(),
        ty: SymTy::Ref(false, Box::new(SymTy::of(t))),
        at: At::Field(format!("A_{k}")),
        flags: (flags & !RETAIN) | PARTLY,
    }
}

/// A variable for the table: `field` its C field in its structure.
pub(crate) fn member(name: &str, field: &str, place: &crate::names::Place, flags: u16) -> Member {
    use crate::names::Place;
    let (ty, at) = match place {
        Place::Field(_, e) => (SymTy::Elem(*e), At::Field(field.to_string())),
        Place::Located { area, size, byte, bit, ty } => {
            let bit = (*size == 'X').then_some(*bit);
            (SymTy::Elem(*ty), At::Image(*area, *byte, bit))
        }
        Place::Fb(_, fb) => (SymTy::Std(fb.stem.clone()), At::Field(field.to_string())),
        Place::Block(_, k) => (SymTy::Block(k.clone()), At::Field(field.to_string())),
        Place::Itf(..) => unreachable!("an interface is left out of the symbol table (docs/c-output.md §6)"),
        // At a hierarchical address: its slot in the context, a
        // reference, as a `*` variable's is (docs/c-output.md §6).
        Place::Data(crate::names::Lv::Ref(r), t) if r.starts_with("ctx->H_") => {
            return Member {
                name: name.to_string(),
                ty: SymTy::Ref(false, Box::new(SymTy::of(t))),
                at: At::Ctx(r["ctx->".len()..].to_string()),
                flags: (flags & !RETAIN) | PARTLY,
            };
        }
        Place::Data(
            crate::names::Lv::C(_)
            | crate::names::Lv::Ref(_)
            | crate::names::Lv::Part { .. }
            | crate::names::Lv::Bytes { area: crate::names::Area::Mem(_), .. },
            t,
        ) => (SymTy::of(t), At::Field(field.to_string())),
        Place::Data(crate::names::Lv::Bytes { area: crate::names::Area::Image(area), off, .. }, t) => {
            let byte = off.trim_end_matches('u').parse().unwrap_or(0);
            let packed = if t.scalar().is_none() { PACKED } else { 0 };
            return Member {
                name: name.to_string(),
                ty: SymTy::of(t),
                at: At::Image(*area, byte, None),
                flags: flags | packed,
            };
        }
    };
    Member { name: name.to_string(), ty, at, flags }
}

pub(crate) fn dir_flags(d: Option<Dir>) -> u16 {
    match d {
        Some(Dir::Input) => INPUT,
        Some(Dir::Output) => OUTPUT,
        _ => 0,
    }
}

/// The tables being written.
#[derive(Default)]
pub(crate) struct Tables {
    index: HashMap<String, u32>,
    entries: Vec<String>,
    /// Each entry as JSON, for `<p>.symbols.json` (docs/c-output.md §6).
    json: Vec<String>,
    /// Member and bound arrays the entries point to.
    arrays: String,
}

fn elem_code(e: Elem) -> String {
    format!("IECF_T_{}", e.name())
}

/// A JSON string: the names are IEC identifiers and type names, ASCII.
fn js(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn json_flags(flags: u16) -> String {
    let names: Vec<String> = [
        (INPUT, "INPUT"),
        (OUTPUT, "OUTPUT"),
        (CONSTANT, "CONSTANT"),
        (RETAIN, "RETAIN"),
        (PACKED, "PACKED"),
        (PARTLY, "PARTLY"),
    ]
    .iter()
    .filter(|(b, _)| flags & b != 0)
    .map(|(_, n)| js(n))
    .collect();
    format!("[{}]", names.join(", "))
}

impl Gen<'_> {
    /// The index of `t` in `<p>_types`, described once.
    fn type_index(&mut self, t: &SymTy, tb: &mut Tables) -> u32 {
        let key = t.name();
        if let Some(i) = tb.index.get(&key) {
            return *i;
        }
        let me = tb.entries.len() as u32;
        tb.index.insert(key.clone(), me);
        // A namespace's type by its path as written, `Plant.POINT`.
        let name = iecf_sema::namespaces::display(&key);
        tb.entries.push(String::new()); // filled below, after what it refers to
        tb.json.push(String::new());
        let size = format!("(uint32_t){}", t.size());
        let entry = match t {
            SymTy::Itf => unreachable!("an interface is left out of the table"),
            SymTy::Elem(e) => {
                tb.json[me as usize] = format!("{{\"name\": {}, \"kind\": \"elementary\"}}", js(&name));
                format!("{{\"{name}\", IECF_KIND_ELEM, {}, {size}, 0u, 0u, NULL, 0u, NULL}}", elem_code(*e))
            }
            SymTy::Str(n, w) => {
                let (kind, code) = if *w { ("wstring", "IECF_T_WSTRING") } else { ("string", "IECF_T_STRING") };
                tb.json[me as usize] = format!("{{\"name\": {}, \"kind\": \"{kind}\", \"length\": {n}}}", js(&name));
                format!("{{\"{name}\", IECF_KIND_ELEM, {code}, {size}, 0u, 0u, NULL, 0u, NULL}}")
            }
            SymTy::Enum(k, b) => {
                let base = self.type_index(&SymTy::Elem(*b), tb);
                let values = self.enums.get(k).map(|i| i.values.clone()).unwrap_or_default();
                let list: Vec<String> = values
                    .iter()
                    .map(|(v, n)| format!("{{\"{v}\", {base}u, 0u, 0u, INT64_C({n}), IECF_NO_BIT}}"))
                    .collect();
                let arr = format!("iecf_members_{me}");
                let _ = writeln!(tb.arrays, "static const iecf_member_t {arr}[] = {{{}}};", list.join(", "));
                let vj: Vec<String> = values.iter().map(|(v, n)| format!("[{}, {n}]", js(v))).collect();
                tb.json[me as usize] = format!(
                    "{{\"name\": {}, \"kind\": \"enum\", \"base\": {base}, \"values\": [{}]}}",
                    js(&name),
                    vj.join(", ")
                );
                format!(
                    "{{\"{name}\", IECF_KIND_ENUM, {}, {size}, 0u, 0u, NULL, {}u, {arr}}}",
                    elem_code(*b),
                    list.len()
                )
            }
            SymTy::Struct(k) => {
                let fields = self.structs.get(k).cloned().unwrap_or_default();
                let layout = self.layouts.get(k).cloned();
                let stars = self.star_fields.get(k).cloned().unwrap_or_default();
                let members: Vec<Member> = fields
                    .iter()
                    .map(|(n, ft)| match layout.as_ref().and_then(|l| l.at.get(n)) {
                        _ if stars.contains_key(n) => partly(n, ft, 0),
                        // Its parts back to back from its byte.
                        Some((byte, bit)) => Member {
                            name: n.clone(),
                            ty: SymTy::of(ft),
                            at: At::Laid(*byte, *bit),
                            flags: if ft.scalar().is_some() { 0 } else { PACKED },
                        },
                        None => {
                            Member { name: n.clone(), ty: SymTy::of(ft), at: At::Field(format!("V_{n}")), flags: 0 }
                        }
                    })
                    .collect();
                let (kind, kind_json) =
                    if layout.is_some() { ("IECF_KIND_LAYOUT", "layout") } else { ("IECF_KIND_STRUCT", "struct") };
                self.members_entry(&name, kind, &format!("T_{k}"), &size, me, &members, tb, kind_json)
            }
            SymTy::Block(k) => {
                let members = self.block_members.get(k).cloned().unwrap_or_default();
                self.members_entry(&name, "IECF_KIND_BLOCK", &format!("B_{k}_t"), &size, me, &members, tb, "block")
            }
            SymTy::Std(s) => {
                let members: Vec<Member> = StdFb::named(s)
                    .map(|fb| {
                        fb.params
                            .iter()
                            .map(|(n, e, d)| Member {
                                name: n.clone(),
                                ty: SymTy::Elem(*e),
                                at: At::Field(n.clone()),
                                flags: dir_flags(Some(*d)),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.members_entry(&name, "IECF_KIND_BLOCK", &format!("iecf_{s}_t"), &size, me, &members, tb, "block")
            }
            SymTy::Ref(_, to) => {
                let of = self.type_index(to, tb);
                tb.json[me as usize] = format!("{{\"name\": {}, \"kind\": \"reference\", \"of\": {of}}}", js(&name));
                format!("{{\"{name}\", IECF_KIND_REF, 0u, {size}, {of}u, 0u, NULL, 0u, NULL}}")
            }
            SymTy::Array(e, b) => {
                let of = self.type_index(e, tb);
                let arr = format!("iecf_bounds_{me}");
                let list: Vec<String> = b.iter().map(|(lo, hi)| format!("INT64_C({lo}), INT64_C({hi})")).collect();
                let _ = writeln!(tb.arrays, "static const int64_t {arr}[] = {{{}}};", list.join(", "));
                let dj: Vec<String> = b.iter().map(|(lo, hi)| format!("[{lo}, {hi}]")).collect();
                tb.json[me as usize] = format!(
                    "{{\"name\": {}, \"kind\": \"array\", \"of\": {of}, \"dims\": [{}]}}",
                    js(&name),
                    dj.join(", ")
                );
                format!("{{\"{name}\", IECF_KIND_ARRAY, 0u, {size}, {of}u, {}u, {arr}, 0u, NULL}}", b.len())
            }
        };
        tb.entries[me as usize] = entry;
        me
    }

    #[allow(clippy::too_many_arguments)]
    fn members_entry(
        &mut self,
        name: &str,
        kind: &str,
        c: &str,
        size: &str,
        me: u32,
        members: &[Member],
        tb: &mut Tables,
        kind_json: &str,
    ) -> String {
        let mut list = Vec::new();
        let mut mj = Vec::new();
        for m in members.iter().filter(|m| !m.ty.hidden()) {
            let (offset, bit, at) = match &m.at {
                At::Field(f) => (format!("(uint32_t)offsetof({c}, {f})"), "IECF_NO_BIT".to_string(), String::new()),
                At::Laid(byte, Some(b)) => {
                    (format!("{byte}u"), format!("{b}u"), format!(", \"byte\": {byte}, \"bit\": {b}"))
                }
                At::Laid(byte, None) => (format!("{byte}u"), "IECF_NO_BIT".to_string(), format!(", \"byte\": {byte}")),
                At::Image(..) | At::Ctx(_) => continue,
            };
            let t = self.type_index(&m.ty, tb);
            list.push(format!("{{\"{}\", {t}u, {offset}, {}u, 0, {bit}}}", m.name, m.flags));
            mj.push(format!("{{\"name\": {}, \"type\": {t}, \"flags\": {}{at}}}", js(&m.name), json_flags(m.flags)));
        }
        tb.json[me as usize] =
            format!("{{\"name\": {}, \"kind\": \"{kind_json}\", \"members\": [{}]}}", js(name), mj.join(", "));
        if list.is_empty() {
            return format!("{{\"{name}\", {kind}, 0u, {size}, 0u, 0u, NULL, 0u, NULL}}");
        }
        let arr = format!("iecf_members_{me}");
        let _ = writeln!(tb.arrays, "static const iecf_member_t {arr}[] = {{\n    {}\n}};", list.join(",\n    "));
        format!("{{\"{name}\", {kind}, 0u, {size}, 0u, 0u, NULL, {}u, {arr}}}", list.len())
    }

    /// `<p>_types` and `<p>_symbols` for `symbols`: (path, variable, the
    /// C structure it is a field of, and that structure's offset in the
    /// context as C: `""` for the context itself).
    /// `access`: the VAR_ACCESS entries - name, the variable's path,
    /// its type, READ_WRITE.
    pub(crate) fn symbol_tables(
        &mut self,
        symbols: &[(String, Member, String, String)],
        access: &[(String, String, String, bool)],
    ) -> String {
        let p = self.prefix.clone();
        let mut tb = Tables::default();
        let mut rows = Vec::new();
        let mut json_rows = Vec::new();
        for (path, m, container, base) in symbols.iter().filter(|(_, m, ..)| !m.ty.hidden()) {
            let t = self.type_index(&m.ty, &mut tb);
            let (offset, area, bit) = match &m.at {
                At::Field(f) => (format!("(uint32_t)({base}offsetof({container}, {f}))"), "0".to_string(), "0u".into()),
                At::Image(a, byte, Some(bit)) => (format!("{byte}u"), format!("'{a}'"), format!("{bit}u")),
                At::Image(a, byte, None) => (format!("{byte}u"), format!("'{a}'"), "IECF_NO_BIT".into()),
                At::Laid(..) => unreachable!("a variable is in the context or the image; only a member is laid"),
                At::Ctx(f) => (format!("(uint32_t)(offsetof({p}_t, {f}))"), "0".to_string(), "0u".into()),
            };
            rows.push(format!("{{\"{path}\", {t}u, {offset}, {area}, {bit}, {}u}}", m.flags));
            let at = match &m.at {
                At::Field(_) | At::Laid(..) | At::Ctx(_) => String::new(),
                At::Image(a, byte, Some(bit)) => format!(", \"area\": \"{a}\", \"byte\": {byte}, \"bit\": {bit}"),
                At::Image(a, byte, None) => format!(", \"area\": \"{a}\", \"byte\": {byte}"),
            };
            json_rows.push(format!(
                "    {{\"path\": {}, \"type\": {t}, \"flags\": {}{at}}}",
                js(path),
                json_flags(m.flags)
            ));
        }
        let access_json: Vec<String> = access
            .iter()
            .map(|(name, path, ty, rw)| {
                format!(
                    "    {{\"name\": {}, \"path\": {}, \"type\": {}, \"direction\": \"{}\"}}",
                    js(name),
                    js(path),
                    js(ty),
                    if *rw { "READ_WRITE" } else { "READ_ONLY" }
                )
            })
            .collect();
        // A debug build: the lines it stops at, with their POU and file.
        let debug = if self.debug {
            let lines: Vec<String> = self
                .debug_lines
                .iter()
                .map(|(pou, line, file)| {
                    let name = self.pou_names.get(*pou as usize).map_or("", String::as_str);
                    format!(
                        "    {{\"pou\": {}, \"file\": {}, \"line\": {line}}}",
                        js(&iecf_sema::namespaces::display(name)),
                        js(file)
                    )
                })
                .collect();
            format!(",\n  \"debug\": true,\n  \"lines\": [\n{}\n  ]", lines.join(",\n"))
        } else {
            String::new()
        };
        self.symbols_json = format!(
            "{{\n  \"configuration\": {},\n  \"symbols\": [\n{}\n  ],\n  \"types\": [\n{}\n  ],\n  \"access\": [\n{}\n  ]{debug}\n}}\n",
            js(&p),
            json_rows.join(",\n"),
            tb.json.iter().map(|j| format!("    {j}")).collect::<Vec<_>>().join(",\n"),
            access_json.join(",\n")
        );
        let mut c = String::new();
        let _ = writeln!(c, "/* The symbol table (docs/c-output.md §6). */");
        c.push_str(&tb.arrays);
        if tb.entries.is_empty() {
            tb.entries.push("{NULL, 0u, 0u, 0u, 0u, 0u, NULL, 0u, NULL}".into());
            let _ = writeln!(c, "const iecf_type_t {p}_types[] = {{{}}};", tb.entries[0]);
            let _ = writeln!(c, "const uint32_t {p}_type_count = 0u;");
        } else {
            let _ = writeln!(c, "const iecf_type_t {p}_types[] = {{\n    {}\n}};", tb.entries.join(",\n    "));
            let _ = writeln!(c, "const uint32_t {p}_type_count = {}u;", tb.entries.len());
        }
        if rows.is_empty() {
            // C has no empty array: one entry that names nothing.
            let _ = writeln!(c, "const iecf_symbol_t {p}_symbols[] = {{{{NULL, 0u, 0u, 0, 0u, 0u}}}};");
            let _ = writeln!(c, "const uint32_t {p}_symbol_count = 0u;");
        } else {
            let _ = writeln!(c, "const iecf_symbol_t {p}_symbols[] = {{\n    {}\n}};", rows.join(",\n    "));
            let _ = writeln!(c, "const uint32_t {p}_symbol_count = {}u;", rows.len());
        }
        // The access paths (Ed.3 6.8.1, VAR_ACCESS): each name, the
        // variable it reaches as a path of the symbol table, and whether
        // a communication service may write it.
        if access.is_empty() {
            let _ = writeln!(c, "const iecf_access_t {p}_access[] = {{{{NULL, NULL, 0u}}}};");
            let _ = writeln!(c, "const uint32_t {p}_access_count = 0u;");
        } else {
            let rows: Vec<String> = access
                .iter()
                .map(|(name, path, _, rw)| {
                    format!("{{\"{name}\", \"{path}\", {}}}", if *rw { "IECF_ACCESS_WRITE" } else { "0u" })
                })
                .collect();
            let _ = writeln!(c, "const iecf_access_t {p}_access[] = {{\n    {}\n}};", rows.join(",\n    "));
            let _ = writeln!(c, "const uint32_t {p}_access_count = {}u;", rows.len());
        }
        c
    }
}
