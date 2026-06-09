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
	command.arg("--output").arg(output_path);
	command.env("XDG_DATA_HOME", data_home);

	for deck_path in deck_paths {
		command.arg(deck_path);
	}

	Ok(command.output()?)
}

#[test]
fn cli_outputs_expected_shape_for_a_single_deck() -> Result<(), Box<dyn Error>> {
	let data_home = prepare_data_home()?;
	let deck_root = tempfile::tempdir()?;
	let grammar_repo =
		borrow_deck_repo(deck_root.path(), "grammar.deck", "out/Grammar.deck/index.flash", None)?;
	let output_dir = tempfile::tempdir()?;
	let output_path = output_dir.path().join("grammar.json");

	let output = run_cli(&[grammar_repo], &output_path, data_home.path())?;
	assert!(
		output.status.success(),
		"flash CLI failed:\nstdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);

	let json: Value = serde_json::from_str(&fs::read_to_string(&output_path)?)?;

	assert_eq!(json["__type__"], "Deck");
	assert_eq!(json["name"], "regex");
	assert_eq!(json["desc"], "");
	assert_eq!(json["dyn"], 0);
	assert_eq!(json["note_models"].as_array().map(Vec::len), Some(1));
	assert_eq!(json["note_models"][0]["name"], "Basic");
	assert_eq!(json["note_models"][0]["flds"][0]["name"], "Question");
	assert_eq!(json["note_models"][0]["flds"][1]["name"], "Answer");
	assert_eq!(json["notes"][0]["fields"][0], "What can a noun be?");
	assert!(
		json["notes"][0]["fields"][1]
			.as_str()
			.is_some_and(|field| field.contains("People, places, things, or ideas"))
	);

	Ok(())
}

#[test]
fn cli_handles_real_fixture_deck_repos() -> Result<(), Box<dyn Error>> {
	let data_home = prepare_data_home()?;
	let deck_root = tempfile::tempdir()?;
	let grammar_repo =
		borrow_deck_repo(deck_root.path(), "grammar.deck", "out/Grammar.deck/index.flash", None)?;
	let micro_repo = borrow_deck_repo(
		deck_root.path(),
		"micro.deck",
		"out/Micro.deck/index.flash",
		Some("/ Basic /\nalias Question to Q\nalias Answer to A\n\n"),
	)?;
	let output_dir = tempfile::tempdir()?;
	let output_path = output_dir.path().join("fixture-decks.json");

	let output = run_cli(&[grammar_repo, micro_repo], &output_path, data_home.path())?;
	assert!(
		output.status.success(),
		"flash CLI failed:\nstdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);

	let json: Value = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
	let decks = json.as_array().ok_or("expected CLI output to be an array")?;

	assert_eq!(decks.len(), 2);
	assert_eq!(decks[0]["__type__"], "Deck");
	assert_eq!(decks[1]["__type__"], "Deck");
	assert_eq!(decks[0]["note_models"][0]["name"], "Basic");
	assert_eq!(decks[1]["note_models"][0]["name"], "Basic");
	assert_eq!(decks[0]["notes"][0]["fields"][0], "What can a noun be?");
	assert_eq!(decks[1]["notes"][0]["fields"][0], "What is a rival good?");
	assert!(decks[0]["notes"].as_array().is_some_and(|notes| !notes.is_empty()));
	assert!(decks[1]["notes"].as_array().is_some_and(|notes| !notes.is_empty()));

	Ok(())
}
