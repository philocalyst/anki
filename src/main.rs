use std::fs;

use eyre::{Context, Result};
use flash::{self, change_resolver::resolve_changes, change_router::determine_changes, deck_locator::{find_deck_directory, scan_deck_contents}, model_loader, print_note_debug, types::{deck::Deck, note::{Identified, Note}, note_methods::Identifiable}};
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

	let mut all_contents: Vec<String> = Vec::new();
	let mut all_cards: Vec<Vec<Note>> = Vec::new();
	let mut last_cards: Vec<&Note> = Vec::new();
	let mut static_cards: Vec<Identified<&Note>> = Vec::new();

	for entry in deck.read_journal_entries()? {
		let content = deck.read_file_content(&entry).wrap_err("Failed to read file content")?;

		all_contents.push(content);
		let content_ref = all_contents.last().unwrap();

		let active_cards: Vec<Note> =
			deck.parse_cards(content_ref).wrap_err("Failed to parse cards from history")?;

		all_cards.push(active_cards);
		let active_cards_ref: Vec<&Note> = all_cards.last().unwrap().iter().collect();

		if let Some(changes) =
			determine_changes(&last_cards, &active_cards_ref).wrap_err("Failed to determine changes")?
		{
			resolve_changes(&changes, &mut static_cards, Uuid::default());
		}

		last_cards = active_cards_ref;
	}
	dbg!(&static_cards);

	info!("Deck parsing completed");
	Ok(())
}
