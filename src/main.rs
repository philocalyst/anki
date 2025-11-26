use std::fs;

use eyre::{Context, Result};
use flash::{self, change_resolver::resolve_changes, change_router::determine_changes, deck_locator::{find_deck_directory, scan_deck_contents}, model_loader, print_note_debug, types::{deck::Deck, note::Note, note_methods::Identifiable}};
use tracing::{info, instrument, warn};
use uuid::Uuid;

#[instrument]
fn main() -> Result<()> {
	// Initialize tracing
	tracing_subscriber::fmt().with_target(false).with_level(true).init();
	color_eyre::install()?;

	info!("Starting Anki deck parser");

	// Find and scan deck
	let deck_path = find_deck_directory().wrap_err("Failed to find deck directory")?;
	info!("Found deck at: {:?}", deck_path);

	let (model_paths, card_paths) =
		scan_deck_contents(&deck_path).wrap_err("Failed to scan deck contents")?;

	if card_paths.is_empty() {
		warn!("No card files found");
		return Ok(());
	}

	// Load models
	let models =
		model_loader::load_models(&model_paths, &deck_path).wrap_err("Failed to load models")?;

	// Open repository
	let repo_path = deck_path.join(".git");
	info!("Opening repository at: {:?}", repo_path);
	let backing_vcs = gix::open(repo_path).wrap_err("Failed to open git repository")?;

	// Create deck
	let deck = Deck::new(models, backing_vcs);

	// Parse first card file as example
	let first_card_path = &card_paths[0];
	info!("Parsing card file: {:?}", first_card_path);
	let card_content = fs::read_to_string(first_card_path).wrap_err("Failed to read card file")?;

	let cards = deck.parse_cards(&card_content).wrap_err("Failed to parse cards")?;

	info!("Successfully parsed {} cards", cards.len());
	for card in &cards {
		print_note_debug(card);
	}
	info!("Generating UUIDs for notes in {}", "index.flash");

	// Generating against the initial point of creation for the file, taking into
	// account renames. This should keep things stable as long as the git repo is
	// the token of trade
	let history = deck.get_file_history("index.flash").wrap_err("Failed to get file history")?;
	let mut point = 0;
	let mut last_cards = Vec::new();
	let mut static_cards = Vec::new();

	while point < history.len() {
		let (active_entry, active_commit) = history[point].clone();
		let content = deck.read_file_content(&active_entry).wrap_err("Failed to read file content")?;

		// Parse and immediately extract owned data
		let active_cards: Vec<Note> =
			deck.parse_cards(&content).wrap_err("Failed to parse cards from history")?;

		if point == 0 {
			// Generate initial set of UUIDs
			let uuids = deck
				.generate_note_uuids((active_entry, active_commit))
				.wrap_err("Failed to generate UUIDs")?;
			static_cards =
				active_cards.iter().zip(uuids).map(|(card, id)| card.clone().identified(id)).collect();
			last_cards = active_cards;
			point += 1;
			continue;
		}

		let possible_changes =
			determine_changes(&last_cards, &active_cards).wrap_err("Failed to determine changes")?;

		if let Some(changes) = possible_changes {
			// Assuming resolve_uuids mutates static_cards in place or returns new value
			// If it returns a new value:
			resolve_changes(&changes, &mut static_cards, Uuid::default());
		}

		last_cards = active_cards;
		point += 1;
	}
	dbg!(&static_cards);

	info!("Deck parsing completed");
	Ok(())
}
