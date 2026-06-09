use std::{fs, path::Path};

use ankit::{AnkiClient, actions::MultiAction};
use base64::{Engine, prelude::BASE64_STANDARD};
use eyre::Result;
use tracing::info;

pub async fn sync_media(client: &AnkiClient, deck_path: &Path) -> Result<()> {
	let assets_dir = deck_path.join("Assets");
	if !assets_dir.is_dir() {
		return Ok(());
	}

	info!("Syncing media from {:?}", assets_dir);

	for entry in fs::read_dir(&assets_dir)
		.map_err(|e| eyre::eyre!("Failed to read Assets directory {:?}: {}", assets_dir, e))?
	{
		let entry = entry.map_err(|e| eyre::eyre!("Failed to read directory entry: {}", e))?;
		let path = entry.path();

		if path.is_file() {
			if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
				info!("Storing media file: {}", filename);

				let data = base64_encode_file(&path)?;

				let params = serde_json::json!({
					"filename": filename,
					"data": data,
					"deleteExisting": true,
				});

				client
					.misc()
					.multi(&[MultiAction::with_params("storeMediaFile", params)])
					.await
					.map_err(|e| eyre::eyre!("Failed to store media file '{}': {}", filename, e))?;
			}
		}
	}

	Ok(())
}

fn base64_encode_file(path: &Path) -> Result<String> {
	let data = fs::read(path).map_err(|e| eyre::eyre!("Failed to read file {:?}: {}", path, e))?;
	Ok(BASE64_STANDARD.encode(data))
}
