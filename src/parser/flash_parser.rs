use std::{
	borrow::Cow,
	collections::{HashMap, HashSet},
	fs,
	path::{Path, PathBuf},
};

use chumsky::{input::ValueInput, prelude::*};
use evalexpr::{DefaultNumericTypes, HashMapContext, Value, eval_empty_with_context_mut};
use logos::Logos;

use crate::note::{Cloze, Note, NoteField, NoteModel, TextElement};

#[cfg(test)]
use crate::{deck::Deck, note::Field};
#[cfg(test)]
use semver::Version;
#[cfg(test)]
use uuid::Uuid;

/// Preprocessor that expands import statements recursively
pub struct ImportExpander {
	/// Track visited files to prevent circular imports
	visited: HashSet<PathBuf>,
	/// Base directory for resolving relative imports
	base_dir: PathBuf,
}

impl ImportExpander {
	pub fn new(base_dir: impl AsRef<Path>) -> Self {
		Self { visited: HashSet::new(), base_dir: base_dir.as_ref().to_path_buf() }
	}

	/// Expands all imports in the given content recursively
	pub fn expand(&mut self, content: &str, current_file: &Path) -> Result<String, String> {
		// Mark current file as visited
		let canonical = current_file
			.canonicalize()
			.map_err(|e| format!("Cannot resolve path {}: {}", current_file.display(), e))?;

		if !self.visited.insert(canonical.clone()) {
			return Err(format!("Circular import detected: {}", current_file.display()));
		}

		let mut result = String::new();

		for line in content.lines() {
			let trimmed = line.trim();

			// Check for import statement: "import path/to/file.flash"
			if let Some(import_path) = trimmed.strip_prefix("import ") {
				let import_path = import_path.trim();

				// Resolve relative to current file's directory
				let import_file = current_file.parent().unwrap_or(&self.base_dir).join(import_path);

				// Read and recursively expand the imported file
				let imported_content = fs::read_to_string(&import_file)
					.map_err(|e| format!("Cannot read import {}: {}", import_file.display(), e))?;

				let expanded = self.expand(&imported_content, &import_file)?;
				result.push_str(&expanded);

				// Add a blank line to separate imported content
				if !expanded.ends_with("\n\n") {
					result.push('\n');
				}
			} else {
				// Regular line - keep as is
				result.push_str(line);
				result.push('\n');
			}
		}

		// Remove from visited when done
		self.visited.remove(&canonical);

		Ok(result)
	}
}

type Span = SimpleSpan;

use std::fmt;

impl<'a> fmt::Display for Token<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Slash => write!(f, "/"),
			Self::RArrow => write!(f, ">"),
			Self::Colon => write!(f, ":"),
			Self::LBracket => write!(f, "["),
			Self::RBracket => write!(f, "]"),
			Self::LParen => write!(f, "("),
			Self::RParen => write!(f, ")"),
			Self::LBrace => write!(f, "{{"),
			Self::RBrace => write!(f, "}}"),
			Self::Pipe => write!(f, "|"),
			Self::Comma => write!(f, ","),
			Self::Alias => write!(f, "alias"),
			Self::To => write!(f, "to"),
			Self::Newline => write!(f, "\\n"),
			Self::WS(s) => write!(f, "{}", s),
			Self::Text(s) => write!(f, "{}", s),
			Self::Comment(s) => write!(f, "{}", s),
			Self::Error => write!(f, "<parse error>"),
		}
	}
}

