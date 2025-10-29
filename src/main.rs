use std::{error::Error, fs::{self, write}, path::{Path, PathBuf}, sync::Arc};

use chumsky::Parser;
use fs_err::read;
use gix::{Commit, Repository, bstr::ByteVec, diff::index::{Change, ChangeRef}, open};
use serde::Serialize;

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
	let parse_result = parser(binding.as_slice()).parse(&example_content);

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

	find_initial_file_creation(&deck.backing_vcs)?;

	Ok(())
}

fn find_initial_file_creation(repo: &Repository) -> Result<(), Box<dyn Error>> {
	let mut head = repo.head()?;
	let target = "test.flash";

	let revwalk = repo.rev_walk([head.peel_to_object()?.id()]);

	for commit_id in revwalk.all()? {
		let commit_id = commit_id?;

		// This all relies on the commit tree, which contains the files/data at a
		// particular commit
		let commit = repo.find_commit(commit_id.id())?;
		let tree = commit.tree()?;

		let parent_ids: Vec<_> = commit.parent_ids().collect();

		// Initial commit - check if file exists
		if parent_ids.is_empty() {
			if let Ok(Some(_entry)) = tree.lookup_entry_by_path(target) {
				println!("File created in initial commit {}", commit.id());
				return Ok(());
			}
			continue;
		}

		// Regular commits - check against parents, for which there is usually one
		// (if there are no branches)
		for parent_id in parent_ids {
			// Resolve to the respective commit
			let parent_commit = repo.find_commit(parent_id)?;

			let parent_tree = parent_commit.tree()?;

			// Then look at both the parent and current
			let in_parent = parent_tree.lookup_entry_by_path(target).is_ok();
			let in_current = tree.lookup_entry_by_path(target).is_ok();

			if in_current && !in_parent {
				println!("File first created in commit {}", commit.id());
				return Ok(());
			}
		}
	}

	println!("File not found in repository history");
	Ok(())
}

fn get_commit_contents(
	repo: &Repository,
	commit: Commit,
	path: &str,
) -> Result<String, Box<dyn Error>> {
	// Get the entry of the tree where our file is found
	let tree = commit.tree()?;
	let entry = tree.lookup_entry_by_path(path)?.expect("We know it's here already");

	// Load its data
	let blob = repo.find_blob(entry.id())?;

	Ok(blob.data.clone().into_string()?)
}
