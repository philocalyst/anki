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
// Lexer (unchanged)
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
	Alias,

	#[token("to", priority = 5)]
	To,

	#[token("\n")]
	Newline,

	#[regex(r"[ \t]+")]
	WS(&'a str),

	#[regex(r"[^ \t\n:=\[\]{},|]+", priority = 4)]
	Text(&'a str),

	#[regex(r"//[^\n]*", allow_greedy = true, priority = 3)]
	Comment(&'a str),

	// Matches from start of line/whitespace to a colon
	#[regex(r"[^ \t\n:=\[\]{},|]+:", |lex| lex.slice().strip_suffix(':'))]
	Field(&'a str),

	Error,
}

// Basic Token extractors

/// Extract whitespace (including = as special whitespace)
/// Extract structural whitespace
fn ws<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, (), extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	select! { Token::WS(_) => () }.labelled("whitespace")
}

/// Extract identifier-like tokens (Text, alias, to)
fn ident<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, &'src str, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	select! {
		Token::Alias => "alias",
		Token::To => "to",
	}
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

/// End of line: newline or EOF
fn eol<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, (), extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	just(Token::Newline).ignored().or(end())
}

/// Line ending with optional leading whitespace
fn line_ending<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, (), extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	ws().repeated().ignore_then(eol())
}

// ----------------------------------------------------------------------------
// Semantic Parsers
// ----------------------------------------------------------------------------

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

	just(Token::Eq)
		.ignore_then(model_name_parts.repeated().collect::<Vec<_>>())
		.then_ignore(just(Token::Eq))
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
		.then_ignore(line_ending())
		.labelled("tags")
}

/// Parse cloze: {Answer|Hint}
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

	let hint =
		just(Token::Pipe).ignore_then(cloze_part.clone()).map(|s| s.trim().to_string()).or_not();

	just(Token::LBrace)
		.ignore_then(cloze_part.map(|s| s.trim().to_string()))
		.then(hint)
		.then_ignore(just(Token::RBrace))
		.map(|(answer, hint)| TextElement::Cloze(Cloze { id: 0, answer, hint }))
		.labelled("cloze")
}

/// Parse field content (text and clozes)
fn field_content<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Vec<TextElement>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	let text_chars = select! {
		Token::Text(s) => s,
		Token::WS(s) => s,
		Token::Alias => "alias",
		Token::To => "to",
		Token::Comma => ",",
		Token::Eq => "=",
		Token::LBracket => "[",
		Token::RBracket => "]",
		Token::Colon => ":",
	};

	let content_text = text_chars.map(|s: &str| TextElement::Text(s.to_string()));
	let content_element = cloze().or(content_text);

	content_element.repeated().collect::<Vec<_>>().map(merge_adjacent_text)
}

/// Parse field: Name: Content
fn field_declaration<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, NoteField, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	select! {Token::Field(s) => s}
		.map(|s| s.to_string())
		.then_ignore(ws().repeated())
		.then(field_content())
		.map(|(name, content)| NoteField { name, content })
		.then_ignore(line_ending())
		.labelled("field")
}

/// Parse comment line
fn comment_line<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, (), extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	select! { Token::Comment(_) => () }.then_ignore(line_ending()).labelled("comment")
}

// ----------------------------------------------------------------------------
// Note Builder
// ----------------------------------------------------------------------------

/// Merges adjacent Text elements to reduce fragmentation
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

/// Build a note from parsed components
struct NoteComponents<'m> {
	model:   &'m NoteModel,
	aliases: HashMap<String, String>,
	tags:    Vec<String>,
	fields:  Vec<NoteField>,
}

impl<'m> NoteComponents<'m> {
	fn into_note(mut self) -> Note<'m> {
		// Resolve aliases in fields
		for field in &mut self.fields {
			if let Some(target) = self.aliases.get(&field.name) {
				field.name = target.clone();
			}
		}

		Note { fields: self.fields, model: std::borrow::Cow::Borrowed(self.model), tags: self.tags }
	}
}

// ----------------------------------------------------------------------------
// Main Parser
// ----------------------------------------------------------------------------

pub fn flash<'tokens, 'src: 'tokens, I>(
	available_models: &'tokens [NoteModel],
) -> impl Parser<'tokens, I, Vec<Note<'tokens>>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>,
{
	let blank_line = just(Token::Newline);

	let note_block = model_declaration()
        .validate(move |model_name, extra, emitter| {
            let span = extra.span();
            available_models
                .iter()
                .find(|m| m.name == model_name)
                .map_or_else(|| {
                    let available = available_models
                        .iter()
                        .map(|m| m.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    emitter.emit(Rich::custom(span, format!("Unknown model '{}'. Available: [{}]", model_name, available)));
                    None
                }, Some)
        })		.then_ignore(line_ending())
        .then(alias_declaration()		.then_ignore(line_ending())
.repeated().collect::<Vec<_>>())		.then_ignore(blank_line.clone())
        .then(tags_declaration().or_not())
        .then(field_declaration().repeated().at_least(1).collect::<Vec<_>>())
        .validate(move |(((model_opt, aliases), tags), fields), extra, emitter| {
            let model = model_opt?;
            let span = extra.span();

            let alias_map: HashMap<_, _> = aliases.into_iter().collect();      for field in &fields {
                let resolved_name = alias_map.get(&field.name).unwrap_or(&field.name);
                if !model.fields.iter().any(|f| &f.name == resolved_name) {
                    emitter.emit(Rich::custom(
                        span,
                        format!("Field '{}' not found in model '{}'", field.name, model.name),
                    ));
                }
            }Some(NoteComponents {
                model,
                aliases: alias_map,
                tags: tags.unwrap_or_default(),
                fields,
            }
            .into_note())
        })
        // Fix: Use try_map to unwrap the Option and filter out None values
        .try_map(|opt, span| opt.ok_or_else(|| Rich::custom(span, "Invalid note structure")))
        .then_ignore(blank_line.clone().ignored().or(end()))
        .recover_with(skip_then_retry_until(any().ignored(), blank_line.clone().ignored()));

	note_block
        .padded_by(comment_line().repeated())
        .padded_by(blank_line.repeated())
        .repeated()
        .collect::<Vec<_>>() // Now this successfully collects into Vec<Note>
        .then_ignore(end())
}
