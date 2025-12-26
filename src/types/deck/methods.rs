use std::{fs, path::{Path, PathBuf}};

use chumsky::{Parser, input::Input, span::SimpleSpan};
use gix::{Commit, Repository, Tree, object::tree::Entry};
use logos::Logos;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{change_resolver::resolve_changes, change_router::determine_changes, deck_locator::scan_deck_contents, error::DeckError, model_loader, parse::{ImportExpander, Token, flash}, types::{BEntry, crowd_anki_config::DeckConfig, deck::Deck, note::{Identified, Note, NoteModel}, note_methods::Identifiable}, uuid_generator};

pub fn get_file_history<'a>(
	vcs: &'a Repository,
	target: &str,
) -> Result<Vec<(gix::object::tree::Entry<'a>, gix::Commit<'a>)>, DeckError> {
	info!("Finding history of file: {}", target);

	let mut history = Vec::new();
	let mut head = vcs.head()?;
	let revwalk = vcs.rev_walk([head.peel_to_object()?.id()]);

	for commit_id in revwalk.all()? {
		let commit_id = commit_id?;
		let commit = vcs.find_commit(commit_id.id())?;
		let tree = commit.tree()?;

		// Check if file exists in this commit
		let current_entry = tree.lookup_entry_by_path(target)?.filter(|e| e.mode().is_blob());

		if current_entry.is_none() {
			continue; // File doesn't exist in this commit
		}

		let current_entry = current_entry.unwrap();
		let parent_ids: Vec<_> = commit.parent_ids().collect();

		if parent_ids.is_empty() {
			// Initial commit with the file
			info!("File created in initial commit {}", commit.id());
			history.push((current_entry, commit));
			continue;
		}

		// Check if file was added or modified compared to ANY parent
		let mut file_changed = false;

		for parent_id in parent_ids {
			let parent_commit = vcs.find_commit(parent_id)?;
			let parent_tree = parent_commit.tree()?;
			let parent_entry = parent_tree.lookup_entry_by_path(target)?.filter(|e| e.mode().is_blob());

			match parent_entry {
				None => {
					// File didn't exist in this parent - it was added
					file_changed = true;
					info!("File added in commit {} (from parent {})", commit.id(), parent_id);
					break;
				}
				Some(entry) => {
					// File exists in parent - check if it changed
					if entry.oid() != current_entry.oid() {
						file_changed = true;
						break;
					}
				}
			}
		}

		if file_changed {
			history.push((current_entry, commit));
		}
	}

	// Reverse to get chronological order (oldest first)
	history.reverse();

	if history.is_empty() {
		error!("File not found in repository history");
		Err(DeckError::FileNotInHistory(target.to_string()))
	} else {
		info!("Found {} commits in file history", history.len());
		Ok(history)
	}
}

impl<'b> super::Deck<'b> {
	#[instrument(skip(deck_path))]
	pub fn from<P: AsRef<Path>>(deck_path: P) -> Result<Self, DeckError> {
		let deck_path = deck_path.as_ref();
		info!("Initializing deck from: {:?}", deck_path);

		// Scan deck contents
		let (model_paths, card_paths) = scan_deck_contents(deck_path)
			.map_err(|e| DeckError::DeckInit(format!("Failed to scan deck contents: {}", e)))?;

		if card_paths.is_empty() {
			warn!("No card files found in deck directory");
		}

		// Load models
		let models = model_loader::load_models(&model_paths, deck_path)
			.map_err(|e| DeckError::DeckInit(format!("Failed to load models: {}", e)))?;

		info!("Loaded {} models", models.len());

		// Open Git repository
		let repo_path = deck_path.join(".git");
		debug!("Opening repository at: {:?}", repo_path);
		let backing_vcs = gix::open(repo_path)
			.map_err(|e| DeckError::DeckInit(format!("Failed to open git repository: {}", e)))?;

		// Load or create default configuration
		let config_path = deck_path.join("config.toml");

		let config_content = fs::read_to_string(&config_path)
			.map_err(|_| DeckError::DeckConfigNotFound(config_path.clone()))?;

		let configuration: DeckConfig = toml::from_str(&config_content)?;

		// Generating against the initial point of creation for the file, taking into
		// account renames. This should keep things stable as long as the git repo is
		// the token of trade
		let vcs = backing_vcs.clone();
		let history = get_file_history(&vcs, "index.flash")?;

		// Store all content strings so they live long enough
		let content: Vec<String> = history
			.iter()
			.map(|(entry, _)| get_content(&backing_vcs, entry))
			.collect::<Result<Vec<_>, DeckError>>()?;

		// SAFETY: We use unsafe here to work around Rust's self-referential struct
		// limitations. The cards will contain references to models and content. We
		// construct the cards first with a temporary lifetime, then move everything
		// into the Deck together. The safety invariant is: as long as the Deck
		// exists, models and content exist, so the references in cards remain valid
		// for the lifetime 'b of the Deck.
		let cards = unsafe {
			// Create cards with an arbitrary lifetime
			let models_ref: &[NoteModel] = &models;
			let content_ref: &[String] = &content;

			// Process with temporary lifetime
			let temp_cards = process_card_history(models_ref, content_ref, &backing_vcs, &history)?;

			// Transmute to the target lifetime 'b
			// This is safe because we're about to move models and content into the Deck,
			// and the cards will be moved along with them
			std::mem::transmute::<Vec<Identified<Note<'_>>>, Vec<Identified<Note<'b>>>>(temp_cards)
		};

		info!("Deck initialized successfully");
		Ok(Self { models, backing_vcs, cards, configuration, content })
	}

