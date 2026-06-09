use ankit::{AnkiClient, actions::MultiAction};
use eyre::{Result, bail};
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::DeckConfig;
use crate::sync::connection::CollectionSnapshot;
use crate::sync::identity::FLASH_DECK_UUID_KEY;

pub struct DeckSyncData {
	pub uuid:   Uuid,
	pub name:   String,
	pub config: DeckConfig,
}

/// Sync a deck: create if new, update if existing.
/// Returns the deck name as it exists in Anki (may differ if renamed).
pub async fn sync_deck(
	client: &AnkiClient,
	deck: &DeckSyncData,
	snapshot: &CollectionSnapshot,
) -> Result<String> {
	let uuid_str = deck.uuid.to_string();

	// Look up deck by UUID stored in its config
	let existing_name = find_deck_by_config_uuid(client, &deck.uuid).await?;

	if let Some(anki_name) = existing_name {
		info!("Deck '{}' found in Anki (uuid: {})", anki_name, uuid_str);

		// Ensure the deck exists (user may have deleted it)
		if !snapshot.decks.contains_key(&anki_name) {
			warn!("Deck '{}' not in snapshot, recreating", anki_name);
			client.decks().create(&anki_name).await.map_err(|e| {
				eyre::eyre!("Failed to create deck '{}': {}", anki_name, e)
			})?;
		}

		Ok(anki_name)
	} else {
		// No deck with our UUID found — check if a deck with our name already exists
		let target_name = if snapshot.decks.contains_key(&deck.name) {
			// Name collision — append suffix (like CrowdAnki does)
			let mut suffix = 2;
			let mut candidate = format!("{} {}", deck.name, suffix);
			while snapshot.decks.contains_key(&candidate) {
				suffix += 1;
				candidate = format!("{} {}", deck.name, suffix);
			}
			warn!(
				"Deck name '{}' already taken, using '{}'",
				deck.name, candidate
			);
			candidate
		} else {
			deck.name.clone()
		};

		info!("Creating new deck '{}'", target_name);
		client.decks().create(&target_name).await.map_err(|e| {
			eyre::eyre!("Failed to create deck '{}': {}", target_name, e)
		})?;

		// Store UUID in deck config
		store_deck_uuid(client, &target_name, &deck.uuid).await?;

		Ok(target_name)
	}
}

/// Find a deck by its flash UUID stored in the deck config.
async fn find_deck_by_config_uuid(
	client: &AnkiClient,
	deck_uuid: &Uuid,
) -> Result<Option<String>> {
	let deck_names = client.decks().names().await.map_err(|e| {
		eyre::eyre!("Failed to get deck names: {}", e)
	})?;

	let uuid_str = deck_uuid.to_string();

	for name in &deck_names {
		let results = client
			.misc()
			.multi(&[MultiAction::with_params(
				"getDeckConfig",
				json!({"deck": name}),
			)])
			.await
			.map_err(|e| eyre::eyre!("Failed to get deck config for '{}': {}", name, e))?;

		if let Some(config) = results.into_iter().next() {
			if let Some(found) = config.get(FLASH_DECK_UUID_KEY).and_then(|v| v.as_str()) {
				if found == uuid_str {
					return Ok(Some(name.clone()));
				}
			}
		}
	}

	Ok(None)
}

/// Store the flash UUID in a deck's config so we can find it later.
async fn store_deck_uuid(
	client: &AnkiClient,
	deck_name: &str,
	deck_uuid: &Uuid,
) -> Result<()> {
	let results = client
		.misc()
		.multi(&[MultiAction::with_params(
			"getDeckConfig",
			json!({"deck": deck_name}),
		)])
		.await
		.map_err(|e| eyre::eyre!("Failed to get deck config for '{}': {}", deck_name, e))?;

	let mut config = match results.into_iter().next() {
		Some(serde_json::Value::Object(map)) => map,
		_ => {
			bail!("Unexpected deck config format for '{}'", deck_name);
		}
	};

	config.insert(FLASH_DECK_UUID_KEY.to_string(), json!(deck_uuid.to_string()));

	client
		.misc()
		.multi(&[MultiAction::with_params(
			"saveDeckConfig",
			json!({"config": &config}),
		)])
		.await
		.map_err(|e| eyre::eyre!("Failed to save deck config for '{}': {}", deck_name, e))?;

	info!("Stored flash UUID in deck config for '{}'", deck_name);
	Ok(())
}
