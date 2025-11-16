use std::{error::Error, fs};

use flash::{self, change_resolver::{IdentifiedNote, resolve_changes}, change_router::determine_changes, deck_locator::DeckLocator, model_loader, print_note_debug, types::{deck::Deck, note::ONote}};
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

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
	let models = model_loader::load_models(&model_paths, &deck_path)?;

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
			info!("Generating UUIDs for notes in {}", "index.flash");

			// Generating against the initial point of creation for the file, taking into
			// account renames. This should keep things stable as long as the git repo is
			// the token of trade
			let history = deck.get_file_history("index.flash")?;
			let mut point = 0;
			let mut last_cards = Vec::new();
			let mut static_cards = Vec::new();

			while point < history.len() {
				let (active_entry, active_commit) = history[point].clone();
				let content = deck.read_file_content(&active_entry)?;

				// Parse and immediately extract owned data
				let active_cards: Vec<ONote> = deck
					.parse_cards(content.as_ref())?
					.into_iter()
					.map(|note| ONote { fields: note.fields.clone(), tags: note.tags.clone() })
					.collect();

				if point == 0 {
					// Generate initial set of UUIDs
					let uuids = deck.generate_note_uuids((active_entry, active_commit))?;
					static_cards = active_cards
						.iter()
						.zip(uuids)
						.map(|(card, id)| IdentifiedNote { id, note: card.clone() })
						.collect();
					last_cards = active_cards;
					point += 1;
					continue;
				}

				let possible_changes = determine_changes(&last_cards, &active_cards)?;

				if let Some(changes) = possible_changes {
					// Assuming resolve_uuids mutates static_cards in place or returns new value
					// If it returns a new value:
					resolve_changes(&changes, &mut static_cards, Uuid::default());
				}

				last_cards = active_cards;
				point += 1;
			}
		}
		Err(error) => {
			error!("Parsing error: {}", error);
		}
	}

	info!("Deck parsing completed");
	Ok(())
}
