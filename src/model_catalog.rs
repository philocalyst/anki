use std::{fs::ReadDir, path::Path};

use dir_spec::data_home;

use crate::{error::DeckError, model_loader, note::NoteModel};

pub trait ModelCatalog {
	fn load_models(&self, deck_path: &Path) -> Result<Vec<NoteModel>, DeckError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FilesystemModelCatalog;

impl FilesystemModelCatalog {
	fn find_models(&self) -> Result<ReadDir, DeckError> {
		let flash_home = data_home().unwrap().join("flash");

		Ok(std::fs::read_dir(flash_home)?)
	}
}

impl ModelCatalog for FilesystemModelCatalog {
	fn load_models(&self, deck_path: &Path) -> Result<Vec<NoteModel>, DeckError> {
		model_loader::load_models(self.find_models()?, deck_path)
	}
}
