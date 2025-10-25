use std::{error::Error, fs, path::{Path, PathBuf}};

use crate::types::note::NoteModel;

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

	for model in models.clone() {
		let config = model.join("config.toml");

		// Load config first
		let example_config = fs::read_to_string(config)?;
		let mut config: NoteModel = toml::from_str(&example_config).unwrap();

		config.complete(Path::new("/Users/philocalyst/Projects/anki/COVID.deck/ClozeWithSource"))?;

		all_models.push(config);
	}

	let example_content = fs::read_to_string(cards[0].clone())?;

	let parse_result = parser(all_models.as_slice()).parse(&example_content);

	match parse_result.into_result() {
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

	Ok(())
}
