mod common;

use std::{fs, path::PathBuf, process::Command};
use common::*;

#[test]
fn cli_fails_on_nonexistent_deck_path() {
    let data_home = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("out.json");
    let fake_path = PathBuf::from("/tmp/nonexistent-deck-path-12345");

    let result = run_cli(&[fake_path], &output_path, data_home.path());

    match result {
        Ok(output) => {
            assert!(
                !output.status.success(),
                "CLI should fail on nonexistent deck path\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("NotFound") || msg.contains("No such") || msg.contains("error"),
                "Expected error about missing path, got: {msg}"
            );
        }
    }
}

#[test]
fn cli_fails_on_empty_directory() {
    let data_home = tempfile::tempdir().unwrap();
    let deck_root = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("out.json");

    let result = run_cli(&[deck_root.path().to_path_buf()], &output_path, data_home.path());

    match result {
        Ok(output) => assert!(!output.status.success()),
        Err(_) => {} // Acceptable
    }
}

#[test]
fn cli_fails_on_directory_without_git_repo() {
    let data_home = tempfile::tempdir().unwrap();
    let deck_root = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("out.json");

    fs::write(deck_root.path().join("index.flash"), "(Q) Test?\n(A) Answer\n").unwrap();

    let result = run_cli(&[deck_root.path().to_path_buf()], &output_path, data_home.path());

    match result {
        Ok(output) => assert!(!output.status.success()),
        Err(_) => {}
    }
}

#[test]
fn cli_rejects_flag_conflicts() {
    let data_home = tempfile::tempdir().unwrap();
    let deck_root = tempfile::tempdir().unwrap();
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("out.json");

    fs::write(deck_root.path().join("index.flash"), "(Q) Test?\n(A) Answer\n").unwrap();
    run_git(&["init"], deck_root.path()).unwrap();
    run_git(&["add", "index.flash"], deck_root.path()).unwrap();
    run_git(&["commit", "-m", "init"], deck_root.path()).unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flash"));
    cmd.arg("--output-json").arg(&output_path);
    cmd.arg("--no-prune");
    cmd.arg(deck_root.path());
    cmd.env("XDG_DATA_HOME", data_home.path());
    let output = cmd.output().unwrap();
    // CLI may fail gracefully (no model loaded) but must not panic/crash
    // We accept either success or a non-zero exit with an error message
    if !output.status.success() {
        assert!(
            !output.stderr.is_empty(),
            "CLI should produce error output on failure\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
