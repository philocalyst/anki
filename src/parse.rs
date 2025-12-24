use std::{collections::{HashMap, HashSet}, fmt, fs, path::{Path, PathBuf}};

use chumsky::{input::{Emitter, ValueInput}, prelude::*};
use logos::Logos;

use crate::types::{note::{Cloze, Note, NoteField, NoteModel, TextElement}, parser::FlashItem};

/// Preprocessor that expands import statements recursively
pub struct ImportExpander {
	/// Track visited files to prevent circular imports
	visited:  HashSet<PathBuf>,
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

	/// Convenience method to expand from a file path
	pub fn expand_file(&mut self, path: impl AsRef<Path>) -> Result<String, String> {
		let path = path.as_ref();
		let content = fs::read_to_string(path)
			.map_err(|e| format!("Cannot read file {}: {}", path.display(), e))?;

		self.expand(&content, path)
	}
}

type Span = SimpleSpan;

// ----------------------------------------------------------------------------
// Lexer
// ----------------------------------------------------------------------------

#[derive(Logos, Clone, Debug, PartialEq)]
pub enum Token<'a> {
	#[token("=")]
	Eq,
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
	#[token("|")]
	Pipe,
	#[token(",")]
	Comma,

	#[token("alias")]
	KwAlias,
	#[token("to")]
	KwTo,

	#[token("\n")]
	Newline,

	// We capture whitespace explicitly to preserve formatting in text fields.
	// Structural whitespace (indentation, etc.) can be ignored by the parser
	// where appropriate.
	#[regex(r"[ \t]+")]
	WS(&'a str),

	// Matches generic text: sequences of chars that aren't delimiters or control chars.
	#[regex(r"[^ \t\n:=\[\]{},|]+")]
	Text(&'a str),

	// Comments start with // and go to end of line (but do not consume the newline)
	// Comments start with // and go to end of line (but do not consume the newline)
	// Using [^\n]* ensures it stops at the newline and doesn't backtrack or scan ahead.
	#[regex(r"//[^\n]*", allow_greedy = true)]
	Comment(&'a str),

	// We manually define an Error variant to handle Logos errors in the stream
	Error,
}

impl fmt::Display for Token<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Token::Eq => write!(f, "="),
			Token::Colon => write!(f, ":"),
			Token::LBracket => write!(f, "["),
			Token::RBracket => write!(f, "]"),
			Token::LBrace => write!(f, "{{"),
			Token::RBrace => write!(f, "}}"),
			Token::Pipe => write!(f, "|"),
			Token::Comma => write!(f, ","),
			Token::KwAlias => write!(f, "alias"),
			Token::KwTo => write!(f, "to"),
			Token::Newline => write!(f, "\\n"),
			Token::WS(s) => write!(f, "WS({:?})", s),
			Token::Text(s) => write!(f, "{}", s),
			Token::Comment(_) => write!(f, "//..."),
			Token::Error => write!(f, "<Error>"),
		}
	}
}

// ----------------------------------------------------------------------------
// Builder State
// ----------------------------------------------------------------------------

/// State machine for building notes from parsed items
#[derive(Default)]
struct NoteBuilder<'m> {
	current_model: Option<&'m NoteModel>,
	aliases:       HashMap<String, String>,
	tags:          Vec<String>,
	fields:        Vec<NoteField>,
	notes:         Vec<Note<'m>>,
}

impl<'m> NoteBuilder<'m> {
	fn has_pending_note(&self) -> bool { !self.fields.is_empty() }

	fn finalize_note(&mut self) {
		if !self.has_pending_note() {
			return;
		}

		if let Some(model) = self.current_model {
			self.notes.push(Note {
				fields: std::mem::take(&mut self.fields),
				tags:   std::mem::take(&mut self.tags),
				model:  std::borrow::Cow::Borrowed(model),
			});
		} else {
			// No active model: discard accumulated fields/tags
			self.clear_current_note();
		}
	}

	fn clear_current_note(&mut self) {
		self.fields.clear();
		self.tags.clear();
	}

	fn switch_model(&mut self, model: Option<&'m NoteModel>) {
		self.finalize_note();
		self.aliases.clear();
		self.current_model = model;
	}

	fn set_tags(&mut self, tags: Vec<String>) { self.tags = tags; }

