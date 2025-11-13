use std::{error::Error, fs::{self, write}, path::{Path, PathBuf}, sync::Arc};

use chumsky::Parser;
use fs_err::read;
use gix::{Commit, Repository, Tree, bstr::{ByteSlice, ByteVec}, diff::index::{Change, ChangeRef}, object::tree::{Entry, EntryRef}, open};
use serde::Serialize;
use uuid::Uuid;

use crate::{parse::parser, types::{crowd_anki_models::CrowdAnkiEntity, note::{Note, NoteModel}}};

struct Deck<'a> {
	cards:       Vec<Note<'a>>,
	models:      Vec<NoteModel>,
	backing_vcs: Repository,
}

mod parse;
mod types;

fn find_model<'a>(
	name: &str,
	available_models: &'a [NoteModel],
) -> Result<&'a NoteModel, Box<dyn Error>> {
	for model in available_models {
		if model.name == name {
			return Ok(model);
		}
	}

	return Err("Model doesn't exist".into());
}

fn is_deck_dir<P: AsRef<Path>>(path: P) -> bool {
	let p = path.as_ref();
	// is_dir() will return false if it doesn't exist or isn't a dir
	p.is_dir() && p.extension().and_then(|e| e.to_str()) == Some("deck")
}

fn main() -> Result<(), Box<dyn Error>> {
	// Check for a deck
	let mut dirs = Vec::new();
	for entry in fs::read_dir(".")? {
		let entry = entry?;
		if entry.file_type()?.is_dir() {
			dirs.push(entry.path());
		}
	}

	let deck: PathBuf = dirs.into_iter().find(|dir| is_deck_dir(dir)).unwrap();

	// Get the models and flashcards in the deck
	let mut models = Vec::new();
	let mut cards = Vec::new();
	for entry in fs::read_dir(deck)? {
		let entry = entry?;
		if entry.file_type()?.is_dir() {
			models.push(entry.path());
		} else if entry.path().extension().and_then(|ext| ext.to_str()) == Some("flash") {
			cards.push(entry.path());
		}
	}

	let mut all_models: Vec<NoteModel> = Vec::new();

	let backing_vcs: Repository;
	backing_vcs = open("/Users/philocalyst/Projects/anki/COVID.deck/.git")?;

	for model in models.clone() {
		let config = model.join("config.toml");

		// Load config first
		let example_config = fs::read_to_string(config)?;
		let mut config: NoteModel = toml::from_str(&example_config).unwrap();

		config.complete(Path::new("/Users/philocalyst/Projects/anki/COVID.deck/ClozeWithSource"))?;

		all_models.push(config);
		break;
	}

	let example_content = fs::read_to_string(cards[0].clone())?;

	let binding = all_models.clone();
	let parser_method = parser(binding.as_slice());
	let parse_result = parser_method.parse(&example_content);

	let deck =
		Deck { cards: parse_result.clone().into_result().unwrap(), models: all_models, backing_vcs };

	match parse_result.clone().into_result() {
		Ok(cards) => {
			println!("  Cards:");
			for card in &cards {
				for field in card.fields.clone() {
					println!("{} : {:?}", field.name, field.content);
				}
				if !card.tags.is_empty() {
					println!("    Tags: {:?}", card.tags);
				}
				println!();
			}
			println!("---");
		}

		Err(errors) => {
			eprintln!("Parsing errors:");
			for error in errors {
				eprintln!("  {}", error);
			}
		}
	}

	let first_relevant = find_initial_file_creation(&deck.backing_vcs)?;

	let seed = first_relevant.1;

	let host_uuid = create_host_uuid(seed.author()?.name.to_string(), seed.time()?.seconds);

	let parser2 = parser(binding.as_slice());

	let file_content = file_content(&deck.backing_vcs, &first_relevant.0)?;

	let parser_method = parser(binding.as_slice());

	let parsed = parser_method.parse(&file_content).unwrap();

	Ok(())
}

// Creates the main UUID based off of the author of the initial commit and the
// time it was made
fn create_host_uuid(author: String, time: i64) -> Uuid {
	let namespace = format!("{}{}", author, time);
	Uuid::new_v5(&Uuid::NAMESPACE_DNS, namespace.as_bytes())
}

fn find_initial_file_creation(repo: &Repository) -> Result<(Entry, Commit), Box<dyn Error>> {
	let mut head = repo.head()?;
	let target = "index.flash";

	let revwalk = repo.rev_walk([head.peel_to_object()?.id()]);

	for commit_id in revwalk.all()? {
		let commit_id = commit_id?;
		let commit = repo.find_commit(commit_id.id())?;
		let tree = commit.tree()?;

		let parent_ids: Vec<_> = commit.parent_ids().collect();

		// Initial commit
		if parent_ids.is_empty() {
			if let Ok(Some(entry)) = tree.lookup_entry_by_path(target) {
				if entry.mode().is_blob() {
					println!("File created in initial commit {}", commit.id());
					return Ok((entry, commit));
				}
			}
			continue;
		}

		// Check each parent
		for parent_id in parent_ids {
			let parent_commit = repo.find_commit(parent_id)?;
			let parent_tree = parent_commit.tree()?;

			let in_parent = matches!(parent_tree.lookup_entry_by_path(target), Ok(Some(_)));
			let in_current = matches!(tree.lookup_entry_by_path(target), Ok(Some(_)));

			if in_current && !in_parent {
				println!("File first created in commit {}", commit.id());
				if let Ok(Some(entry)) = tree.lookup_entry_by_path(target) {
					return Ok((entry, commit));
				}
			}

			if in_current && in_parent {
				track_file_changes(repo, &parent_tree, &tree, target)?;
			}
		}
	}

	println!("File not found in repository history");
	todo!()
}

fn track_file_changes(
	repo: &Repository,
	parent_tree: &Tree,
	current_tree: &Tree,
	path: &str,
) -> Result<(), Box<dyn Error>> {
	let parent_entry = parent_tree.lookup_entry_by_path(path)?.unwrap();
	let current_entry = current_tree.lookup_entry_by_path(path)?.unwrap();

	// Check if content changed
	if parent_entry.id() != current_entry.id() {
		println!("  File modified: {}", path);
		// You can diff here if needed
	}

	Ok(())
}

fn file_content(repo: &Repository, entry: &Entry) -> Result<String, Box<dyn Error>> {
	if !entry.mode().is_blob() {
		todo!()
	}

	let blob = repo.find_blob(entry.id())?;
	let content = blob.data.clone().into_string()?;
	Ok(content)
}
