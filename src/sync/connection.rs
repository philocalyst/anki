use std::collections::HashMap;

use ankit::{AnkiClient, actions::MultiAction};
use eyre::{Result, bail};
use serde_json::Value;
use tracing::info;

use crate::sync::{client::FlashClient, identity::FLASH_DECK_UUID_KEY};

pub async fn check_connection(client: &AnkiClient) -> Result<()> {
	let version = client.misc().version().await.map_err(|e| {
		eyre::eyre!("Cannot connect to Anki via AnkiConnect: {}. Make sure Anki is running with Anki-Connect-Plus installed.", e)
	})?;

	if version < 6 {
		bail!("AnkiConnect version {} is too old, need version 6+", version);
	}

	info!("Connected to Anki (AnkiConnect API v{})", version);
	Ok(())
}

pub async fn get_collection_snapshot(client: &AnkiClient) -> Result<CollectionSnapshot> {
	let results = client
		.misc()
		.multi(&[
			MultiAction::with_params("getCollection", Value::Null),
			MultiAction::with_params("deckNamesAndIds", Value::Null),
			MultiAction::with_params("modelNamesAndIds", Value::Null),
		])
		.await
		.map_err(|e| eyre::eyre!("Failed to get collection snapshot: {}", e))?;

	if results.len() < 3 {
		bail!("Unexpected response from multi action");
	}

	let snapshot = CollectionSnapshot {
		decks: parse_ids(&results[1]),
		models: parse_ids(&results[2]),
		collection: results[0].clone(),
	};

	Ok(snapshot)
}

pub struct CollectionSnapshot {
	pub decks: HashMap<String, i64>,
	pub models: HashMap<String, i64>,
	pub collection: Value,
}

fn parse_ids(value: &Value) -> HashMap<String, i64> {
	fn to_ids(obj: &serde_json::Map<String, Value>) -> HashMap<String, i64> {
		obj.iter().filter_map(|(name, id)| id.as_i64().map(|id| (name.clone(), id))).collect()
	}

	value.as_object().map(|obj| to_ids(obj)).unwrap_or_default()
}

/// Find a deck by its flash_uuid stored in the deck config.
/// Returns the deck name if found.
pub async fn find_deck_by_uuid(
	client: &AnkiClient,
	deck_uuid: &uuid::Uuid,
	snapshot: &CollectionSnapshot,
) -> Result<Option<String>> {
	let uuid_str = deck_uuid.to_string();

	// Try to find by matching snapshot deck names with our lookup
	for deck_name in snapshot.decks.keys() {
		let results = client
			.misc()
			.multi(&[MultiAction::with_params("getDeckConfig", serde_json::json!({"deck": deck_name}))])
			.await
			.map_err(|e| eyre::eyre!("Failed to get deck config for '{}': {}", deck_name, e))?;

		if let Some(config) = results.into_iter().next() {
			if let Some(found_uuid) = config.get(FLASH_DECK_UUID_KEY).and_then(|v| v.as_str()) {
				if found_uuid == uuid_str {
					return Ok(Some(deck_name.clone()));
				}
			}
		}
	}

	Ok(None)
}

#[cfg(test)]
mod tests {
	use serde_json::json;

	use super::{parse_deck_ids, parse_model_ids};

	#[test]
	fn parse_deck_ids_happy_path() {
		let value = json!({
			"Default": 1,
			"MyDeck": 42,
			"Spanish::Verbs": 100
		});
		let result = parse_deck_ids(&value);
		assert_eq!(result.len(), 3);
		assert_eq!(result.get("Default"), Some(&1));
		assert_eq!(result.get("MyDeck"), Some(&42));
		assert_eq!(result.get("Spanish::Verbs"), Some(&100));
	}

	#[test]
	fn parse_deck_ids_empty_object() {
		let value = json!({});
		let result = parse_deck_ids(&value);
		assert!(result.is_empty());
	}

	#[test]
	fn parse_deck_ids_null_returns_empty() {
		let result = parse_deck_ids(&serde_json::Value::Null);
		assert!(result.is_empty());
	}

	#[test]
	fn parse_deck_ids_non_object_returns_empty() {
		let result = parse_deck_ids(&json!("not an object"));
		assert!(result.is_empty());
		let result = parse_deck_ids(&json!(42));
		assert!(result.is_empty());
	}

	#[test]
	fn parse_deck_ids_skips_non_integer_values() {
		let value = json!({
			"Valid": 1,
			"Bad": "string",
			"AlsoBad": null,
			"AlsoValid": 99
		});
		let result = parse_deck_ids(&value);
		assert_eq!(result.len(), 2);
		assert_eq!(result.get("Valid"), Some(&1));
		assert_eq!(result.get("AlsoValid"), Some(&99));
	}

	#[test]
	fn parse_model_ids_happy_path() {
		let value = json!({
			"Basic": 12345,
			"Cloze": 67890
		});
		let result = parse_model_ids(&value);
		assert_eq!(result.len(), 2);
		assert_eq!(result.get("Basic"), Some(&12345));
		assert_eq!(result.get("Cloze"), Some(&67890));
	}

	#[test]
	fn parse_model_ids_empty() {
		assert!(parse_model_ids(&json!({})).is_empty());
		assert!(parse_model_ids(&serde_json::Value::Null).is_empty());
	}

	#[test]
	fn parse_model_ids_skips_invalid_entries() {
		let value = json!({
			"Valid": 1,
			"BadFloat": 1.5,
			"BadString": "x"
		});
		let result = parse_model_ids(&value);
		assert_eq!(result.len(), 1);
	}
}
