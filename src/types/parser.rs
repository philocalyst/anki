use std::collections::HashMap;

use chumsky::span::SimpleSpan;

use crate::types::note::{NoteField, NoteModel, TextElement};

struct ParserNoteModel<'a> {
	pub model:   Option<&'a NoteModel>,
	pub span:    Option<SimpleSpan>,
	pub aliases: HashMap<String, &'a NoteField>,
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
