use std::{error::Error, fs, path::{Path, PathBuf}, process::{Command, Output}};
use tempfile::TempDir;

pub const MINIMAL_DECK_CONTENT: &str = "\
/ Basic /
alias Question to Q
alias Answer to A

(Q) What can a noun be?
(A) People, places, things, or ideas?

(Q) What is a verb?
(A) An action word.
";

pub fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

pub fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
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

pub fn prepare_data_home() -> Result<TempDir, Box<dyn Error>> {
    let temp_dir = tempfile::tempdir()?;
    let flash_home = temp_dir.path().join("flash");

    fs::create_dir_all(&flash_home)?;
    copy_dir_recursive(&fixture_path("out/Micro.deck/Basic.model"), &flash_home.join("Basic.model"))?;

    Ok(temp_dir)
}

pub fn run_git(arguments: &[&str], working_directory: &Path) -> Result<(), Box<dyn Error>> {
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

pub fn make_deck_repo(root: &Path, name: &str, content: &str) -> Result<PathBuf, Box<dyn Error>> {
    let repo_path = root.join(name);
    fs::create_dir_all(&repo_path)?;
    fs::write(repo_path.join("index.flash"), content)?;
    run_git(&["init"], &repo_path)?;
    run_git(&["add", "index.flash"], &repo_path)?;
    run_git(&["commit", "-m", "init"], &repo_path)?;
    Ok(repo_path)
}

pub fn borrow_deck_repo(
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

pub fn run_cli(
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
