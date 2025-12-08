use std::{borrow::Cow, fs};

use eyre::{Context, Result};
use flash::{self, change_resolver::resolve_changes, change_router::determine_changes, deck_locator::{find_deck_directory, scan_deck_contents}, model_loader, print_note_debug, types::{crowd_anki_models::CrowdAnkiEntity, deck::Deck, note::Note, note_methods::Identifiable}};
use fs_err::write;
use opentelemetry::trace::{Tracer, TracerProvider as _};
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
	let mut point = 0;

	// TODO: Pre-allocate, possibly switching away from Vecs altogether if
	// pre-parsing the final proves to be worth it?
	let mut last_cards = Vec::new();
	let mut static_cards = Vec::new();

	while point < history.len() {
		let (active_entry, active_commit) = history[point].clone();
		let content = deck.read_file_content(&active_entry).wrap_err("Failed to read file content")?;

		// Parse and immediately extract owned data
		let active_cards: Vec<Note> = deck
			.parse_cards(content.as_ref())
			.wrap_err("Failed to parse cards from history")?
			.into_iter()
			.map(|note| Note {
				fields: note.fields.clone(),
				model:  Cow::Owned(note.model.into_owned()),
				tags:   note.tags.clone(),
			})
			.collect();

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

		let possible_changes = determine_changes(last_cards.as_slice(), &active_cards)
			.wrap_err("Failed to determine changes")?;

		if let Some(changes) = possible_changes {
			// Assuming resolve_uuids mutates static_cards in place or returns new value
			// If it returns a new value:
			resolve_changes(&changes, &mut static_cards, Uuid::default());
		}

		last_cards = active_cards.clone();
		point += 1;
	}

	// Done with history
	drop(history);

	deck.cards = static_cards;

	let out: CrowdAnkiEntity = deck.into();

	let out = sonic_rs::serde::to_string(&out)?;

	write("flash.json", out)?;

	info!("Deck parsing completed");
	Ok(())
}
