use std::{collections::HashMap, error::Error, fs, io, ops::Range, path::{Path, PathBuf}};

use ariadne::Source;
use chumsky::prelude::*;
use evalexpr::Node;
use semver::Version;
use serde::Deserialize;

use crate::types::crowd_anki_models::{CrowdAnkiEntity, NoteModelType};

mod types;

struct ParserNoteModel<'a> {
	pub model:   Option<&'a NoteModel>,
	pub span:    Option<SimpleSpan>,
	pub aliases: HashMap<String, &'a NoteField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note<'a> {
	pub fields: Vec<NoteField>,
	pub model:  &'a NoteModel,
	pub tags:   Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NoteField {
	name:    String,
	content: Vec<TextElement>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct NoteModel {
	pub name: String,

	// The available templates
	pub templates: Vec<Template>,

	// The version of the schema that we're on
	pub schema_version: Version,

	// The default field configuration
	pub defaults: Option<Defaults>,

	// Anything with serde skip means I don't want it to be possible to be included in the TOML
	// representation
	#[serde(skip)]
	pub css: String,

	pub fields: Vec<Field>,

	#[serde(skip)]
	pub latex_pre:  Option<String>,
	#[serde(skip)]
	pub latex_post: Option<String>,

	// The field to sort around
	pub sort_field: Option<String>,
	pub tags:       Option<Vec<String>>,

	// The required fields are determined at runtime, this String holds a boolean expression that
	// affirms this.
	pub required: Node,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cloze {
	pub id:     u32,
	pub answer: String,
	pub hint:   Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextElement {
	Text(String),
	Cloze(Cloze),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlashItem {
	NoteModel(String),
	Alias { from: String, to: String },
	Tags(Vec<String>),
	Field { name: String, content: Vec<TextElement> },
	Comment(String),
	BlankLine,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Defaults {
	pub font: String,
	pub size: u32,
	pub rtl:  bool,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Field {
	pub name:             String,
	pub sticky:           Option<bool>,
	pub associated_media: Option<Vec<PathBuf>>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Template {
	pub name: String,

	#[serde(skip)]
	pub order: i32,

	#[serde(skip)]
	pub question_format: String,
	#[serde(skip)]
	pub answer_format:   String,

	#[serde(skip)]
	pub browser_question_format: String,
	#[serde(skip)]
	pub browser_answer_format:   String,
}

impl NoteModel {
	pub fn complete(&mut self, dir: &Path) -> Result<(), Box<dyn Error>> {
		// Load CSS if present
		let css_path = dir.join("style.css");
		if css_path.exists() {
			self.css = fs::read_to_string(css_path)?;
		}

		// Load LaTeX pre/post if present
		let pre_path = dir.join("pre.tex");
		if pre_path.exists() {
			self.latex_pre = Some(fs::read_to_string(pre_path)?);
		}

		let post_path = dir.join("post.tex");
		if post_path.exists() {
			self.latex_post = Some(fs::read_to_string(post_path)?);
		}

		// Load templates from .hbs files
		let mut templates = Vec::new();
		for entry in fs::read_dir(dir)? {
			let entry = entry?;
			let path = entry.path();

			if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
				if ext == "hbs" {
					let filename = path.file_stem().unwrap().to_string_lossy().to_string();

					// Parse naming convention: NAME+front.hbs, NAME+back.browser.hbs, etc.
					let parts: Vec<&str> = filename.split('+').collect();
					if parts.len() != 2 {
						continue; // skip malformed
					}

					let template_name = parts[0].to_string();
					let side = parts[1];

					// Find or create template
					let tmpl = templates.iter_mut().find(|t: &&mut Template| t.name == template_name);

					let tmpl = if let Some(t) = tmpl {
						t
					} else {
						templates.push(Template {
							name:                    template_name.clone(),
							order:                   templates.len() as i32,
							question_format:         String::new(),
							answer_format:           String::new(),
							browser_question_format: String::new(),
							browser_answer_format:   String::new(),
						});
						templates.last_mut().unwrap()
					};

					let content = fs::read_to_string(&path)?;

					// Assign based on side
					if side.starts_with("front") {
						if side.contains("browser") {
							tmpl.browser_question_format = content;
						} else {
							tmpl.question_format = content;
						}
					} else if side.starts_with("back") {
						if side.contains("browser") {
							tmpl.browser_answer_format = content;
						} else {
							tmpl.answer_format = content;
						}
					}
				}
			}
		}

		self.templates = templates;
		Ok(())
	}
}

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

pub fn parser<'a>(
	available_models: &'a [NoteModel],
) -> impl Parser<'a, &'a str, Vec<Note<'a>>, extra::Err<Rich<'a, char>>> + Clone {
	// Inline whitespace (spaces and tabs only; excludes newlines)
	let ws = one_of(" \t").repeated().ignored();

	// "= Model Name =" line
	let note_model = just('=')
		.ignore_then(none_of('=').repeated().collect::<String>())
		.then_ignore(just('='))
		.map(|name| FlashItem::NoteModel(name.trim().to_string()));

	// "alias <from> to <to>"
	let alias = text::keyword("alias")
		.padded_by(ws.clone())
		.ignore_then(none_of([' ', '\t', '\n']).repeated().at_least(1).collect::<String>())
		.then_ignore(ws.clone())
		.then_ignore(text::keyword("to"))
		.then_ignore(ws.clone())
		.then(none_of([' ', '\t', '\n']).repeated().at_least(1).collect::<String>())
		.map(|(from, to)| FlashItem::Alias { from, to });

	let cloze_content = none_of(['|', '}', '\n'])
		.repeated()
		.at_least(1)
		.collect::<String>()
		.map(|s| s.trim().to_string());

	let cloze_hint = just('|').ignore_then(cloze_content.clone()).or_not();

	let cloze = just('{')
		.ignore_then(cloze_content)
		.then(cloze_hint)
		.then_ignore(just('}'))
		.map(|(answer, hint)| TextElement::Cloze(Cloze { id: 0, answer, hint }));

	// [tag1, tag2, ...]
	let tag = none_of(",[]").repeated().at_least(1).collect::<String>().map(|s| s.trim().to_string());

	let tags = tag
		.separated_by(just(',').padded())
		.collect::<Vec<_>>()
		.delimited_by(just('['), just(']'))
		.map(FlashItem::Tags);

	// Field name (identifier) before the colon, then its content (text + clozes)
	let field_name = text::ident().map(|s: &str| s.to_string()).then_ignore(just(':'));

	let regular_text =
		none_of(['{', '#', '\n']).repeated().at_least(1).collect::<String>().map(TextElement::Text);

	let content_element = cloze.or(regular_text);

	// Merge adjacent Text(...) for cleaner output
	let content = content_element.repeated().collect::<Vec<TextElement>>().validate(
		|elements, _span, _emitter| {
			let mut merged = Vec::new();
			let mut buf = String::new();

			for el in elements {
				match el {
					TextElement::Text(t) => buf.push_str(&t),
					cloze @ TextElement::Cloze(_) => {
						if !buf.is_empty() {
							merged.push(TextElement::Text(std::mem::take(&mut buf)));
						}

						merged.push(cloze);
					}
				}
			}

			if !buf.is_empty() {
				merged.push(TextElement::Text(buf));
			}

			merged
		},
	);

	let field_pair = field_name
		.then_ignore(ws.clone())
		.then(content)
		.map(|(name, content)| FlashItem::Field { name, content });

	let comment =
		just("//").ignore_then(none_of('\n').repeated().collect::<String>()).map(FlashItem::Comment);

	let blank_line = text::newline().to(FlashItem::BlankLine);

	// A single line in the input (we keep the span to enable good error messages)
	let line = choice((note_model, alias, tags, field_pair, comment, blank_line))
		.map_with(|item, e| (item, e.span()));

	// For building notes, we need a little bit of state.
	#[derive(Default)]
	struct BuildState<'m> {
		current_model: Option<&'m NoteModel>,
		aliases:       HashMap<String, String>,
		tags:          Vec<String>,
		fields:        Vec<NoteField>,
		notes:         Vec<Note<'m>>,
	}

	// Helper: find model by name
	let find_model = move |name: &str| available_models.iter().find(|m| m.name == name);

	// Helper: finalize current note (if any), clear working buffers
	let mut finalize_note = |state: &mut BuildState<'a>| {
		if state.fields.is_empty() {
			// Nothing to flush
			return;
		}
		if let Some(model) = state.current_model {
			state.notes.push(Note {
				fields: std::mem::take(&mut state.fields),
				tags: std::mem::take(&mut state.tags),
				model,
			});
		} else {
			// If no model is active, discard accumulated fields/tags
			state.fields.clear();
			state.tags.clear();
		}
	};

	// Build notes and emit user-friendly errors for unknown fields or models.
	line.repeated().collect::<Vec<(FlashItem, SimpleSpan)>>().then_ignore(end()).validate(
		move |items, _span, emitter| {
			let mut state = BuildState::default();

			for (item, span) in items {
				match item {
					FlashItem::NoteModel(name) => {
						// Flush any pending note before switching model
						finalize_note(&mut state);
						state.aliases.clear();

						match find_model(&name) {
							Some(m) => state.current_model = Some(m),
							None => {
								// Keep logic: selecting a model that doesn't exist is an error.
								emitter.emit(Rich::custom(span, format!("Unknown note model '{}'", name)));
								state.current_model = None;
							}
						}
					}

					FlashItem::Alias { from, to } => {
						// Only meaningful if a model is active
						if state.current_model.is_some() {
							state.aliases.insert(from, to);
						}
					}

					FlashItem::Tags(ts) => {
						state.tags = ts;
					}

					FlashItem::Field { name, content } => {
						// Apply alias, if any
						let resolved_name = state.aliases.get(&name).cloned().unwrap_or_else(|| name.clone());

						// Validate against the active model's fields, if there is a model
						if let Some(model) = state.current_model {
							let exists = model.fields.iter().any(|f| f.name == resolved_name);

							if !exists {
								// Keep original logic: complain and skip this field
								let available =
									model.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", ");

								emitter.emit(Rich::custom(
									span,
									format!(
										"Unknown field '{}' for model '{}'. Available: [{}]",
										name, model.name, available
									),
								));
								continue;
							}
						}

						state.fields.push(NoteField { name: resolved_name, content });
					}

					FlashItem::Comment(_) => {
						// Ignore
					}

					FlashItem::BlankLine => {
						finalize_note(&mut state);
					}
				}
			}

			// Final flush at EOF
			finalize_note(&mut state);

			state.notes
		},
	)
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
