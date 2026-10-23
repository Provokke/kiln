//! Differential testing against node: every conformance script is executed by
//! `node` *live* and by kiln (default options: tiered JIT with the normal
//! threshold) and the stdout must be identical. Skipped (with a message) if
//! node is not installed.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn differential_against_node() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("node not found; skipping differential test");
        return;
    }
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "js"))
        .collect();
    files.sort();
    let mut mismatches = Vec::new();
    for f in &files {
        let node = Command::new("node").arg("--expose-gc").arg(f).output().unwrap();
        assert!(node.status.success(), "node failed on {f:?}");
        let expected = String::from_utf8(node.stdout).unwrap();
        let src = std::fs::read_to_string(f).unwrap();
        let got = kiln::with_big_stack(move || kiln::run_source(&src, kiln::vm::Options::default()));
        if got.output != expected || got.error.is_some() {
            mismatches.push(format!("{f:?}: {:?}", got.error));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches vs node:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    eprintln!("differential: {} scripts identical to node", files.len());
}
