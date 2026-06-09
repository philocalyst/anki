use tracing::instrument;

use crate::note::{Note, TextElement, djot_to_string};

impl<'model> Note<'model> {
	/// Generate a deterministic string representation of the note's content
	/// for UUID generation
	#[instrument(skip(self))]
	pub fn to_content_string(&self) -> String {
		let mut content = String::new();

		for field in &self.fields {
			content.push_str(&field.name);

			let field_content = field
				.content
				.iter()
				.map(|part| match part {
					TextElement::Text(text) => text.clone(),
					TextElement::Cloze(cloze) => djot_to_string(&cloze.answer),
				})
				.collect::<Vec<String>>()
				.join("\0");

			content.push_str(&field_content);
		}

		content
	}
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;

	use crate::note::{Cloze, Note, NoteField, NoteModel, TextElement};

	fn text_field(name: &str, value: &str) -> NoteField<'static> {
		NoteField { name: name.into(), content: vec![TextElement::Text(value.into())] }
	}

	#[test]
	fn to_content_string_empty_fields() {
		let note = Note {
			fields: Vec::new(),
			model: Cow::Owned(NoteModel::default()),
			tags: Vec::new(),
		};
		assert_eq!(note.to_content_string(), "");
	}

	#[test]
	fn to_content_string_single_field() {
		let note = Note {
			fields: vec![text_field("Front", "Hello World")],
			model: Cow::Owned(NoteModel::default()),
			tags: Vec::new(),
		};
		assert_eq!(note.to_content_string(), "FrontHello World");
	}

	#[test]
	fn to_content_string_multiple_fields() {
		let note = Note {
			fields: vec![text_field("Front", "Hello"), text_field("Back", "World")],
			model: Cow::Owned(NoteModel::default()),
			tags: Vec::new(),
		};
		assert_eq!(note.to_content_string(), "FrontHelloBackWorld");
	}

	#[test]
	fn to_content_string_with_empty_field_content() {
		let note = Note {
			fields: vec![text_field("Front", "")],
			model: Cow::Owned(NoteModel::default()),
			tags: Vec::new(),
		};
		assert_eq!(note.to_content_string(), "Front");
	}

	#[test]
	fn to_content_string_separates_field_elements_with_null() {
		let field = NoteField {
			name: "F".into(),
			content: vec![TextElement::Text("A".into()), TextElement::Text("B".into())],
		};
		let note = Note {
			fields: vec![field],
			model: Cow::Owned(NoteModel::default()),
			tags: Vec::new(),
		};
		assert_eq!(note.to_content_string(), "FA\0B");
	}

	#[test]
	fn to_content_string_with_cloze() {
		let field = NoteField {
			name: "Text".into(),
			content: vec![
				TextElement::Text("Capital of France is ".into()),
				TextElement::Cloze(Cloze {
					id: 1,
					answer: vec![jotdown::Event::Str(std::borrow::Cow::Borrowed("Paris"))],
					hint: None,
				}),
			],
		};
		let note = Note {
			fields: vec![field],
			model: Cow::Owned(NoteModel::default()),
			tags: Vec::new(),
		};
		// Elements within a field are joined with null byte
		assert_eq!(note.to_content_string(), "TextCapital of France is \0Paris");
	}
}
