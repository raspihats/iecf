use crate::{Extension, Pack};

#[test]
fn every_extension_is_documented() {
    let doc = include_str!("../../../docs/extensions.md");
    for e in Extension::ALL {
        let heading = format!("## {}: {}", e.name(), e.summary());
        assert!(doc.lines().any(|l| l == heading), "docs/extensions.md needs the heading `{heading}`");
    }
}

#[test]
fn every_pack_is_documented_with_its_extensions() {
    let doc = include_str!("../../../docs/extensions.md");
    for p in Pack::ALL {
        assert!(Extension::from_name(p.name()).is_none(), "a pack's name is no extension's: {}", p.name());
        let start = format!("| `{}` | {} |", p.name(), p.summary());
        let Some(row) = doc.lines().find(|l| l.starts_with(&start)) else {
            panic!("docs/extensions.md needs the row `{start} ...`");
        };
        for e in Extension::ALL {
            let named = row.contains(&format!("`{}`", e.name()));
            assert_eq!(
                named,
                p.members().contains(e),
                "{}: {} in its row or not as in Pack::members",
                p.name(),
                e.name()
            );
        }
    }
}
