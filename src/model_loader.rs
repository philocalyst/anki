use std::{error::Error, fs, path::{Path, PathBuf}};

use tracing::{debug, info, instrument};

use crate::types::note::NoteModel;

#[instrument]
pub fn load_models(
	model_paths: &[PathBuf],
	deck_path: &Path,
) -> Result<Vec<NoteModel>, Box<dyn Error>> {
	info!("Loading {} models", model_paths.len());

	let mut all_models = Vec::new();

	for model_path in model_paths {
		let config_path = model_path.join("config.toml");
		debug!("Loading model config from {:?}", config_path);

		let config_content = fs::read_to_string(&config_path)?;
		let mut model: NoteModel = toml::from_str(&config_content)?;

		// TODO: This path should be more dynamic
		model.complete(deck_path)?;

		info!("Loaded model: {}", model.name);
		all_models.push(model);
	}

	Ok(all_models)
}
