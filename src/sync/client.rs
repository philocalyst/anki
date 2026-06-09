use std::collections::HashSet;

use ankit::{AnkiClient, MultiAction};
use eyre::Result;
use serde_json::Value;

/// Wraps Anki client with a MUCH better API
pub struct FlashClient(pub AnkiClient);

impl FlashClient {
	/// Get the full information of an Anki profile
	pub async fn get_full_information(&self) -> Result<()> {
		self
			.0
			.misc()
			.multi(&[
				MultiAction::with_params("getCollection", Value::Null),
				MultiAction::with_params("deckNamesAndIds", Value::Null),
				MultiAction::with_params("modelNamesAndIds", Value::Null),
			])
			.await
			.map_err(|e| eyre::eyre!("Failed to get collection snapshot: {}", e))?;
		Ok(())
	}

	/// Get deck config from Anki by name
	pub async fn get_deck_config(&self, deck_name: &str) -> Result<Option<Value>> {
		let results = self
			.0
			.misc()
			.multi(&[MultiAction::with_params(
				"getDeckConfig",
				serde_json::json!({"deck": deck_name}),
			)])
			.await
			.map_err(|e| eyre::eyre!("Failed to get deck config for '{}': {}", deck_name, e))?;
		Ok(results.into_iter().next())
	}

	/// Save deck config to Anki
	pub async fn save_deck_config(
		&self,
		config: &serde_json::Map<String, Value>,
	) -> Result<()> {
		self
			.0
			.misc()
			.multi(&[MultiAction::with_params(
				"saveDeckConfig",
				serde_json::json!({"config": config}),
			)])
			.await
			.map_err(|e| eyre::eyre!("Failed to save deck config: {}", e))?;
		Ok(())
	}

	/// Store a media file in Anki
	pub async fn store_media_file(&self, filename: &str, data: &str) -> Result<()> {
		self
			.0
			.misc()
			.multi(&[MultiAction::with_params(
				"storeMediaFile",
				serde_json::json!({
					"filename": filename,
					"data": data,
					"deleteExisting": true,
				}),
			)])
			.await
			.map_err(|e| eyre::eyre!("Failed to store media file '{}': {}", filename, e))?;
		Ok(())
	}
}

impl std::ops::Deref for FlashClient {
	type Target = AnkiClient;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Find a unique name by appending a suffix counter when the base name is taken.
/// e.g. if "Basic" is taken, tries "Basic 2", "Basic 3", etc.
pub fn unique_name(name: &str, taken: &HashSet<String>) -> String {
	if !taken.contains(name) {
		return name.to_string();
	}
	let mut suffix = 2;
	loop {
		let candidate = format!("{} {}", name, suffix);
		if !taken.contains(&candidate) {
			return candidate;
		}
		suffix += 1;
	}
}
