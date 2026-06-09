use std::{fs, path::{Path, PathBuf}};

use tracing::{debug, error, info, instrument};

use crate::error::DeckError;

#[instrument]
pub fn find_deck_directory() -> Result<PathBuf, DeckError> {
	info!("Searching for deck directory");

	pub fn is_deck_dir(path: &Path) -> bool {
		path.is_dir() && path.extension().and_then(|e| e.to_str()) == Some("deck")
	}

	fs::read_dir(".")?.flatten().map(|e| e.path()).find(|p| is_deck_dir(p)).ok_or_else(|| {
		error!("No deck directory found");
		DeckError::NoDeckFound
	})
}

#[instrument]
pub fn scan_deck_contents(deck_path: &Path) -> Result<Vec<PathBuf>, DeckError> {
	info!("Scanning deck contents at {:?}", deck_path);

	let mut cards = Vec::new();

	for entry in fs::read_dir(deck_path)? {
		let path = entry?.path();
		let extension = path.extension().and_then(|s| s.to_str());

		if extension == Some("flash") && path.is_file() {
			debug!("Found card file: {:?}", path);
			cards.push(path);
		}
	}

	info!("Found {} card files", cards.len());
	Ok(cards)
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::scan_deck_contents;

	#[test]
	fn scan_deck_contents_finds_flash_files() {
		let dir = tempfile::tempdir().unwrap();
		fs::write(dir.path().join("a.flash"), "").unwrap();
		fs::write(dir.path().join("b.flash"), "").unwrap();
		fs::write(dir.path().join("readme.md"), "").unwrap();
		fs::create_dir(dir.path().join("subdir")).unwrap();

		let result = scan_deck_contents(dir.path()).unwrap();
		assert_eq!(result.len(), 2);
		let names: Vec<String> = result.iter().map(|p| p.file_name().unwrap().to_string_lossy().into()).collect();
		assert!(names.contains(&"a.flash".into()));
		assert!(names.contains(&"b.flash".into()));
	}

	#[test]
	fn scan_deck_contents_empty_dir() {
		let dir = tempfile::tempdir().unwrap();
		let result = scan_deck_contents(dir.path()).unwrap();
		assert!(result.is_empty());
	}

	#[test]
	fn scan_deck_contents_ignores_non_flash_files() {
		let dir = tempfile::tempdir().unwrap();
		fs::write(dir.path().join("note.txt"), "hello").unwrap();
		fs::write(dir.path().join("data.json"), "{}").unwrap();
		fs::write(dir.path().join("index.flash"), "").unwrap();

		let result = scan_deck_contents(dir.path()).unwrap();
		assert_eq!(result.len(), 1);
	}
}
