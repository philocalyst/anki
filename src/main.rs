use std::{error::Error, fs};

use tracing::{error, info, instrument, warn};

use crate::{deck_locator::DeckLocator, model_loader::ModelLoader, types::{deck::Deck, note::{Note, TextElement}}};

mod change_router;
mod deck_locator;
mod error;
mod model_loader;
mod parse;
mod types;
mod uuid_generator;
mod uuid_resolver;

#[instrument(skip(note))]
fn print_note_debug(note: &Note) {
	for field in &note.fields {
		info!("{} : {:?}", field.name, field.content);
	}
	if !note.tags.is_empty() {
		info!("Tags: {:?}", note.tags);
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
