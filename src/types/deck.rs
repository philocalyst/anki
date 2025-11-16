use std::error::Error;

use chumsky::Parser;
use gix::{Commit, Repository, Tree, bstr::{ByteSlice, ByteVec}, object::tree::Entry};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{error::DeckError, parse::flash, types::note::{Note, NoteModel}, uuid_generator::UuidGenerator};

pub struct Deck {
	models:      Vec<NoteModel>,
	backing_vcs: Repository,
}

impl Deck {
	#[instrument(skip(backing_vcs))]
	pub fn new(models: Vec<NoteModel>, backing_vcs: Repository) -> Self {
		info!("Creating deck with {} models", models.len());
		Self { models, backing_vcs }
	}

	#[instrument(skip(self))]
	pub fn find_model(&self, name: &str) -> Result<&NoteModel, DeckError> {
		debug!("Looking for model: {}", name);
		self.models.iter().find(|model| model.name == name).ok_or_else(|| {
			warn!("Model '{}' not found", name);
			DeckError::ModelNotFound(name.to_string())
		})
	}

	#[instrument(skip(self))]
	pub fn parse_cards<'a>(&'a self, content: &'a str) -> Result<Vec<Note<'a>>, Box<dyn Error>> {
		debug!("Parsing card content");
		let parser = flash(&self.models);
		Ok(parser.parse(content).unwrap())
	}

	#[instrument(skip(self))]
	pub fn get_file_history(
		&self,
		target: &str,
	) -> Result<Vec<(gix::object::tree::Entry<'_>, gix::Commit<'_>)>, Box<dyn Error>> {
		info!("Finding history of file: {}", target);

		let mut history = Vec::new();
		let mut head = self.backing_vcs.head()?;
		let revwalk = self.backing_vcs.rev_walk([head.peel_to_object()?.id()]);

		for commit_id in revwalk.all()? {
			let commit_id = commit_id?;
			let commit = self.backing_vcs.find_commit(commit_id.id())?;
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
				let parent_commit = self.backing_vcs.find_commit(parent_id)?;
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
			Err(DeckError::FileNotInHistory(target.to_string()).into())
		} else {
			info!("Found {} commits in file history", history.len());
			Ok(history)
		}
	}

	#[instrument(skip(self, parent_tree, current_tree))]
	pub fn track_file_changes(
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
	pub fn read_file_content(&self, entry: &Entry) -> Result<String, Box<dyn Error>> {
		if !entry.mode().is_blob() {
			return Err(DeckError::InvalidEntry.into());
		}

		let blob = self.backing_vcs.find_blob(entry.id())?;
		let content = blob.data.clone().into_string()?;
		Ok(content)
	}

	#[instrument(skip(self))]
	pub fn generate_note_uuids(&self, target: (Entry, Commit)) -> Result<Vec<Uuid>, Box<dyn Error>> {
		let (entry, commit) = target;
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