	#[instrument(skip(backing_vcs))]
	pub fn new(
		models: Vec<NoteModel>,
		backing_vcs: Repository,
		cards: Vec<Identified<Note<'b>>>,
		configuration: DeckConfig,
		content: Vec<String>,
	) -> Self {
		info!("Creating deck with {} models", models.len());
		Self { models, backing_vcs, cards, configuration, content }
	}

	#[instrument(skip(self))]
	pub fn find_model(&self, name: &str) -> Result<&NoteModel, DeckError> {
		debug!("Looking for model: {}", name);
		self.models.iter().find(|model| model.name == name).ok_or_else(|| {
			warn!("Model '{}' not found", name);
			DeckError::ModelNotFound(name.to_string())
		})
	}

	pub fn parse_cards<'a>(
		models: &'a [NoteModel],
		content: &'a str,
	) -> Result<Vec<Note<'a>>, DeckError> {
		debug!("Parsing card content");

		// Create the lexer
		let token_iter = Token::lexer(content).spanned().map(|(tok, span)| match tok {
			Ok(t) => (t, span.into()),
			Err(_) => (Token::Error, span.into()),
		});

		// Turn the iterator into a Chumsky-compatible stream
		// We provide a zero-width span at the end of the content for EOI (End Of Input)
		let eoi = SimpleSpan::from(content.len()..content.len());
		let token_stream = chumsky::input::Stream::from_iter(token_iter).map(eoi, |(t, s)| (t, s));

		// Parse the stream using the refactored flash parser
		flash(models).parse(token_stream).into_result().map_err(|e| {
			let error_string =
				e.into_iter().map(|e| format!("at {:?}: ", e)).collect::<Vec<_>>().join("\n");
			DeckError::Parse(error_string)
		})
	}

	#[instrument(skip(self, parent_tree, current_tree))]
	pub fn track_file_changes(
		&self,
		parent_tree: &Tree,
		current_tree: &Tree,
		path: &str,
	) -> Result<(), DeckError> {
		let parent_entry = parent_tree.lookup_entry_by_path(path)?.ok_or(DeckError::InvalidEntry)?;
		let current_entry = current_tree.lookup_entry_by_path(path)?.ok_or(DeckError::InvalidEntry)?;

		if parent_entry.id() != current_entry.id() {
			debug!("File modified: {}", path);
		}

		Ok(())
	}

	#[instrument(skip(backing_vcs))]
	pub fn read_file_content(backing_vcs: &Repository, entry: &BEntry) -> Result<String, DeckError> {
		// Retrieve the entries binary representation from the VCS and serialize as UTF8
		let binary_blob = backing_vcs.find_blob(entry.0.id())?;
		let content = String::from_utf8(binary_blob.data.clone()).map_err(|_| {
			DeckError::InvalidUtf8(backing_vcs.workdir().expect("Worktree should be checked out").into())
		})?;
		Ok(content)
	}

	#[instrument(skip(models, backing_vcs))]
	pub fn generate_note_uuids(
		models: &[NoteModel],
		backing_vcs: &Repository,
		target: (Entry, Commit),
	) -> Result<Vec<Uuid>, DeckError> {
		let (entry, commit) = target;

		let entry = BEntry::new(&entry)?;
		let author = commit.author().unwrap_or_default(); // Just ignore if non-existent, although reasonably impossible I think haha
		let host_uuid =
			uuid_generator::create_host_uuid(author.name.to_string(), commit.time()?.seconds);

		let file_content = Self::read_file_content(backing_vcs, &entry)?;
		let notes = Self::parse_cards(models, &file_content)?;

		let uuids = notes
			.iter()
			.map(|note| {
				let content = note.to_content_string();
				uuid_generator::generate_note_uuid(&host_uuid, &content)
			})
			.collect();

		debug!("Generated {} UUIDs", notes.len());
		Ok(uuids)
	}
}

