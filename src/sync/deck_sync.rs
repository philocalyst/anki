use std::collections::HashSet;

use eyre::{Result, bail};
use rand::RngExt;
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
	config::DeckConfig,
	sync::{
		client::{FlashClient, unique_name},
		connection::CollectionSnapshot,
		identity::FLASH_DECK_UUID_KEY,
	},
};

pub struct DeckSyncData {
	pub uuid: Uuid,
	pub name: String,
	pub config: DeckConfig,
}

/// Sync a deck: create if new, update if existing.
/// Returns the deck name as it exists in Anki (may differ if renamed).
pub async fn sync_deck(
	client: &FlashClient,
	deck: &DeckSyncData,
	snapshot: &CollectionSnapshot,
) -> Result<String> {
	let uuid_str = deck.uuid.to_string();

	// Look up the deck by UUID stored in its config, to see if the clients anki already holds it
	let existing_name = find_external_deck(client, &deck.uuid).await?;

	if let Some(anki_name) = existing_name {
		info!("Deck '{}' found in Anki (uuid: {})", anki_name, uuid_str);

		// Ensure the deck exists (user may have deleted it)
		if !snapshot.decks.contains_key(&anki_name) {
			warn!("Deck '{}' not in snapshot, recreating", anki_name);
			client
				.decks()
				.create(&anki_name)
				.await
				.map_err(|e| eyre::eyre!("Failed to create deck '{}': {}", anki_name, e))?;
		}

		Ok(anki_name)
	} else {
		// No deck with our UUID found — check if a deck with our name already exists
		let existing: HashSet<String> = snapshot.decks.keys().cloned().collect();
		let target_name = unique_name(&deck.name, &existing);

		if target_name != deck.name {
			warn!("Deck name '{}' already taken, using '{}'", deck.name, target_name);
		}

		info!("Creating new deck '{}'", target_name);
		client
			.decks()
			.create(&target_name)
			.await
			.map_err(|e| eyre::eyre!("Failed to create deck '{}': {}", target_name, e))?;

		// Store UUID in deck config
		store_deck_uuid(client, &target_name, &deck.uuid).await?;

		Ok(target_name)
	}
}

/// Find a deck by its flash UUID stored in the deck config.
async fn find_external_deck(client: &FlashClient, local_uuid: &Uuid) -> Result<Option<String>> {
	let external_deck_names =
		client.decks().names().await.map_err(|e| eyre::eyre!("Failed to get deck names: {}", e))?;

	let uuid_str = local_uuid.to_string();

	for external_name in &external_deck_names {
		// TODO: Somehow validate that there's not duplicates?
		if let Some(config) = client.get_deck_config(external_name).await?
			&& let Some(found) = config.get(FLASH_DECK_UUID_KEY).and_then(|v| v.as_str())
			&& found == uuid_str
		{
			return Ok(Some(external_name.clone()));
		}
	}

	Ok(None)
}

/// Store the flash UUID in a deck's config so we can find it later.
/// Only used when setting up new decks
async fn store_deck_uuid(client: &FlashClient, deck_name: &str, deck_uuid: &Uuid) -> Result<()> {
	let mut config = match client.get_deck_config(deck_name).await? {
		Some(serde_json::Value::Object(map)) => map,
		_ => {
			bail!("Unexpected deck config format for '{}'", deck_name);
		}
	};

	// we need to set this to a random number to avoid "configuration group" conflicts (as they all contain unique ids).
	// if there's a cleaner way to store data in ANKI plese let me know
	// TODO: make this all typesafe because this corrupted my whole store bruh
	let id = config.get_mut(&String::from("id")).unwrap();
	let random_identifier = rand::rng().random::<u32>();
	*id = json!(random_identifier);

	// Insert our identifying key
	config.insert(FLASH_DECK_UUID_KEY.to_string(), serde_json::Value::String(deck_uuid.to_string()));

	client.save_deck_config(&config).await?;

	info!("Stored flash UUID in deck config for '{}'", deck_name);
	Ok(())
}
