use std::error::Error;

use chumsky::Parser;
use gix::{Commit, Repository, Tree, bstr::{ByteSlice, ByteVec}, object::tree::Entry};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{error::DeckError, parse::flash, types::note::{Note, NoteModel}, uuid_generator::UuidGenerator};

pub(crate) struct Deck {
	models:      Vec<NoteModel>,
	backing_vcs: Repository,
}

impl Deck {
	#[instrument(skip(backing_vcs))]
	pub(crate) fn new(models: Vec<NoteModel>, backing_vcs: Repository) -> Self {
		info!("Creating deck with {} models", models.len());
		Self { models, backing_vcs }
	}

	#[instrument(skip(self))]
	pub(crate) fn find_model(&self, name: &str) -> Result<&NoteModel, DeckError> {
		debug!("Looking for model: {}", name);
		self.models.iter().find(|model| model.name == name).ok_or_else(|| {
			warn!("Model '{}' not found", name);
			DeckError::ModelNotFound(name.to_string())
		})
	}

	#[instrument(skip(self))]
	pub(crate) fn parse_cards<'a>(
		&'a self,
		content: &'a str,
	) -> Result<Vec<Note<'a>>, Box<dyn Error>> {
		debug!("Parsing card content");
		let parser = flash(&self.models);
		Ok(parser.parse(content).unwrap())
	}

	#[instrument(skip(self))]
	pub(crate) fn find_initial_file_creation(
		&self,
		target: &str,
	) -> Result<(Entry<'_>, Commit<'_>), Box<dyn Error>> {
		info!("Finding initial creation of file: {}", target);

		let mut head = self.backing_vcs.head()?;
		let revwalk = self.backing_vcs.rev_walk([head.peel_to_object()?.id()]);

		for commit_id in revwalk.all()? {
			let commit_id = commit_id?;
			let commit = self.backing_vcs.find_commit(commit_id.id())?;
			let tree = commit.tree()?;

			let parent_ids: Vec<_> = commit.parent_ids().collect();

			// If there are no parent ids, and we find the target, then this is the
			// commit.
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
	pub(crate) fn track_file_changes(
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
	pub(crate) fn read_file_content(&self, entry: &Entry) -> Result<String, Box<dyn Error>> {
		if !entry.mode().is_blob() {
			return Err(DeckError::InvalidEntry.into());
		}

		let blob = self.backing_vcs.find_blob(entry.id())?;
		let content = blob.data.clone().into_string()?;
		Ok(content)
	}

	#[instrument(skip(self))]
	pub(crate) fn generate_note_uuids(&self, target_file: &str) -> Result<Vec<Uuid>, Box<dyn Error>> {
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
				let content = note.to_content_string();
				UuidGenerator::generate_note_uuid(&host_uuid, &content)
			})
			.collect();

		debug!("Generated {} UUIDs", notes.len());
		Ok(uuids)
	}
}
