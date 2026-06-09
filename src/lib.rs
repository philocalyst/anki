use tracing::{info, instrument, warn};

use crate::note::Note;

pub mod change_resolver;
pub mod change_router;
pub mod config;
pub mod deck;
pub mod deck_locator;
pub mod error;
pub mod model_catalog;
pub mod model_loader;
pub mod note;
pub mod note_id_generator;
pub mod parser;
pub mod sync;
pub mod uuid_generator;

#[instrument(skip(note))]
pub fn print_note_debug(note: &Note) {
	for field in &note.fields {
		info!("{} : {:?}", field.name, field.content);
	}
	if !note.tags.is_empty() {
		info!("Tags: {:?}", note.tags);
	}
}
