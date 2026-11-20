//! CODESYS's and TwinCAT's structure extended (the struct-extends
//! extension): `TYPE X EXTENDS Base : STRUCT ... END_STRUCT`, X's fields
//! its base's first, at any depth, then its own - written so before
//! anything is checked, X then a structure as any other. Its base must be
//! a structure; a cycle, or a base that is no structure, is left for the
//! checker to say (check.rs).

use iecf_syntax::ast::*;
use std::collections::HashMap;

/// Whether some structure extends another.
pub fn needed(units: &[&SourceUnit]) -> bool {
    units.iter().any(|u| u.items.iter().any(|i| matches!(i, Item::Types(ts) if ts.iter().any(|t| t.extends.is_some()))))
}

/// Each extending structure given its bases' fields first.
pub fn rewrite(units: &mut [SourceUnit]) {
    let mut structs: HashMap<String, (Option<String>, Vec<VarDecl>)> = HashMap::new();
    for unit in units.iter() {
        for item in &unit.items {
            let Item::Types(ts) = item else { continue };
            for t in ts {
                if let TypeDef::Struct { fields, .. } = &t.def {
                    structs.insert(
                        t.name.text.to_ascii_uppercase(),
                        (t.extends.as_ref().map(|b| b.text.to_ascii_uppercase()), fields.clone()),
                    );
                }
            }
        }
    }
    // The fields a structure inherits: its bases', the farthest first.
    let inherited = |base: &str| {
        let mut chain: Vec<&Vec<VarDecl>> = Vec::new();
        let mut at = Some(base.to_string());
        let mut seen = Vec::new();
        while let Some(b) = at {
            if seen.contains(&b) {
                break;
            }
            seen.push(b.clone());
            let Some((up, fields)) = structs.get(&b) else { break };
            chain.push(fields);
            at = up.clone();
        }
        chain.into_iter().rev().flatten().cloned().collect::<Vec<VarDecl>>()
    };
    for unit in units.iter_mut() {
        for item in unit.items.iter_mut() {
            let Item::Types(ts) = item else { continue };
            for t in ts.iter_mut() {
                let Some(base) = &t.extends else { continue };
                let mut fields = inherited(&base.text.to_ascii_uppercase());
                if let TypeDef::Struct { fields: own, .. } = &mut t.def {
                    fields.append(own);
                    *own = fields;
                }
            }
        }
    }
}
