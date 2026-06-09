use std::{
	error::Error,
	fs,
	path::{Path, PathBuf},
	process::{Command, Output, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

fn fixture_path(relative: &str) -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
	fs::create_dir_all(destination)?;

	for entry in fs::read_dir(source)? {
		let entry = entry?;
		let entry_type = entry.file_type()?;
		let target = destination.join(entry.file_name());

		if entry_type.is_dir() {
			copy_dir_recursive(&entry.path(), &target)?;
		} else {
			fs::copy(entry.path(), target)?;
		}
	}

	Ok(())
}

fn prepare_data_home() -> Result<TempDir, Box<dyn Error>> {
	let temp_dir = tempfile::tempdir()?;
	let flash_home = temp_dir.path().join("flash");

	fs::create_dir_all(&flash_home)?;
	copy_dir_recursive(&fixture_path("out/Micro.deck/Basic.model"), &flash_home.join("Basic.model"))?;

	Ok(temp_dir)
}

fn run_git(arguments: &[&str], working_directory: &Path) -> Result<(), Box<dyn Error>> {
	let mut cmd = Command::new("git");
	cmd.args(&["-c", "commit.gpgSign=false", "-c", "tag.gpgSign=false"]);
	cmd.args(arguments);
	cmd.current_dir(working_directory);
	cmd.env("GIT_AUTHOR_NAME", "Flash Tests");
	cmd.env("GIT_AUTHOR_EMAIL", "flash-tests@example.com");
	cmd.env("GIT_AUTHOR_DATE", "2024-01-01T00:00:00Z");
	cmd.env("GIT_COMMITTER_NAME", "Flash Tests");
	cmd.env("GIT_COMMITTER_EMAIL", "flash-tests@example.com");
	cmd.env("GIT_COMMITTER_DATE", "2024-01-01T00:00:00Z");
	let output = cmd.output()?;

	if output.status.success() {
		return Ok(());
	}

	Err(
		format!(
			"git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
			arguments,
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		)
		.into(),
	)
}

/// Create a minimal deck repo at `root/name` whose `index.flash` content is
/// the given `content`. Runs `git init`, `git add`, `git commit`.
fn make_deck_repo(root: &Path, name: &str, content: &str) -> Result<PathBuf, Box<dyn Error>> {
	let repo_path = root.join(name);
	fs::create_dir_all(&repo_path)?;
	fs::write(repo_path.join("index.flash"), content)?;
	run_git(&["init"], &repo_path)?;
	run_git(&["add", "index.flash"], &repo_path)?;
	run_git(&["commit", "-m", "init"], &repo_path)?;
	Ok(repo_path)
}

/// Copy a fixture deck repo, optionally prepending a prelude.
fn borrow_deck_repo(
	root: &Path,
	name: &str,
	index_source: &str,
	index_prelude: Option<&str>,
) -> Result<PathBuf, Box<dyn Error>> {
	let repo_path = root.join(name);
	fs::create_dir_all(&repo_path)?;

	let mut index_contents = fs::read_to_string(fixture_path(index_source))?;
	if let Some(prelude) = index_prelude {
		index_contents = format!("{prelude}{index_contents}");
	}

	fs::write(repo_path.join("index.flash"), index_contents)?;

	run_git(&["init"], &repo_path)?;
	run_git(&["add", "index.flash"], &repo_path)?;
	run_git(&["commit", "-m", "Import borrowed fixture"], &repo_path)?;

	Ok(repo_path)
}

fn run_cli(
	deck_paths: &[PathBuf],
	output_path: &Path,
	data_home: &Path,
) -> Result<Output, Box<dyn Error>> {
	let mut command = Command::new(env!("CARGO_BIN_EXE_flash"));
	command.arg("--output-json").arg(output_path);
	command.env("XDG_DATA_HOME", data_home);

	for deck_path in deck_paths {
		command.arg(deck_path);
	}

	Ok(command.output()?)
}

// ============================================================================
//  Basic smoke tests using a known-good inline fixture
// ============================================================================

const MINIMAL_DECK_CONTENT: &str = "\
/ Basic /
alias Question to Q
alias Answer to A

(Q) What can a noun be?
(A) People, places, things, or ideas?

(Q) What is a verb?
(A) An action word.
";

#[test]
fn cli_outputs_expected_shape() -> Result<(), Box<dyn Error>> {
	let data_home = prepare_data_home()?;
	let deck_root = tempfile::tempdir()?;
	let deck_repo = make_deck_repo(deck_root.path(), "grammar.deck", MINIMAL_DECK_CONTENT)?;
	let output_dir = tempfile::tempdir()?;
	let output_path = output_dir.path().join("out.json");

	let output = run_cli(&[deck_repo], &output_path, data_home.path())?;
	assert!(
		output.status.success(),
		"flash CLI failed:\nstdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);

	let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;

	assert_eq!(json.len(), 1);
	assert_eq!(json[0]["name"], "regex");
	assert!(json[0]["flash_uuid"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
	assert_eq!(json[0]["note_count"].as_i64().unwrap_or(0), 2);
	assert_eq!(json[0]["model_count"].as_i64().unwrap_or(0), 1);
	assert_eq!(
		json[0]["card_ids"].as_array().map(|a| a.len()).unwrap_or(0),
		2,
		"Should have 2 card IDs for 2 notes"
	);

	Ok(())
}

#[test]
fn cli_outputs_include_note_uuids() -> Result<(), Box<dyn Error>> {
	let data_home = prepare_data_home()?;
	let deck_root = tempfile::tempdir()?;
	let deck_repo = make_deck_repo(deck_root.path(), "test.deck", MINIMAL_DECK_CONTENT)?;
	let output_dir = tempfile::tempdir()?;
	let output_path = output_dir.path().join("ids.json");

	let output = run_cli(&[deck_repo], &output_path, data_home.path())?;
	assert!(
		output.status.success(),
		"flash CLI failed:\nstdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);

	let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
	let card_ids = json[0]["card_ids"].as_array().unwrap();

	assert_eq!(card_ids.len(), 2, "Should have 2 card IDs");
	for id in card_ids {
		let s = id.as_str().unwrap();
		assert_eq!(s.len(), 36, "Each card ID should be a UUID string");
		assert_eq!(s.chars().filter(|&c| c == '-').count(), 4, "UUID format check");
	}

	// UUIDs should all be different
	let unique: std::collections::HashSet<&str> =
		card_ids.iter().map(|v| v.as_str().unwrap()).collect();
	assert_eq!(unique.len(), card_ids.len(), "All UUIDs should be unique");

	Ok(())
}

#[test]
fn cli_uuids_are_consistent_across_runs() -> Result<(), Box<dyn Error>> {
	let data_home = prepare_data_home()?;
	let deck_root = tempfile::tempdir()?;

	let deck_repo = make_deck_repo(deck_root.path(), "test.deck", MINIMAL_DECK_CONTENT)?;
	let output_dir = tempfile::tempdir()?;
	let output_path = output_dir.path().join("run1.json");

	run_cli(&[deck_repo.clone()], &output_path, data_home.path())?;
	let json1: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
	let ids1: Vec<String> = json1[0]["card_ids"]
		.as_array()
		.unwrap()
		.iter()
		.map(|v| v.as_str().unwrap().to_string())
		.collect();

	let output_path2 = output_dir.path().join("run2.json");
	run_cli(&[deck_repo], &output_path2, data_home.path())?;
	let json2: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path2)?)?;
	let ids2: Vec<String> = json2[0]["card_ids"]
		.as_array()
		.unwrap()
		.iter()
		.map(|v| v.as_str().unwrap().to_string())
		.collect();

	assert_eq!(ids1, ids2, "UUIDs should be deterministic across runs");
	assert_eq!(json1[0]["flash_uuid"], json2[0]["flash_uuid"], "Deck UUID should be deterministic");

	Ok(())
}

#[test]
fn cli_handles_multiple_decks() -> Result<(), Box<dyn Error>> {
	let data_home = prepare_data_home()?;
	let deck_root = tempfile::tempdir()?;

	let deck_a = make_deck_repo(deck_root.path(), "deck-a", MINIMAL_DECK_CONTENT)?;
	let deck_b = make_deck_repo(deck_root.path(), "deck-b", MINIMAL_DECK_CONTENT)?;
	let output_dir = tempfile::tempdir()?;
	let output_path = output_dir.path().join("multi.json");

	let output = run_cli(&[deck_a, deck_b], &output_path, data_home.path())?;
	assert!(
		output.status.success(),
		"flash CLI failed:\nstdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);

	let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
	assert_eq!(json.len(), 2);
	assert!(json[0]["flash_uuid"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
	assert!(json[1]["flash_uuid"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
	// Both decks have identical content — deterministic UUIDs should match
	assert_eq!(json[0]["flash_uuid"], json[1]["flash_uuid"]);

	Ok(())
}

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
fn cli_handles_deck_with_one_note() -> Result<(), Box<dyn Error>> {
	let data_home = prepare_data_home()?;
	let deck_root = tempfile::tempdir()?;
	let content = "/ Basic /\nalias Question to Q\n\n(Q) Just one?\n";
	let deck_repo = make_deck_repo(deck_root.path(), "lonely.deck", content)?;
	let output_dir = tempfile::tempdir()?;
	let output_path = output_dir.path().join("out.json");

	let output = run_cli(&[deck_repo], &output_path, data_home.path())?;
	assert!(output.status.success());

	let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
	assert_eq!(json.len(), 1);
	assert_eq!(json[0]["note_count"].as_i64().unwrap_or(0), 1);
	assert_eq!(json[0]["card_ids"].as_array().map(|a| a.len()).unwrap_or(0), 1);

	Ok(())
}

#[test]
fn cli_handles_deck_with_many_notes() -> Result<(), Box<dyn Error>> {
	let data_home = prepare_data_home()?;
	let deck_root = tempfile::tempdir()?;

	// Build a deck with 50 notes
	let mut content = String::from("/ Basic /\nalias Question to Q\nalias Answer to A\n\n");
	for i in 0..50 {
		content.push_str(&format!("(Q) Question {i}?\n(A) Answer {i}\n\n"));
	}
	let deck_repo = make_deck_repo(deck_root.path(), "big.deck", &content)?;
	let output_dir = tempfile::tempdir()?;
	let output_path = output_dir.path().join("out.json");

	let output = run_cli(&[deck_repo], &output_path, data_home.path())?;
	assert!(output.status.success());

	let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
	assert_eq!(json.len(), 1);
	assert_eq!(json[0]["note_count"].as_i64().unwrap_or(0), 50);
	assert_eq!(json[0]["card_ids"].as_array().map(|a| a.len()).unwrap_or(0), 50);

	// All UUIDs must be unique
	let ids: Vec<&str> =
		json[0]["card_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
	let unique: std::collections::HashSet<&&str> = ids.iter().collect();
	assert_eq!(unique.len(), 50, "All 50 UUIDs should be unique");

	Ok(())
}
