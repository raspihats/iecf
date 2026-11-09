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
    /// `STRING(n)`.
    Str(u32),
    /// The user's FUNCTION_BLOCK, by its upper-cased name.
    Block(String),
    /// A standard function block, by its stem in runtime/iecf.h.
    Std(String),
    /// `REF_TO` the type; `true`: `REFERENCE TO` it (reference-to).
    Ref(bool, Box<SymTy>),
}

impl SymTy {
    pub(crate) fn of(t: &DTy) -> SymTy {
        match &t.kind {
            Kind::Elem(e) => SymTy::Elem(*e),
            Kind::Enum(k, b) => SymTy::Enum(k.clone(), *b),
            Kind::Struct(k) => SymTy::Struct(k.clone()),
            Kind::Array(e, b) => SymTy::Array(Box::new(SymTy::of(e)), b.clone()),
            Kind::Str(n) => SymTy::Str(*n),
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

    /// Its name in the table, which also tells types apart.
    fn name(&self) -> String {
        match self {
            SymTy::Elem(e) => e.name().to_string(),
            SymTy::Str(n) => format!("STRING({n})"),
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
            SymTy::Str(n) => format!("{}u", *n as u64 + 1),
            SymTy::Ref(..) => "sizeof(iecf_ref_t)".into(),
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
}

pub(crate) const INPUT: u16 = 1;
pub(crate) const OUTPUT: u16 = 2;
pub(crate) const CONSTANT: u16 = 4;
pub(crate) const RETAIN: u16 = 8;
/// An array, structure or STRING at an address: its parts back to back,
/// not at the offsets of the type table (docs/c-output.md §6).
pub(crate) const PACKED: u16 = 16;

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
        Place::Data(crate::names::Lv::C(_) | crate::names::Lv::Ref(_), t) => {
            (SymTy::of(t), At::Field(field.to_string()))
        }
        Place::Data(crate::names::Lv::Image { area, off }, t) => {
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
    let names: Vec<String> =
        [(INPUT, "INPUT"), (OUTPUT, "OUTPUT"), (CONSTANT, "CONSTANT"), (RETAIN, "RETAIN"), (PACKED, "PACKED")]
            .iter()
            .filter(|(b, _)| flags & b != 0)
            .map(|(_, n)| js(n))
            .collect();
    format!("[{}]", names.join(", "))
}

impl Gen<'_> {
    /// The index of `t` in `<p>_types`, described once.
    fn type_index(&mut self, t: &SymTy, tb: &mut Tables) -> u32 {
        let name = t.name();
        if let Some(i) = tb.index.get(&name) {
            return *i;
        }
        let me = tb.entries.len() as u32;
        tb.index.insert(name.clone(), me);
        tb.entries.push(String::new()); // filled below, after what it refers to
        tb.json.push(String::new());
        let size = format!("(uint32_t){}", t.size());
        let entry = match t {
            SymTy::Elem(e) => {
                tb.json[me as usize] = format!("{{\"name\": {}, \"kind\": \"elementary\"}}", js(&name));
                format!("{{\"{name}\", IECF_KIND_ELEM, {}, {size}, 0u, 0u, NULL, 0u, NULL}}", elem_code(*e))
            }
            SymTy::Str(n) => {
                tb.json[me as usize] = format!("{{\"name\": {}, \"kind\": \"string\", \"length\": {n}}}", js(&name));
                format!("{{\"{name}\", IECF_KIND_ELEM, IECF_T_STRING, {size}, 0u, 0u, NULL, 0u, NULL}}")
            }
            SymTy::Enum(k, b) => {
                let base = self.type_index(&SymTy::Elem(*b), tb);
                let values = self.enums.get(k).map(|i| i.values.clone()).unwrap_or_default();
                let list: Vec<String> =
                    values.iter().map(|(v, n)| format!("{{\"{v}\", {base}u, 0u, 0u, INT64_C({n})}}")).collect();
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
                let members: Vec<Member> = fields
                    .iter()
                    .map(|(n, ft)| Member {
                        name: n.clone(),
                        ty: SymTy::of(ft),
                        at: At::Field(format!("V_{n}")),
                        flags: 0,
                    })
                    .collect();
                self.members_entry(&name, "IECF_KIND_STRUCT", &format!("T_{k}"), &size, me, &members, tb, "struct")
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
        for m in members {
            let At::Field(f) = &m.at else { continue };
            let t = self.type_index(&m.ty, tb);
            list.push(format!("{{\"{}\", {t}u, (uint32_t)offsetof({c}, {f}), {}u, 0}}", m.name, m.flags));
            mj.push(format!("{{\"name\": {}, \"type\": {t}, \"flags\": {}}}", js(&m.name), json_flags(m.flags)));
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
    pub(crate) fn symbol_tables(&mut self, symbols: &[(String, Member, String, String)]) -> String {
        let p = self.prefix.clone();
        let mut tb = Tables::default();
        let mut rows = Vec::new();
        let mut json_rows = Vec::new();
        for (path, m, container, base) in symbols {
            let t = self.type_index(&m.ty, &mut tb);
            let (offset, area, bit) = match &m.at {
                At::Field(f) => (format!("(uint32_t)({base}offsetof({container}, {f}))"), "0".to_string(), "0u".into()),
                At::Image(a, byte, Some(bit)) => (format!("{byte}u"), format!("'{a}'"), format!("{bit}u")),
                At::Image(a, byte, None) => (format!("{byte}u"), format!("'{a}'"), "IECF_NO_BIT".into()),
            };
            rows.push(format!("{{\"{path}\", {t}u, {offset}, {area}, {bit}, {}u}}", m.flags));
            let at = match &m.at {
                At::Field(_) => String::new(),
                At::Image(a, byte, Some(bit)) => format!(", \"area\": \"{a}\", \"byte\": {byte}, \"bit\": {bit}"),
                At::Image(a, byte, None) => format!(", \"area\": \"{a}\", \"byte\": {byte}"),
            };
            json_rows.push(format!(
                "    {{\"path\": {}, \"type\": {t}, \"flags\": {}{at}}}",
                js(path),
                json_flags(m.flags)
            ));
        }
        self.symbols_json = format!(
            "{{\n  \"configuration\": {},\n  \"symbols\": [\n{}\n  ],\n  \"types\": [\n{}\n  ]\n}}\n",
            js(&p),
            json_rows.join(",\n"),
            tb.json.iter().map(|j| format!("    {j}")).collect::<Vec<_>>().join(",\n")
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
        c
    }
}