#[derive(Logos, Clone, Debug, PartialEq)]
pub enum Token<'a> {
	#[token("/")]
	Slash,

	#[token(":")]
	Colon,

	#[token("[")]
	LBracket,

	#[token("]")]
	RBracket,

	#[token("{")]
	LBrace,

	#[token("}")]
	RBrace,

	#[token(">")]
	RArrow,

	#[token("(")]
	LParen,

	#[token(")")]
	RParen,

	#[token("|")]
	Pipe,

	#[token(",")]
	Comma,

	#[token("alias")]
	Alias,

	#[token("to", priority = 5)]
	To,

	#[token("\n")]
	Newline,

	#[regex(r"[ \t]+")]
	WS(&'a str),

	#[regex(r"[^ \t\n:=\[\](){},|/]+", priority = 4)]
	Text(&'a str),

	#[regex(r"//[^\n]*", allow_greedy = true, priority = 3)]
	Comment(&'a str),

	Error,
}

// Basic Token extractors

fn noise<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, (), extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	select! {
			Token::Newline => (),
			Token::Comment(_) => (),
			Token::WS(_) => (),
	}
	.labelled("newline, comment, or whitespace")
	.ignored()
}

/// Extract whitespace (including = as special whitespace)
/// Extract structural whitespace
fn ws<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, (), extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	select! { Token::WS(_) => () }.labelled("whitespace")
}

fn text<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, &'src str, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	select! {
		Token::Text(s) => s,
	}
}

/// Parse model declaration: = Model Name =
fn model_declaration<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, String, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	let model_name_parts = select! {
		Token::Text(s) => s,
		Token::WS(s) => s,
	};

	just(Token::Slash)
		.ignore_then(model_name_parts.repeated().collect::<Vec<_>>())
		.then_ignore(just(Token::Slash))
		.map(|parts: Vec<&str>| parts.concat().trim().to_string())
		.labelled("model declaration")
}

/// Parse alias: alias <from> to <to>
fn alias_declaration<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, (String, String), extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	just(Token::Alias)
		.ignore_then(ws().repeated().at_least(1))
		.ignore_then(text().map(|s| s.to_string()))
		.then_ignore(ws().repeated())
		.then_ignore(just(Token::To))
		.then_ignore(ws().repeated())
		.then(text().map(|s| s.to_string()))
		.labelled("alias declaration")
}

/// Parse tags: [tag1, tag2, tag3]
fn tags_declaration<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Vec<String>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	let tag_chars = select! {
		Token::Text(s) => s,
		Token::WS(s) => s,
		Token::Alias => "alias",
		Token::To => "to",
	};

	let single_tag = tag_chars
		.repeated()
		.at_least(1)
		.collect::<Vec<&str>>()
		.map(|parts| parts.concat().trim().to_string());

	single_tag
		.separated_by(just(Token::Comma))
		.allow_trailing()
		.collect()
		.delimited_by(just(Token::LBracket), just(Token::RBracket))
		.then_ignore(noise())
		.labelled("tags")
}

/// Parse cloze: {Answer|Hint>Id}
/// Parse cloze: {Answer|Hint} or just { if incomplete
fn cloze<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, TextElement, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	let cloze_chars = select! {
		Token::Text(s) => s,
		Token::WS(s) => s,
		Token::Alias => "alias",
		Token::To => "to",
		Token::Comma => ",",
		Token::Colon => ":",
	};

	let cloze_part = cloze_chars.repeated().at_least(1).collect::<Vec<&str>>().map(|v| v.concat());
	let hint = just(Token::Pipe).ignore_then(cloze_part).map(|s| s.trim().to_string()).or_not();

	// TODO: Fail on a non-number input
	let id = just(Token::RArrow).ignore_then(cloze_chars).or_not();

	// Try complete cloze pattern with closing brace
	let complete_cloze = just(Token::LBrace)
		.ignore_then(cloze_part.map(|s| s.trim().to_string()))
		.then(hint)
		.then(id)
		.then_ignore(just(Token::RBrace))
		.map(|((answer, hint), id)| {
			TextElement::Cloze(Cloze { id: id.and_then(|s| s.parse().ok()).unwrap_or(0), answer, hint })
		});

	// If complete pattern fails, just treat { as literal text
	complete_cloze.or(just(Token::LBrace).to(TextElement::Text("{".to_string()))).labelled("cloze")
}

/// Parse field content (text and clozes)
fn field_content<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Vec<TextElement>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	let text_token = any()
		.filter(|t: &Token| !matches!(t, Token::Newline))
		.map(|t: Token| TextElement::Text(t.to_string()));

	let content_element = cloze().or(text_token);

	content_element.repeated().collect().labelled("field content")
}

