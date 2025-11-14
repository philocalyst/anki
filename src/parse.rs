use std::collections::HashMap;

use chumsky::{input::Emitter, prelude::*};

use crate::types::{note::{Cloze, Note, NoteField, NoteModel, TextElement}, parser::FlashItem};

type Span = SimpleSpan;

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
				tags: std::mem::take(&mut self.tags),
				model,
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
			// Inserting with the key of to, as during resolution time, what's being
			// searched is the to case (an alias within the flash file), in an attempt to
			// find what it's linked to
			self.aliases.insert(to, from);
		}
	}

	fn validate_and_add_field(
		&mut self,
		name: String,
		content: Vec<TextElement>,
		span: Span,
		emitter: &mut Emitter<Rich<char>>,
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
		emitter: &mut Emitter<Rich<char>>,
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
	emitter: &mut Emitter<Rich<char>>,
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

/// Main parser entry point
pub fn flash<'a>(
	available_models: &'a [NoteModel],
) -> impl Parser<'a, &'a str, Vec<Note<'a>>, extra::Err<Rich<'a, char>>> + Clone {
	// Inline whitespace (spaces and tabs only; excludes newlines)
	let ws = one_of(" \t").repeated().ignored();

	// A line can end with a newline or the end of the input
	let eol = text::newline().or(end());
	let line_ending = ws.clone().then_ignore(eol);

	// "= Model Name =" line
	let note_model = just('=')
		.ignore_then(none_of('=').repeated().collect::<String>())
		.then_ignore(just('='))
		.map(|name| FlashItem::NoteModel(name.trim().to_string()))
		.then_ignore(line_ending.clone());

	// "alias <from> to <to>"
	let identifier = none_of([' ', '\t', '\n', ':']).repeated().at_least(1).collect::<String>();

	let alias = text::keyword("alias")
		.ignore_then(ws.clone())
		.ignore_then(identifier.clone())
		.then_ignore(ws.clone())
		.then_ignore(text::keyword("to"))
		.then_ignore(ws.clone())
		.then(identifier)
		.map(|(from, to)| FlashItem::Alias { from, to })
		.then_ignore(line_ending.clone());

	// Cloze parsing
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
	let tag =
		none_of(",[]\n").repeated().at_least(1).collect::<String>().map(|s| s.trim().to_string());

	let tags = tag
		.separated_by(just(',').padded())
		.allow_trailing()
		.collect::<Vec<_>>()
		.delimited_by(just('['), just(']'))
		.map(FlashItem::Tags)
		.then_ignore(line_ending.clone());

	// Field content (text and cloze deletions)
	let regular_text =
		none_of(['{', '#', '\n']).repeated().at_least(1).collect::<String>().map(TextElement::Text);

	let content_element = cloze.or(regular_text);

	let content = content_element
		.repeated()
		.collect::<Vec<TextElement>>()
		.validate(|elements, _span, _emitter| merge_adjacent_text(elements));

	// Field declaration: "FieldName: content"
	let field_name = text::ident().map(|s: &str| s.to_string()).then_ignore(just(':'));

	let field_pair = field_name
		.then_ignore(ws.clone())
		.then(content)
		.map(|(name, content)| FlashItem::Field { name, content })
		.then_ignore(line_ending.clone());

	// Comment line: "// comment text"
	let comment = just("//")
		.ignore_then(none_of('\n').repeated().collect::<String>())
		.map(FlashItem::Comment)
		.then_ignore(line_ending.clone());

	// A blank line is now just a newline that isn't part of another item's ending
	let blank_line = text::newline().to(FlashItem::BlankLine);

	// A single item in the input. Order matters.
	let item = choice((note_model, alias, tags, field_pair, comment, blank_line))
		.map_with(|item, e| (item, e.span()));

	// Parse all items and build notes
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
