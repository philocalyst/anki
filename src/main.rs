use std::{error::Error, fmt, fs, path::{Path, PathBuf}};

use chumsky::Parser;
use gix::{Commit, Repository, Tree, bstr::{ByteSlice, ByteVec}, object::tree::Entry};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{parse::parser, types::note::{Note, NoteModel, TextElement}};

mod parse;
mod types;

#[derive(Debug)]
pub enum DeckError {
	NoDeckFound,
	ModelNotFound(String),
	FileNotInHistory(String),
	InvalidEntry,
}

impl fmt::Display for DeckError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NoDeckFound => write!(f, "No .deck directory found"),
			Self::ModelNotFound(name) => write!(f, "Model '{}' not found", name),
			Self::FileNotInHistory(path) => write!(f, "File '{}' not found in history", path),
			Self::InvalidEntry => write!(f, "Invalid tree entry"),
		}
	}
}

impl Error for DeckError {}

struct Deck {
	models:      Vec<NoteModel>,
	backing_vcs: Repository,
}

impl Deck {
	#[instrument(skip(backing_vcs))]
	fn new(models: Vec<NoteModel>, backing_vcs: Repository) -> Self {
		info!("Creating deck with {} models", models.len());
		Self { models, backing_vcs }
	}

	#[instrument(skip(self))]
	fn find_model(&self, name: &str) -> Result<&NoteModel, DeckError> {
		debug!("Looking for model: {}", name);
		self.models.iter().find(|model| model.name == name).ok_or_else(|| {
			warn!("Model '{}' not found", name);
			DeckError::ModelNotFound(name.to_string())
		})
	}

	#[instrument(skip(self))]
	fn parse_cards<'a>(&'a self, content: &'a str) -> Result<Vec<Note<'a>>, Box<dyn Error>> {
		debug!("Parsing card content");
		let parser = parser(&self.models);
		Ok(parser.parse(content).unwrap())
	}

	#[instrument(skip(self))]
	fn find_initial_file_creation(&self, target: &str) -> Result<(Entry, Commit), Box<dyn Error>> {
		info!("Finding initial creation of file: {}", target);

		let mut head = self.backing_vcs.head()?;
		let revwalk = self.backing_vcs.rev_walk([head.peel_to_object()?.id()]);

		for commit_id in revwalk.all()? {
			let commit_id = commit_id?;
			let commit = self.backing_vcs.find_commit(commit_id.id())?;
			let tree = commit.tree()?;

			let parent_ids: Vec<_> = commit.parent_ids().collect();

			// Initial commit
			if parent_ids.is_empty() {
				if let Some(entry) = tree.lookup_entry_by_path(target)?.filter(|e| e.mode().is_blob()) {
					info!("File created in initial commit {}", commit.id());
					return Ok((entry, commit));
				}
				continue;
			}

			// Check each parent
			for parent_id in parent_ids {
				let parent_commit = self.backing_vcs.find_commit(parent_id)?;
				let parent_tree = parent_commit.tree()?;

				let in_parent = parent_tree.lookup_entry_by_path(target)?.is_some();
				let in_current = tree.lookup_entry_by_path(target)?.is_some();

				if in_current && !in_parent {
					info!("File first created in commit {}", commit.id());
					if let Some(entry) = tree.lookup_entry_by_path(target)? {
						return Ok((entry, commit));
					}
				}

				if in_current && in_parent {
					self.track_file_changes(&parent_tree, &tree, target)?;
				}
			}
		}

		error!("File not found in repository history");
		Err(DeckError::FileNotInHistory(target.to_string()).into())
	}

	#[instrument(skip(self, parent_tree, current_tree))]
	fn track_file_changes(
		&self,
		parent_tree: &Tree,
		current_tree: &Tree,
		path: &str,
	) -> Result<(), Box<dyn Error>> {
		let parent_entry = parent_tree.lookup_entry_by_path(path)?.ok_or(DeckError::InvalidEntry)?;
		let current_entry = current_tree.lookup_entry_by_path(path)?.ok_or(DeckError::InvalidEntry)?;

		if parent_entry.id() != current_entry.id() {
			debug!("File modified: {}", path);
		}

		Ok(())
	}

	#[instrument(skip(self))]
	fn read_file_content(&self, entry: &Entry) -> Result<String, Box<dyn Error>> {
		if !entry.mode().is_blob() {
			return Err(DeckError::InvalidEntry.into());
		}

		let blob = self.backing_vcs.find_blob(entry.id())?;
		let content = blob.data.clone().into_string()?;
		Ok(content)
	}

	#[instrument(skip(self))]
	fn generate_note_uuids(&self, target_file: &str) -> Result<Vec<Uuid>, Box<dyn Error>> {
		info!("Generating UUIDs for notes in {}", target_file);

		// Generating against the initial point of creation for the file, taking into
		// account renames. This should keep things stable as long as the git repo is
		// the token of trade
		let (entry, commit) = self.find_initial_file_creation(target_file)?;
		let host_uuid =
			UuidGenerator::create_host_uuid(commit.author()?.name.to_string(), commit.time()?.seconds);

		let file_content = self.read_file_content(&entry)?;
		let notes = self.parse_cards(&file_content)?;

		let uuids = notes
			.iter()
			.map(|note| {
				let content = note_to_content_string(note);
				UuidGenerator::generate_note_uuid(&host_uuid, &content)
			})
			.collect();

		debug!("Generated {} UUIDs", notes.len());
		Ok(uuids)
	}
}

// Note Utilities

