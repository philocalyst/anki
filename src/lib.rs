use std::{error::Error, fs};

use tracing::{error, info, instrument, warn};

use crate::{deck_locator::DeckLocator, types::{deck::Deck, note::{Note, TextElement}}, uuid_resolver::{IdentifiedNote, resolve_uuids}};

pub mod change_router;
pub mod deck_locator;
pub mod error;
pub mod model_loader;
pub mod parse;
pub mod types;
pub mod uuid_generator;
pub mod uuid_resolver;

#[instrument(skip(note))]
pub fn print_note_debug(note: &Note) {
	for field in &note.fields {
		info!("{} : {:?}", field.name, field.content);
	}
	if !note.tags.is_empty() {
		info!("Tags: {:?}", note.tags);
	}
}
