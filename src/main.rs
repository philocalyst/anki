#![allow(async_fn_in_trait)]

use std::path::PathBuf;

use clap::Parser;
use eyre::{Context, Result};
use flash::{deck::Deck, ;use tracing::info;
use tracing_subscriber::EnvFilter;

fn init_tracing() {
	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
	tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();
}

/// Sync flashcard decks to Anki via Anki-Connect-Plus
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Paths to deck directories to parse and sync.
	#[arg(
        value_name = "DECK_PATH",
        value_hint = clap::ValueHint::DirPath,
        num_args = 1..,
        required = true,
    )]
	decks: Vec<PathBuf>,

	/// Output the parsed deck content as JSON to the specified file.
	#[arg(long, value_name = "FILE")]
	output_json: Option<PathBuf>,

	/// Skip deletion reconciliation (keep orphaned notes in Anki).
	#[arg(long)]
	no_prune: bool,

	/// Skip media file syncing.
	#[arg(long)]
	no_media: bool,
}

fn init() -> Result<()> {
	init_tracing();
	color_eyre::install()?;

	Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
	init()?;
	let args = Args::parse();

	info!("Starting flash sync for {} deck(s)", args.decks.len());

	// Parse all decks
	let mut decks = Vec::with_capacity(args.decks.len());
	for deck_path in &args.decks {
		info!("Parsing deck at: {:?}", deck_path);
		let deck = Deck::from(deck_path.clone())
			.wrap_err_with(|| format!("Failed to parse deck at {:?}", deck_path))?;
		decks.push((deck_path.clone(), deck));
	}

	// Connect to Anki and sync all decks
	let mut backend = if let Some(output_path) = args.output_json {
		info!("Initializing JSON export backend...");
		flash::sync::AnyBackend::Json(flash::sync::JsonBackend::new(output_path))
	} else {
		info!("Initializing Anki sync engine...");
		flash::sync::AnyBackend::Sync(flash::sync::SyncEngine::new().await?)
	};

	for (deck_path, deck) in &decks {
		backend.export(deck_path, deck).await?;
	}

	backend.finalize().await?;

	info!("Sync completed for {} deck(s)", decks.len());
	Ok(())
}
