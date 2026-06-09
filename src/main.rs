use std::path::PathBuf;

use clap::Parser;
use eyre::{Context, Result};
use flash::deck::Deck;
use tracing::info;
use tracing_subscriber::EnvFilter;

use flash::sync::SyncEngine;

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
	info!("Initializing sync engine...");
	let mut engine = SyncEngine::new().await?;

	// TODO: Move behind a trait for different export backends?
	for (deck_path, deck) in &decks {
		let deck_uuid = deck
			.configuration
			.flash_uuid
			.parse()
			.wrap_err_with(|| format!("Invalid deck UUID: {}", deck.configuration.flash_uuid))?;

		engine
			.sync(
				deck_path,
				&deck_uuid,
				&deck.configuration.name,
				&deck.models,
				&deck.cards,
				&deck.configuration,
			)
			.await?;
	}

	info!("Sync completed for {} deck(s)", decks.len());
	Ok(())
}

/// Create a template repository
// fn create_template(where: PathBuf, ) -> {
// 	let repo =	ThreadSafeRepository::init_opts(where, Kind::WithWorktree, Options::default(), Options::default());

// 	let flash_entry = where.join("index.flash");

// 	// Empt for now
// 	fs::write(flash_entry, "");
// }