fn field_declaration<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, NoteField, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	just(Token::LParen)
		.ignore_then(text())
		.map(|s| s.to_string())
		.then_ignore(just(Token::RParen))
		.then_ignore(ws().repeated()) // Whitespace after field name
		.then(field_content()).then_ignore(just(Token::Newline)) // ignore trailing
		.map(|(name, content)| NoteField { name, content })
		.labelled("field declaration")
}

// Note Builder

/// Build a note from parsed components
struct NoteComponents<'m> {
	model: &'m NoteModel,
	aliases: HashMap<String, String>,
	tags: Vec<String>,
	fields: Vec<NoteField>,
}

impl<'m> NoteComponents<'m> {
	fn into_note(mut self) -> Note<'m> {
		// Resolve aliases in fields
		for field in &mut self.fields {
			// Get the corresponding alias
			if let Some(target) = self.aliases.get(&field.name) {
				field.name = target.clone();
			}
		}

		Note { fields: self.fields, model: Cow::Borrowed(self.model), tags: self.tags }
	}
}

/// Parse a single note's content (tags and fields only)
fn note<'tokens, 'src: 'tokens, I>() -> impl Parser<
	'tokens,
	I,
	(Option<Vec<String>>, Vec<NoteField>),
	extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	tags_declaration().or_not().then(field_declaration().repeated().at_least(1).collect::<Vec<_>>())
}

type AliasPairs = Vec<(String, String)>;

/// Parse an intro of metadata for a set of notes
fn intro<'tokens, 'src: 'tokens, I>(
	available_models: &'tokens [NoteModel],
) -> impl Parser<
	'tokens,
	I,
	(Option<&'tokens NoteModel>, AliasPairs),
	extra::Err<Rich<'tokens, Token<'src>, Span>>,
> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	model_declaration()
		.validate(move |model_name, extra, emitter| {
			let span = extra.span();
			available_models.iter().find(|m| m.name == model_name).map_or_else(
				|| {
					let available =
						available_models.iter().map(|m| m.name.as_str()).collect::<Vec<_>>().join(", ");
					emitter.emit(Rich::custom(
						span,
						format!("Unknown model '{}'. Available: [{}]", model_name, available),
					));
					None
				},
				Some,
			)
		})
		// Parse aliases with explicit noise handling
		.then(
            noise().repeated()
            .ignore_then(alias_declaration())
            .repeated()
            .collect::<Vec<_>>()
        )
		.then_ignore(noise().repeated())
}

type RawNote = (Option<Vec<String>>, Vec<NoteField>);

