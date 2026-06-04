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