// Parse cards from a string reference
fn parse_cards_from_content<'a>(
	models: &'a [NoteModel],
	content: &'a str,
) -> Result<Vec<Note<'a>>, DeckError> {
	Deck::parse_cards(models, content).map_err(|_| DeckError::Parse(String::default()))
}

// Initialize the first state with UUIDs
fn initialize_cards<'a>(
	models: &'a [NoteModel],
	backing_vcs: &Repository,
	entry: &Entry,
	commit: &Commit,
	cards: Vec<Note<'a>>,
) -> Result<Vec<Identified<Note<'a>>>, DeckError> {
	// Generate initial set of UUIDs
	let uuids = Deck::generate_note_uuids(models, backing_vcs, (entry.clone(), commit.clone()))?;

	Ok(cards.into_iter().zip(uuids).map(|(card, id)| card.identified(id)).collect())
}

/// Interpret the passing of a cycle
fn process_cycle(
	last_cards: &[Note],
	current_cards: &[Note],
	static_cards: &mut Vec<Identified<Note>>,
) -> Result<(), DeckError> {
	// It might be that a change was made but nothing of note happened, like a misc.
	// newline, check for this.
	if let Some(changes) = determine_changes(last_cards, current_cards)? {
		// Assuming resolve_uuids mutates static_cards in place or returns new value
		// If it returns a new value:
		resolve_changes(&changes, static_cards, Uuid::default());
	}
	Ok(())
}

fn get_content(backing_vcs: &Repository, entry: &Entry) -> Result<String, DeckError> {
	let file: PathBuf =
		backing_vcs.git_dir().parent().unwrap().join(PathBuf::from(entry.filename().to_string()));

	let content = Deck::read_file_content(backing_vcs, &entry.try_into()?)?;

	// Expand all imports first
	let mut expander = ImportExpander::new(file.parent().unwrap_or_else(|| Path::new(".")));

	Ok(expander.expand(&content, file.as_path()).unwrap())
}

// Main processing logic
fn process_card_history<'a>(
	models: &'a [NoteModel],
	content: &'a [String],
	backing_vcs: &Repository,
	history: &[(Entry, Commit)],
) -> Result<Vec<Identified<Note<'a>>>, DeckError> {
	let mut history_iter = history.iter();

	// Handle first entry separately
	let (first_entry, first_commit) = history_iter.next().ok_or_else(|| DeckError::EmptyHistory)?;

	let first_cards = parse_cards_from_content(models, &content[0])?;

	// Blankly initialize, as we immediately overwrite
	let mut bygone_cards = Vec::with_capacity(first_cards.len());

	let mut elder_cards =
		initialize_cards(models, backing_vcs, first_entry, first_commit, first_cards)?;

	// Process remaining entries
	for (idx, _entry_info) in history_iter.enumerate() {
		let cards_of_the_day = parse_cards_from_content(models, &content[idx + 1])?;

		// Make a diff of the changes and update the final cards appropriately
		process_cycle(&bygone_cards, &cards_of_the_day, &mut elder_cards)?;

		// Cycle complete, the once-new cards lose their youth.
		bygone_cards = cards_of_the_day;
	}

	Ok(elder_cards)
}
