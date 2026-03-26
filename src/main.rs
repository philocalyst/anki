use std::{
	fs,
	path::{Path, PathBuf},
};

use eyre::{Context, Result, eyre};
use flash::{
	change_resolver::resolve_changes,
	change_router::determine_changes,
	deck_locator::find_deck_directory,
	parse::ImportExpander,
	types::{
		crowd_anki_models::CrowdAnkiEntity,
		deck::Deck,
		note::{Identified, Note},
		note_methods::Identifiable,
	},
};
use gix::{Commit, object::tree::Entry};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_stdout::SpanExporter;
use tracing::{
	info,
	instrument::{self, WithSubscriber},
	warn,
};
use tracing_subscriber::{
	EnvFilter, Registry,
	fmt::{self, time::ChronoUtc},
	prelude::__tracing_subscriber_SubscriberExt,
};
use uuid::Uuid;

pub fn init_tracing() {
	// Uses RUST_LOG if set; otherwise default to info.
	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

	tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();
}

fn main() -> Result<()> {
	init_tracing();
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
