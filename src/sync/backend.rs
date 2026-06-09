use std::path::{Path, PathBuf};

use eyre::Result;
use serde::Serialize;
use uuid::Uuid;

use crate::{deck::Deck, sync::SyncEngine};

pub trait ExportBackend {
	async fn export(&mut self, deck_path: &Path, deck: &Deck<'_>) -> Result<()>;
	async fn finalize(&mut self) -> Result<()> { Ok(()) }
}

pub struct JsonBackend {
	output_path: PathBuf,
	decks:       Vec<JsonDeckData>,
}

#[derive(Serialize)]
struct JsonDeckData {
	name:        String,
	flash_uuid:  String,
	note_count:  usize,
	model_count: usize,
	card_ids:    Vec<Uuid>,
}

impl JsonBackend {
	pub fn new(output_path: PathBuf) -> Self { Self { output_path, decks: Vec::new() } }
}

impl ExportBackend for JsonBackend {
	async fn export(&mut self, _deck_path: &Path, deck: &Deck<'_>) -> Result<()> {
		let data = JsonDeckData {
			name:        deck.configuration.name.clone(),
			flash_uuid:  deck.configuration.flash_uuid.clone(),
			note_count:  deck.cards.len(),
			model_count: deck.models.len(),
			card_ids:    deck.cards.iter().map(|c| c.id).collect(),
		};
		self.decks.push(data);
		Ok(())
	}

	async fn finalize(&mut self) -> Result<()> {
		let json = serde_json::to_string_pretty(&self.decks)?;
		fs_err::write(&self.output_path, json)?;
		Ok(())
	}
}

pub enum AnyBackend {
	Json(JsonBackend),
	Sync(SyncEngine),
}

impl AnyBackend {
	pub async fn export(&mut self, deck_path: &Path, deck: &Deck<'_>) -> Result<()> {
		match self {
			AnyBackend::Json(b) => b.export(deck_path, deck).await,
			AnyBackend::Sync(b) => b.export(deck_path, deck).await,
		}
	}

	pub async fn finalize(&mut self) -> Result<()> {
		match self {
			AnyBackend::Json(b) => b.finalize().await,
			AnyBackend::Sync(b) => b.finalize().await,
		}
	}
}
