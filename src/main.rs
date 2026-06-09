use std::path::PathBuf;

use clap::Parser;
use eyre::{Context, Result};
use flash::deck::Deck;
use tracing::info;
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

fn main() -> Result<()> {
	init();
	let args = Args::parse();

	info!("Starting Anki deck parser");

	let mut entities = Vec::with_capacity(args.decks.len());

	for deck_path in &args.decks {
		info!("Parsing deck at: {:?}", deck_path);
		let deck = Deck::from(deck_path.clone())
			.wrap_err_with(|| format!("Failed to parse deck at {:?}", deck_path))?;
		entities.push(deck.into());
	}

	let out = if entities.len() == 1 {
		sonic_rs::serde::to_string(&entities[0])?
	} else {
		sonic_rs::serde::to_string(&entities)?
	};

	fs::write(&args.output, out)
		.wrap_err_with(|| format!("Failed to write output to {:?}", args.output))?;

	info!("Wrote output to {:?}", args.output);
	info!("Deck parsing completed ({} deck(s))", args.decks.len());

	Ok(())
}

/// Create a template repository
// fn create_template(where: PathBuf, ) -> {
// 	let repo =	ThreadSafeRepository::init_opts(where, Kind::WithWorktree, Options::default(), Options::default());

// 	let flash_entry = where.join("index.flash");

// 	// Empt for now
// 	fs::write(flash_entry, "");
// }
