//! The commit iecf is built from, for `iecf --version` and the browser's
//! module (`iecf_driver::VERSION`). A platform that runs the module in a
//! browser and the binary on its server compares the two: the version
//! number alone is the same for every commit between two releases, so it
//! cannot tell a stale module from a fresh one.
//!
//! `IECF_COMMIT` is the commit's first 12 hex digits; with changes not
//! committed, `.dirty.` and 16 hex digits that hash them (`git diff HEAD`
//! and the untracked files' names and contents), so that two builds of two
//! different working trees do not say the same. Without git - a source
//! archive - `unknown`.

use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git").arg("-C").arg(root).args(args).output().ok()?;
    out.status.success().then_some(out.stdout)
}

/// FNV-1a, 64 bits: no dependency, and nothing here needs more.
fn fnv(hash: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *hash ^= u64::from(b);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let root = Path::new(&manifest).join("../..");
    let dot_git = root.join(".git");
    // Again when the commit, the index or a source changes.
    for p in ["HEAD", "index", "packed-refs", "refs/heads"] {
        println!("cargo:rerun-if-changed={}", dot_git.join(p).display());
    }
    println!("cargo:rerun-if-changed={}", root.join("crates").display());

    let commit = git(&root, &["rev-parse", "--short=12", "HEAD"])
        .and_then(|o| String::from_utf8(o).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let version = match commit {
        None => "unknown".to_string(),
        Some(commit) => {
            let diff = git(&root, &["diff", "HEAD", "--binary"]).unwrap_or_default();
            let untracked = git(&root, &["ls-files", "--others", "--exclude-standard", "-z"]).unwrap_or_default();
            if diff.is_empty() && untracked.is_empty() {
                commit
            } else {
                let mut hash = 0xcbf2_9ce4_8422_2325_u64;
                fnv(&mut hash, &diff);
                for name in untracked.split(|&b| b == 0).filter(|n| !n.is_empty()) {
                    fnv(&mut hash, name);
                    let path = root.join(String::from_utf8_lossy(name).as_ref());
                    fnv(&mut hash, &std::fs::read(path).unwrap_or_default());
                }
                format!("{commit}.dirty.{hash:016x}")
            }
        }
    };
    println!("cargo:rustc-env=IECF_COMMIT={version}");
}