	fn resolve_field_name(&self, name: &str) -> String {
		self.aliases.get(name).cloned().unwrap_or_else(|| name.to_string())
	}

	fn add_alias(&mut self, from: String, to: String) {
		if self.current_model.is_some() {
			self.aliases.insert(to, from);
		}
	}

	fn validate_and_add_field(
		&mut self,
		name: String,
		content: Vec<TextElement>,
		span: Span,
		emitter: &mut Emitter<Rich<Token>>,
	) {
		let resolved_name = self.resolve_field_name(&name);

		if let Some(model) = self.current_model {
			if !Self::field_exists_in_model(model, &resolved_name) {
				Self::emit_unknown_field_error(model, &name, span, emitter);
				return;
			}
		}

		self.fields.push(NoteField { name: resolved_name, content });
	}

	fn field_exists_in_model(model: &NoteModel, field_name: &str) -> bool {
		model.fields.iter().any(|f| f.name == field_name)
	}

	fn emit_unknown_field_error(
		model: &NoteModel,
		field_name: &str,
		span: Span,
		emitter: &mut Emitter<Rich<Token>>,
	) {
		let available = model.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", ");

		emitter.emit(Rich::custom(
			span,
			format!(
				"Unknown field '{}' for model '{}'. Available: [{}]",
				field_name, model.name, available
			),
		));
	}

	fn into_notes(mut self) -> Vec<Note<'m>> {
		self.finalize_note();
		self.notes
	}
}

/// Merges adjacent Text elements into single elements
fn merge_adjacent_text(elements: Vec<TextElement>) -> Vec<TextElement> {
	let mut merged = Vec::new();
	let mut text_buffer = String::new();

	for element in elements {
		match element {
			TextElement::Text(text) => text_buffer.push_str(&text),
			cloze @ TextElement::Cloze(_) => {
				if !text_buffer.is_empty() {
					merged.push(TextElement::Text(std::mem::take(&mut text_buffer)));
				}
				merged.push(cloze);
			}
		}
	}

	if !text_buffer.is_empty() {
		merged.push(TextElement::Text(text_buffer));
	}

	merged
}

/// Finds a model by name in the available models list
fn find_model<'a>(available_models: &'a [NoteModel], name: &str) -> Option<&'a NoteModel> {
	available_models.iter().find(|m| m.name == name)
}

/// Processes a single parsed item and updates the builder state
fn process_item<'m>(
	item: FlashItem,
	span: Span,
	builder: &mut NoteBuilder<'m>,
	available_models: &'m [NoteModel],
	emitter: &mut Emitter<Rich<Token>>,
) {
	match item {
		FlashItem::NoteModel(name) => {
			let model = find_model(available_models, &name);
			if model.is_none() {
				emitter.emit(Rich::custom(span, format!("Unknown note model '{}'", name)));
			}
			builder.switch_model(model);
		}
		FlashItem::Alias { from, to } => {
			builder.add_alias(from, to);
		}
		FlashItem::Tags(tags) => {
			builder.set_tags(tags);
		}
		FlashItem::Field { name, content } => {
			builder.validate_and_add_field(name, content, span, emitter);
		}
		FlashItem::Comment(_) => {
			// Comments are ignored
		}
		FlashItem::BlankLine => {
			builder.finalize_note();
		}
	}
}

// ----------------------------------------------------------------------------
// Parser
// ----------------------------------------------------------------------------

