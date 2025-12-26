use std::{fs, path::{Path, PathBuf}};

use eyre::{Context, Result, eyre};
use flash::{change_resolver::resolve_changes, change_router::determine_changes, deck_locator::find_deck_directory, parse::ImportExpander, types::{crowd_anki_models::CrowdAnkiEntity, deck::Deck, note::{Identified, Note}, note_methods::Identifiable}};
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

	let out: CrowdAnkiEntity = deck.into();

	let out = sonic_rs::serde::to_string(&out)?;

	fs::write("flash.json", out)?;

	info!("Deck parsing completed");
	Ok(())
}