/// Generate a deterministic string representation of the note's content
/// for UUID generation
#[instrument(skip(note))]
fn note_to_content_string(note: &Note) -> String {
	let mut content = String::new();

	for field in &note.fields {
		content.push_str(&field.name);

		let field_content = field
			.content
			.iter()
			.map(|part| match part {
				TextElement::Text(text) => text.as_str(),
				TextElement::Cloze(cloze) => cloze.answer.as_str(),
			})
			.collect::<Vec<&str>>()
			.join("\0");

		content.push_str(&field_content);
	}

	content
}

#[instrument(skip(note))]
fn print_note_debug(note: &Note) {
	for field in &note.fields {
		info!("{} : {:?}", field.name, field.content);
	}
	if !note.tags.is_empty() {
		info!("Tags: {:?}", note.tags);
	}
}

struct UuidGenerator;

impl UuidGenerator {
	/// Creates the main UUID based on the author of the initial commit and the
	/// time
	#[instrument]
	fn create_host_uuid(author: String, time: i64) -> Uuid {
		debug!("Creating host UUID for author: {}, time: {}", author, time);

		// Note: This is fragile and will break under rebase conditions
		// This is inherent to the design for deterministic generation
		let namespace = format!("{}{}", author, time);
		Uuid::new_v5(&Uuid::NAMESPACE_DNS, namespace.as_bytes())
	}

	/// Generate a UUID for a specific note based on its content
	#[instrument(skip(content))]
	fn generate_note_uuid(host_uuid: &Uuid, content: &str) -> Uuid {
		Uuid::new_v5(host_uuid, content.as_bytes())
	}
}

struct DeckLocator;

impl DeckLocator {
	#[instrument]
	fn find_deck_directory() -> Result<PathBuf, Box<dyn Error>> {
		info!("Searching for deck directory");

		let dirs: Vec<PathBuf> = fs::read_dir(".")?
			.filter_map(Result::ok)
			.filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
			.map(|entry| entry.path())
			.collect();

		dirs.into_iter().find(|dir| Self::is_deck_dir(dir)).ok_or_else(|| {
			error!("No deck directory found");
			DeckError::NoDeckFound.into()
		})
	}

	fn is_deck_dir(path: &Path) -> bool {
		path.is_dir() && path.extension().and_then(|e| e.to_str()) == Some("deck")
	}

	#[instrument]
	fn scan_deck_contents(deck_path: &Path) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn Error>> {
		info!("Scanning deck contents at {:?}", deck_path);

		let mut models = Vec::new();
		let mut cards = Vec::new();

		for entry in fs::read_dir(deck_path)? {
			let entry = entry?;
			let path = entry.path();

			if entry.file_type()?.is_dir()
				&& entry.path().extension().and_then(|ext| ext.to_str()) == Some("model")
			{
				debug!("Found model directory: {:?}", path);
				models.push(path);
			} else if path.extension().and_then(|ext| ext.to_str()) == Some("flash") {
				debug!("Found card file: {:?}", path);
				cards.push(path);
			}
		}

		info!("Found {} models and {} card files", models.len(), cards.len());
		Ok((models, cards))
	}
}

struct ModelLoader;

impl ModelLoader {
	#[instrument]
	fn load_models(
		model_paths: &[PathBuf],
		deck_path: &Path,
	) -> Result<Vec<NoteModel>, Box<dyn Error>> {
		info!("Loading {} models", model_paths.len());

		let mut all_models = Vec::new();

		for model_path in model_paths {
			let config_path = model_path.join("config.toml");
			debug!("Loading model config from {:?}", config_path);

			let config_content = fs::read_to_string(&config_path)?;
			let mut model: NoteModel = toml::from_str(&config_content)?;

			// TODO: This path should be more dynamic
			model.complete(deck_path)?;

			info!("Loaded model: {}", model.name);
			all_models.push(model);
		}

		Ok(all_models)
	}
}

// Main Entry Point

#[instrument]
fn main() -> Result<(), Box<dyn Error>> {
	// Initialize tracing
	tracing_subscriber::fmt().with_target(false).with_level(true).init();

	info!("Starting Anki deck parser");

	// Find and scan deck
	let deck_path = DeckLocator::find_deck_directory()?;
	info!("Found deck at: {:?}", deck_path);

	let (model_paths, card_paths) = DeckLocator::scan_deck_contents(&deck_path)?;

	if card_paths.is_empty() {
		warn!("No card files found");
		return Ok(());
	}

	// Load models
	let models = ModelLoader::load_models(&model_paths, &deck_path)?;

	// Open repository
	let repo_path = deck_path.join(".git");
	info!("Opening repository at: {:?}", repo_path);
	let backing_vcs = gix::open(repo_path)?;

	// Create deck
	let deck = Deck::new(models, backing_vcs);

	// Parse first card file as example
	let first_card_path = &card_paths[0];
	info!("Parsing card file: {:?}", first_card_path);
	let card_content = fs::read_to_string(first_card_path)?;

	let parse_result = deck.parse_cards(&card_content);

	match parse_result {
		Ok(cards) => {
			info!("Successfully parsed {} cards", cards.len());
			for card in &cards {
				print_note_debug(card);
			}

			// Generate UUIDs
			let uuids = deck.generate_note_uuids("index.flash")?; // TODO: Make this run per the card file
			info!("Generated UUIDs:");

			for uuid in uuids {
				info!("{}", uuid);
			}
		}
		Err(error) => {
			error!("Parsing error: {}", error);
		}
	}

	info!("Deck parsing completed");
	Ok(())
}
