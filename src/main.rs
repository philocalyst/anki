use std::{error::Error, fmt, fs, path::{Path, PathBuf}};

use chumsky::Parser;
use gix::{Commit, Repository, Tree, bstr::{ByteSlice, ByteVec}, object::tree::Entry};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{deck_locator::DeckLocator, error::DeckError, model_loader::ModelLoader, parse::parser, types::{crowd_anki_models::Deck, note::{Note, NoteModel, TextElement}}};

mod deck;
mod deck_locator;
mod error;
mod model_loader;
mod parse;
mod types;

/// Generate a deterministic string representation of the note's content
/// for UUID generation
#[instrument(skip(note))]
fn note_to_content_string(note: &Note) -> String {
	let mut content = String::new();

	for field in &note.fields {
		content.push_str(&field.name);

		let field_content = field
			.content
			.iter()
			.map(|part| match part {
				TextElement::Text(text) => text.as_str(),
				TextElement::Cloze(cloze) => cloze.answer.as_str(),
			})
			.collect::<Vec<&str>>()
			.join("\0");

		content.push_str(&field_content);
	}

	content
}

#[instrument(skip(note))]
fn print_note_debug(note: &Note) {
	for field in &note.fields {
		info!("{} : {:?}", field.name, field.content);
	}
	if !note.tags.is_empty() {
		info!("Tags: {:?}", note.tags);
	}
}

struct UuidGenerator;

impl UuidGenerator {
	/// Creates the main UUID based on the author of the initial commit and the
	/// time
	#[instrument]
	fn create_host_uuid(author: String, time: i64) -> Uuid {
		debug!("Creating host UUID for author: {}, time: {}", author, time);

		// Note: This is fragile and will break under rebase conditions
		// This is inherent to the design for deterministic generation
		let namespace = format!("{}{}", author, time);
		Uuid::new_v5(&Uuid::NAMESPACE_DNS, namespace.as_bytes())
	}

	/// Generate a UUID for a specific note based on its content
	#[instrument(skip(content))]
	fn generate_note_uuid(host_uuid: &Uuid, content: &str) -> Uuid {
		Uuid::new_v5(host_uuid, content.as_bytes())
	}
}

#[instrument]
fn main() -> Result<(), Box<dyn Error>> {
	// Initialize tracing
	tracing_subscriber::fmt().with_target(false).with_level(true).init();

	info!("Starting Anki deck parser");

	// Find and scan deck
	let deck_path = DeckLocator::find_deck_directory()?;
	info!("Found deck at: {:?}", deck_path);

	let (model_paths, card_paths) = DeckLocator::scan_deck_contents(&deck_path)?;

	if card_paths.is_empty() {
		warn!("No card files found");
		return Ok(());
	}

	// Load models
	let models = ModelLoader::load_models(&model_paths, &deck_path)?;

	// Open repository
	let repo_path = deck_path.join(".git");
	info!("Opening repository at: {:?}", repo_path);
	let backing_vcs = gix::open(repo_path)?;

	// Create deck
	let deck = Deck::new(models, backing_vcs);

	// Parse first card file as example
	let first_card_path = &card_paths[0];
	info!("Parsing card file: {:?}", first_card_path);
	let card_content = fs::read_to_string(first_card_path)?;

	let parse_result = deck.parse_cards(&card_content);

	match parse_result {
		Ok(cards) => {
			info!("Successfully parsed {} cards", cards.len());
			for card in &cards {
				print_note_debug(card);
			}

			// Generate UUIDs
			let uuids = deck.generate_note_uuids("index.flash")?; // TODO: Make this run per the card file
			info!("Generated UUIDs:");

			for uuid in uuids {
				info!("{}", uuid);
			}
		}
		Err(error) => {
			error!("Parsing error: {}", error);
		}
	}

	info!("Deck parsing completed");
	Ok(())
}
