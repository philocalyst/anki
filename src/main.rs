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

	let mut deck = Deck::from(deck_path)?;

	// Generating against the initial point of creation for the file, taking into
	// account renames. This should keep things stable as long as the git repo is
	// the token of trade
	let history = deck.get_file_history("index.flash").wrap_err("Failed to get file history")?;

	let static_cards = process_card_history(&deck, &history)?;

	// Done with history
	drop(history);

	deck.cards = static_cards;

	info!("Deck parsing completed");
	Ok(())
}

// Extract card reading and parsing into a function
fn read_and_parse_cards<'a>(deck: &Deck, entry: &Entry) -> Result<Vec<Note<'a>>> {
	let file: PathBuf =
		deck.backing_vcs.git_dir().parent().unwrap().join(PathBuf::from(entry.filename().to_string()));
	let content =
		deck.read_file_content(&entry.try_into()?).wrap_err("Failed to read file content")?;

	// Expand all imports first
	let mut expander = ImportExpander::new(file.parent().unwrap_or_else(|| Path::new(".")));

	let expanded_content = expander.expand_file(file).unwrap();

	// Parse and immediately extract owned data
	Ok(
		deck.parse_cards(&expanded_content)
        .wrap_err("Failed to parse cards from history")?
        .into_iter()
        // Hard copy for ownership concerns
        .map(|note| Note {
            fields: note.fields,
            model: Cow::Owned(note.model.into_owned()),
            tags: note.tags,
        })
        .collect(),
	)
}

// Initialize the first state with UUIDs
fn initialize_cards<'a, 'b>(
	deck: &Deck,
	entry: &Entry,
	commit: &Commit,
	cards: &'b [Note<'a>],
) -> Result<Vec<Identified<Note<'a>>>> {
	// Generate initial set of UUIDs
	let uuids = deck
		.generate_note_uuids((entry.clone(), commit.clone()))
		.wrap_err("Failed to generate UUIDs")?;

	Ok(cards.iter().zip(uuids).map(|(card, id)| card.clone().identified(id)).collect())
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

// Main processing logic
fn process_card_history<'a>(
	deck: &Deck,
	history: &[(Entry, Commit)],
) -> Result<Vec<Identified<Note<'a>>>> {
	let mut history_iter = history.iter();

	// Handle first entry separately
	let (first_entry, first_commit) = history_iter.next().ok_or_else(|| eyre!("History is empty"))?;

	let first_cards = read_and_parse_cards(deck, first_entry)?;
	let mut elder_cards = initialize_cards(deck, first_entry, first_commit, &first_cards)?;

	// Blankly initalize, as we immeidately overwrite
	let mut bygone_cards = Vec::with_capacity(first_cards.len());

	// Process remaining entries
	for (entry, _commit) in history_iter {
		let cards_of_the_day = read_and_parse_cards(deck, entry)?;

		// Make a diff of the changes and update the final cards appropriately
		process_cycle(&bygone_cards, &cards_of_the_day, &mut elder_cards)?;

		// Cycle complete, the once-new cards lose their youth.
		bygone_cards = cards_of_the_day;
	}

	Ok(elder_cards)
}
