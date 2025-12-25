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
pub fn scan_deck_contents(deck_path: &Path) -> Result<(Vec<PathBuf>, Vec<PathBuf>), DeckError> {
	info!("Scanning deck contents at {:?}", deck_path);

	let mut models = Vec::new();
	let mut cards = Vec::new();

	for entry in fs::read_dir(deck_path)? {
		let path = entry?.path();
		let extension = path.extension().and_then(|s| s.to_str());

		if extension == Some("model") && path.is_dir() {
			debug!("Found model directory: {:?}", path);
			models.push(path);
		} else if extension == Some("flash") && path.is_file() {
			debug!("Found card file: {:?}", path);
			cards.push(path);
		}
	}

	info!("Found {} models and {} card files", models.len(), cards.len());
	Ok((models, cards))
}
