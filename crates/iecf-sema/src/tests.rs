use crate::Extension;

#[test]
fn every_extension_is_documented() {
    let doc = include_str!("../../../docs/extensions.md");
    for e in Extension::ALL {
        let heading = format!("## {}: {}", e.name(), e.summary());
        assert!(doc.lines().any(|l| l == heading), "docs/extensions.md needs the heading `{heading}`");
    }
}
