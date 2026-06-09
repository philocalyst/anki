use std::{fs, path::Path};

use base64::{Engine, prelude::BASE64_STANDARD};
use eyre::Result;
use tracing::info;

use crate::sync::client::FlashClient;

pub async fn sync_media(client: &FlashClient, deck_path: &Path) -> Result<()> {
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

				client.store_media_file(filename, &data).await?;
			}
		}
	}

	Ok(())
}

fn base64_encode_file(path: &Path) -> Result<String> {
	let file_data = fs::read(path).map_err(|e| eyre::eyre!("Failed to read file {:?}: {}", path, e))?;
	Ok(BASE64_STANDARD.encode(file_data))
}
