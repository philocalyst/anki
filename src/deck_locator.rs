use std::{error::Error, fs, path::{Path, PathBuf}};

use chumsky::Parser;
use tracing::{debug, error, info, instrument, warn};

use crate::error::DeckError;

pub(crate) struct DeckLocator;

impl DeckLocator {
	#[instrument]
	pub(crate) fn find_deck_directory() -> Result<PathBuf, Box<dyn Error>> {
		info!("Searching for deck directory");

		let dirs: Vec<PathBuf> = fs::read_dir(".")?
			.filter_map(Result::ok)
			.filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
			.map(|entry| entry.path())
			.collect();

		dirs.into_iter().find(|dir| Self::is_deck_dir(dir)).ok_or_else(|| {
			error!("No deck directory found");
			DeckError::NoDeckFound.into()
		})
	}

	pub(crate) fn is_deck_dir(path: &Path) -> bool {
		path.is_dir() && path.extension().and_then(|e| e.to_str()) == Some("deck")
	}

	#[instrument]
	pub(crate) fn scan_deck_contents(
		deck_path: &Path,
	) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn Error>> {
		info!("Scanning deck contents at {:?}", deck_path);

		let mut models = Vec::new();
		let mut cards = Vec::new();

		for entry in fs::read_dir(deck_path)? {
			let entry = entry?;
			let path = entry.path();

			if entry.file_type()?.is_dir()
				&& entry.path().extension().and_then(|ext| ext.to_str()) == Some("model")
			{
				debug!("Found model directory: {:?}", path);
				models.push(path);
			} else if path.extension().and_then(|ext| ext.to_str()) == Some("flash") {
				debug!("Found card file: {:?}", path);
				cards.push(path);
			}
		}

		info!("Found {} models and {} card files", models.len(), cards.len());
		Ok((models, cards))
	}
}