/// Main parser entry point expecting a token stream
pub fn flash<'tokens, 'src: 'tokens, I>(
	available_models: &'tokens [NoteModel],
) -> impl Parser<'tokens, I, Vec<Note<'tokens>>, extra::Err<Rich<'tokens, Token<'src>>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
	// Utilities
	// Match inline whitespace or the explicit ignore-case for Eq
	let ws = just(Token::Eq).ignore_then(empty()).or(select! {
			Token::WS(_) => (),
	});

	// Helper for Identifiers.
	// We accept `Text` or Keywords (e.g. if someone names a field "alias")
	let ident = select! {
			Token::Text(s) => s.to_string(),
			Token::KwAlias => "alias".to_string(),
			Token::KwTo => "to".to_string(),
	};

	// End of Line: A newline token OR the actual end of input.
	// We use .ignored() on the token to match the () output of end().
	let eol = just(Token::Newline).ignored().or(end());
	let line_ending = ws.clone().repeated().ignore_then(eol);

	// "= Model Name ="
	// We need to reconstruct the model name from potentially multiple text/ws
	// tokens
	let model_name_content = select! {
			Token::Text(s) => s,
			Token::WS(s) => s,
	}
	.repeated()
	.collect::<Vec<_>>()
	.map(|v| v.concat());

	let note_model = just(Token::Eq)
		.ignore_then(model_name_content)
		.then_ignore(just(Token::Eq))
		.map(|name| FlashItem::NoteModel(name.trim().to_string()))
		.then_ignore(line_ending.clone());

	// "alias <from> to <to>"
	let alias = just(Token::KwAlias)
		.ignore_then(ws.clone().repeated())
		.ignore_then(ident)
		.then_ignore(ws.clone().repeated())
		.then_ignore(just(Token::KwTo))
		.then_ignore(ws.clone().repeated())
		.then(ident)
		.map(|(to, from)| FlashItem::Alias { from, to })
		.then_ignore(line_ending.clone());

	// Clozes: {Answer|Hint} or {Answer}
	// Content inside clozes is Text/WS/Keywords
	let text_or_ws = select! {
			Token::Text(s) => s,
			Token::WS(s) => s,
			Token::KwAlias => "alias",
			Token::KwTo => "to",
			Token::Comma => ",",
			Token::Colon => ":",
	};

	let cloze_part = text_or_ws.repeated().at_least(1).collect::<Vec<_>>().map(|v| v.concat());

	let cloze_hint =
		just(Token::Pipe).ignore_then(cloze_part.clone()).map(|s| s.trim().to_string()).or_not();

	let cloze = just(Token::LBrace)
		.ignore_then(cloze_part.clone().map(|s| s.trim().to_string()))
		.then(cloze_hint)
		.then_ignore(just(Token::RBrace))
		.map(|(answer, hint)| TextElement::Cloze(Cloze { id: 0, answer, hint }));

	// Tags: [tag1, tag2]
	// Tags cannot contain newlines or brackets
	let tag_content = select! {
			Token::Text(s) => s,
			Token::WS(s) => s,
			Token::KwAlias => "alias",
			Token::KwTo => "to",
	}
	.repeated()
	.at_least(1)
	.collect::<Vec<_>>()
	.map(|v| v.concat().trim().to_string());

	let tags = tag_content
		.separated_by(just(Token::Comma))
		.allow_trailing()
		.collect::<Vec<_>>()
		.delimited_by(just(Token::LBracket), just(Token::RBracket))
		.map(FlashItem::Tags)
		.then_ignore(line_ending.clone());

	// Field content
	// We must accept anything until EOL that isn't a cloze start
	let content_text = select! {
			Token::Text(s) => s.to_string(),
			Token::WS(s) => s.to_string(),
			Token::KwAlias => "alias".to_string(),
			Token::KwTo => "to".to_string(),
			Token::Comma => ",".to_string(),
			Token::Eq => "=".to_string(),
			Token::LBracket => "[".to_string(),
			Token::RBracket => "]".to_string(),
			Token::Colon => ":".to_string(),
	}
	.map(TextElement::Text);

	let content_element = cloze.or(content_text);

	let content = content_element
		.repeated()
		.collect::<Vec<TextElement>>()
		.validate(|elements, _span, _emitter| merge_adjacent_text(elements));

	// Field Declaration: "Name: Content"
	let field_name = ident.then_ignore(just(Token::Colon));

	let field_pair = field_name
		.then_ignore(ws.repeated())
		.then(content)
		.map(|(name, content)| FlashItem::Field { name, content })
		.then_ignore(line_ending.clone());

	// Comments
	let comment = select! {
			Token::Comment(c) => c
	}
	.map(|s| FlashItem::Comment(s.to_string()))
	.then_ignore(line_ending);

	// Blank line
	// Multiple newlines produce blank line items
	let blank_line = just(Token::Newline).to(FlashItem::BlankLine);

	let item = choice((note_model, alias, tags, field_pair, comment, blank_line))
		.map_with(|item, e| (item, e.span()));

	item.repeated().collect::<Vec<(FlashItem, Span)>>().then_ignore(end()).validate(
		move |items, _span, mut emitter| {
			let mut builder = NoteBuilder::default();

			for (item, span) in items {
				process_item(item, span, &mut builder, available_models, &mut emitter);
			}

			builder.into_notes()
		},
	)
}
