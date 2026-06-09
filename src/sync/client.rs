use core::ops::Deref;

use ankit::AnkiClient;
use serde_json::Value;

/// Wraps Anki client with a MUCH better API
pub struct FlashClient(AnkiClient);

impl FlashClient {
	/// Get the full information of an Anki profile
	pub fn get_full_information(&self) {
		let results = client
			.misc()
			.multi(&[
				MultiAction::with_params("getCollection", Value::Null),
				MultiAction::with_params("deckNamesAndIds", Value::Null),
				MultiAction::with_params("modelNamesAndIds", Value::Null),
			])
			.await
			.map_err(|e| eyre::eyre!("Failed to get collection snapshot: {}", e))?;
	}
}