pub fn flash<'tokens, 'src: 'tokens, I>(
	available_models: &'tokens [NoteModel],
) -> impl Parser<'tokens, I, Vec<Note<'tokens>>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	// Parse a model declaration followed by aliases, then one or more notes
	let model_section = intro(available_models)
		// Then parse multiple notes
		.then(
	        note()
                .separated_by(noise().repeated().at_least(1))
	            .at_least(1)
                .collect::<Vec<RawNote>>()
        )
        .validate(move |((model_opt, aliases), notes_data): ((Option<&NoteModel>, AliasPairs), Vec<RawNote>), extra, emitter| {
	let model = model_opt?;
	let span = extra.span();


	// Build alias map once for all notes
	let alias_map: HashMap<_, _> =
		aliases.into_iter().map(|(from, to)| (to, from)).collect();

	let mut context = HashMapContext::<DefaultNumericTypes>::new();

// Link each field to its shortest unique prefix
let mut unique_prefix: HashMap<String, String> = HashMap::new();
let mut fields = model.fields.clone();
fields.sort_by(|a, b| a.name.cmp(&b.name));

// Initialize all model fields to false
for model_field in &fields {
    eval_empty_with_context_mut(&format!("{} = false", model_field.name), &mut context).unwrap();
}

// After sorting, only adjacent neighbors can share a prefix, so compare each
// field against its immediate neighbors to find the shortest disambiguating prefix
for (i, model_field) in fields.iter().enumerate() {
    let prev_name = fields.get(i.wrapping_sub(1)).map(|f| f.name.as_str());
    let next_name = fields.get(i + 1).map(|f| f.name.as_str());

    // How many chars do we share with this neighbor?
    let shared_with = |neighbor: &str| {
        model_field.name
            .chars()
            .zip(neighbor.chars())
            .take_while(|(a, b)| a == b)
            .count()
    };

    // We need one char beyond the longest shared run to be unambiguous
    let prefix_len = [prev_name, next_name]
        .into_iter()
        .flatten()
        .map(shared_with)
        .max()
        .map(|shared| shared + 1)
        .unwrap_or(1)
        .min(model_field.name.len());

    let prefix = model_field.name[..prefix_len].to_string();
        unique_prefix.insert(prefix, model_field.name.clone());
}

	let notes: Vec<Note> = notes_data
		.into_iter()
		.filter_map(|(tags, fields)| {
			// Reset context
			let mut context = context.clone();



			// Validate fields against model (with alias resolution)
			for field in &fields {
				let resolved_name = alias_map.get(&field.name).or(unique_prefix.get(&field.name)).unwrap_or(&field.name);
				// Setting the fields provided to true within the evaluation context
				eval_empty_with_context_mut(&format!("{} = true", resolved_name), &mut context).unwrap();

				// Also evaluates correctly if the field is an unimbigious match prefix.
				if !model.fields.iter().any(|f| &f.name == resolved_name) {
					emitter.emit(Rich::custom(
						span,
						format!("Field '{}' not found in model '{}'", field.name, model.name),
					));
					return None;
				}
			}
			// Check against the field constraints
			let has_met_field_constraints = model.required.eval_with_context(&context);
			if has_met_field_constraints.is_err() || has_met_field_constraints == Ok(Value::from(false)) {
				emitter.emit(Rich::custom(
						span,
						format!("The provided fields don't meet model {}'s requirements", model.name),
				));
			}
			Some(
				NoteComponents {
					model,
					aliases: alias_map.clone(), // Clone the shared alias map
					tags: tags.unwrap_or_default(),
					fields,
				}
				.into_note(),
			)
		})
		.collect();
	Some(notes)
})
		.try_map(|opt, span| opt.ok_or_else(|| Rich::custom(span, "Invalid note structure")))
		.recover_with(skip_then_retry_until(any().ignored(), noise().ignored()));

	model_section
		.padded_by(noise().repeated())
		.repeated()
		.collect::<Vec<Vec<Note>>>()
		.map(|v| v.into_iter().flatten().collect())
		.then_ignore(end())
}

#[cfg(test)]
fn mock_model() -> NoteModel {
	NoteModel {
		name: "Basic".to_string(),
		id: Uuid::new_v4(),
		templates: vec![],
		schema_version: Version::new(1, 0, 0),
		defaults: None,
		css: "".to_string(),
		fields: vec![
			Field { name: "Front".into(), sticky: None, associated_media: None },
			Field { name: "Back".into(), sticky: None, associated_media: None },
		],
		latex_pre: None,
		latex_post: None,
		sort_field: None,
		tags: None,
		// Requirement: Front must be present
		required: evalexpr::build_operator_tree("Front").unwrap(),
	}
}

#[test]
#[cfg(test)]
fn test_alias_and_multiple_notes() {
	let models = vec![mock_model()];
	let input = r#"
/ Basic /
alias Front to f
alias Back to b

(Front) What is Rust?
(Back) A systems programming language.

(f) Is it fast?
(b) Yes.
"#;

	let notes = Deck::parse_cards(&models, input).expect("Should parse successfully");
	let first_field_content = notes[0].fields[0]
		.content
		.iter()
		.map(|element| match element {
			TextElement::Text(text) => text.as_str(),
			TextElement::Cloze(_) => "",
		})
		.collect::<String>();

	assert_eq!(notes.len(), 2);
	assert_eq!(notes[0].fields[0].name, "Front");
	assert_eq!(first_field_content, "What is Rust?");
	assert_eq!(notes[1].fields[0].name, "Front");
}
