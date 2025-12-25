use std::{borrow::Cow, path::{Path, PathBuf}};

use eyre::{Context, Result, eyre};
use flash::{change_resolver::resolve_changes, change_router::determine_changes, deck_locator::find_deck_directory, parse::ImportExpander, types::{deck::Deck, note::{Identified, Note}, note_methods::Identifiable}};
use gix::{Commit, object::tree::Entry};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_stdout::SpanExporter;
use tracing::{info, instrument, warn};
use tracing_subscriber::{Registry, fmt::{self, time::ChronoUtc}, prelude::__tracing_subscriber_SubscriberExt};
use uuid::Uuid;

pub fn init_opentelemetry_tracing() {
	// Create a new OpenTelemetry trace pipeline that prints to stdout
	let provider = SdkTracerProvider::builder().with_simple_exporter(SpanExporter::default()).build();
	let tracer = provider.tracer("readme_example");

	// Create a tracing layer with the configured OpenTelemetry tracer
	let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

	let fmt_layer =
		fmt::layer().with_target(false).with_timer(ChronoUtc::new("Sec.%S.Nanos.%f".to_string()));

	let subscriber = Registry::default()
        .with(telemetry_layer) // OpenTelemetry layer
        .with(fmt_layer); // Formatted console output layer
}

#[instrument]
fn main() -> Result<()> {
	init_opentelemetry_tracing();
	color_eyre::install()?;

	info!("Starting Anki deck parser");

	// Find and scan deck
	let deck_path = find_deck_directory().wrap_err("Failed to find deck directory")?;
	info!("Found deck at: {:?}", deck_path);

	let deck = Deck::from(deck_path)?;

	// Generating against the initial point of creation for the file, taking into
	// account renames. This should keep things stable as long as the git repo is
	// the token of trade
	let history = deck.get_file_history("index.flash").wrap_err("Failed to get file history")?;

	// Store all content strings so they live long enough
	let all_contents: Vec<String> =
		history.iter().map(|(entry, _)| get_content(&deck, entry)).collect::<Result<Vec<_>>>()?;

	let _static_cards = process_card_history(&deck, &history, &all_contents)?;

	// Done with history
	drop(history);

	info!("Deck parsing completed");
	Ok(())
}

// Parse cards from a string reference
fn parse_cards_from_content<'a>(deck: &'a Deck<'a>, content: &'a str) -> Result<Vec<Note<'a>>> {
	deck.parse_cards(content).wrap_err("Failed to parse cards")
}

// Initialize the first state with UUIDs
fn initialize_cards<'a>(
	deck: &Deck,
	entry: &Entry,
	commit: &Commit,
	cards: Vec<Note<'a>>,
) -> Result<Vec<Identified<Note<'a>>>> {
	// Generate initial set of UUIDs
	let uuids = deck
		.generate_note_uuids((entry.clone(), commit.clone()))
		.wrap_err("Failed to generate UUIDs")?;

	Ok(cards.into_iter().zip(uuids).map(|(card, id)| card.identified(id)).collect())
}

/// Interpret the passing of a cycle
fn process_cycle(
	last_cards: &[Note],
	current_cards: &[Note],
	static_cards: &mut Vec<Identified<Note>>,
) -> Result<()> {
	// It might be that a change was made but nothing of note happened, like a misc.
	// newline, check for this.
	if let Some(changes) =
		determine_changes(last_cards, current_cards).wrap_err("Failed to determine changes")?
	{
		// Assuming resolve_uuids mutates static_cards in place or returns new value
		// If it returns a new value:
		resolve_changes(&changes, static_cards, Uuid::default());
	}
	Ok(())
}

fn get_content(deck: &Deck, entry: &Entry) -> Result<String> {
	let file: PathBuf =
		deck.backing_vcs.git_dir().parent().unwrap().join(PathBuf::from(entry.filename().to_string()));

	// Expand all imports first
	let mut expander = ImportExpander::new(file.parent().unwrap_or_else(|| Path::new(".")));

	Ok(expander.expand_file(file).unwrap())
}

// Main processing logic
fn process_card_history<'a>(
	deck: &'a Deck<'_>,
	history: &[(Entry, Commit)],
	all_contents: &'a [String],
) -> Result<Vec<Identified<Note<'a>>>> {
	let mut history_iter = history.iter();

	// Handle first entry separately
	let (first_entry, first_commit) = history_iter.next().ok_or_else(|| eyre!("History is empty"))?;

	let first_cards = parse_cards_from_content(deck, &all_contents[0])?;

	// Blankly initialize, as we immediately overwrite
	let mut bygone_cards = Vec::with_capacity(first_cards.len());

	let mut elder_cards = initialize_cards(deck, first_entry, first_commit, first_cards)?;

	// Process remaining entries
	for (idx, (entry, _commit)) in history_iter.enumerate() {
		let cards_of_the_day = parse_cards_from_content(deck, &all_contents[idx + 1])?;

		// Make a diff of the changes and update the final cards appropriately
		process_cycle(&bygone_cards, &cards_of_the_day, &mut elder_cards)?;

		// Cycle complete, the once-new cards lose their youth.
		bygone_cards = cards_of_the_day;
	}

	Ok(elder_cards)
}
