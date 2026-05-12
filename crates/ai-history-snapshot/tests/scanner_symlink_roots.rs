#![cfg(unix)]

use ai_history_core::WorkspacePath;
use ai_history_snapshot::scanner::{Scanner, ScannerConfig};
use std::{fs, os::unix::fs::symlink};

#[test]
fn nested_relative_symlinks_are_resolved_from_their_containing_directory() {
    scan(false);
}

#[test]
fn internal_symlinks_are_preserved_when_the_workspace_root_has_an_alias() {
    scan(true);
}

fn scan(alias: bool) {
    let temp = tempfile::TempDir::new().unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("nested")).unwrap();
    fs::write(workspace.join("target.txt"), "inside").unwrap();
    // The same lexical target resolved from the root would name an outside file.
    fs::write(temp.path().join("target.txt"), "outside decoy").unwrap();
    fs::write(temp.path().join("outside.txt"), "outside").unwrap();
    symlink("target.txt", workspace.join("internal.txt")).unwrap();
    symlink("../target.txt", workspace.join("nested/internal.txt")).unwrap();
    symlink("../../outside.txt", workspace.join("nested/external.txt")).unwrap();
    let root = if alias {
        let root = temp.path().join("alias");
        symlink(&workspace, &root).unwrap();
        root
    } else {
        workspace.canonicalize().unwrap()
    };
    let path = WorkspacePath::from_string(root.to_str().unwrap());
    let manifest = Scanner::new(ScannerConfig::default()).scan(&path).unwrap();
    assert!(manifest.get_entry("target.txt").is_some());
    assert!(
        manifest.get_entry("internal.txt").is_some(),
        "root aliases must not turn an internal link into an external one"
    );
    let internal = manifest
        .get_entry("nested/internal.txt")
        .expect("a symlink to a file inside the workspace must be included");
    assert_eq!(internal.symlink_target.as_deref(), Some("../target.txt"));
    assert!(
        manifest.get_entry("nested/external.txt").is_none(),
        "a relative symlink escaping the workspace must remain excluded"
    );
}
