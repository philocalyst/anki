use std::collections::HashSet;

use ankit::AnkiClient;
use eyre::Result;
use tracing::{info, warn};
use uuid::Uuid;

use crate::sync::identity::{parse_note_uuid_from_tag, flash_tag_wildcard};

/// Delete notes from Anki that have flash tags but whose UUIDs are not in the
/// current set. This handles the case where a note was removed from a .flash file.
pub async fn reconcile_deletions(
	client: &AnkiClient,
	deck_name: &str,
	deck_uuid: &Uuid,
	current_uuids: &HashSet<Uuid>,
) -> Result<()> {
	let query = format!("deck:\"{}\" {}", deck_name, flash_tag_wildcard(deck_uuid));

	let tagged_ids = client
		.notes()
		.find(&query)
		.await
		.map_err(|e| eyre::eyre!("Failed to find tagged notes in deck '{}': {}", deck_name, e))?;

	if tagged_ids.is_empty() {
		return Ok(());
	}

	// Get note info to extract UUIDs from tags
	let notes_info = client
		.notes()
		.info(&tagged_ids)
		.await
		.map_err(|e| eyre::eyre!("Failed to get note info: {}", e))?;

	let mut to_delete = Vec::new();

	for info in &notes_info {
		let found_uuid = info.tags.iter().find_map(|tag| parse_note_uuid_from_tag(tag));

		match found_uuid {
			Some(uuid) if !current_uuids.contains(&uuid) => {
				warn!(
					"Note '{}' (uuid: {}) no longer in flash deck — will delete",
					info.note_id, uuid
				);
				to_delete.push(info.note_id);
			}
			_ => {} // Keep: either UUID matches or we can't parse it
		}
	}

	if !to_delete.is_empty() {
		info!("Deleting {} orphaned note(s) from deck '{}'", to_delete.len(), deck_name);
		client
			.notes()
			.delete(&to_delete)
			.await
			.map_err(|e| eyre::eyre!("Failed to delete orphaned notes: {}", e))?;
	}

	Ok(())
}
