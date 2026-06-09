use std::collections::HashMap;

use ankit::NoteBuilder;
use eyre::Result;
use tracing::info;
use uuid::Uuid;

use crate::note::{Identified, Note, TextElement, djot_to_string};
use crate::sync::client::FlashClient;
use crate::sync::identity::{flash_tag_prefix, make_flash_tag};

pub struct NoteSyncData {
	pub uuid:       Uuid,
	pub model_uuid: Uuid,
	pub model_name: String,
	pub fields:     HashMap<String, String>,
	pub tags:       Vec<String>,
}

impl<'model> From<&Identified<Note<'model>>> for NoteSyncData {
	fn from(note: &Identified<Note<'model>>) -> Self {
		let mut fields = HashMap::new();
		for field in &note.inner.fields {
			let value: String = field
				.content
				.iter()
				.map(|elem| match elem {
					TextElement::Text(s) => s.clone(),
					TextElement::Cloze(c) => {
						let answer = djot_to_string(&c.answer);
						if let Some(hint) = &c.hint {
							format!("{{{{c{}::{}::{}}}}}", c.id, answer, hint)
						} else {
							format!("{{{{c{}::{}}}}}", c.id, answer)
						}
					}
				})
				.collect();
			fields.insert(field.name.clone(), value);
		}

		NoteSyncData {
			uuid:       note.id,
			model_uuid: note.inner.model.id,
			model_name: note.inner.model.name.clone(),
			fields,
			tags:       note.inner.tags.clone(),
		}
	}
}

pub async fn sync_note(
	client: &FlashClient,
	note: &NoteSyncData,
	deck_uuid: &Uuid,
	deck_name: &str,
) -> Result<i64> {
	let flash_tag = make_flash_tag(deck_uuid, &note.uuid);

	let existing_ids = client
		.notes()
		.find(&format!("tag:\"{}\"", flash_tag))
		.await
		.map_err(|e| eyre::eyre!("Failed to find note by tag: {}", e))?;

	if let Some(&note_id) = existing_ids.first() {
		info!("Updating note {} (uuid: {})", note_id, note.uuid);

		let mut all_tags = note.tags.clone();
		if !all_tags.contains(&flash_tag) {
			all_tags.push(flash_tag);
		}

		client
			.notes()
			.update(note_id, Some(&note.fields), Some(&all_tags))
			.await
			.map_err(|e| eyre::eyre!("Failed to update note {}: {}", note_id, e))?;

		Ok(note_id)
	} else {
		info!("Creating new note (uuid: {})", note.uuid);

		let mut all_tags = note.tags.clone();
		all_tags.push(flash_tag);

		let note_obj = NoteBuilder::new(deck_name, &note.model_name);
		let note_obj = note
			.fields
			.iter()
			.fold(note_obj, |builder, (name, value)| builder.field(name, value));
		let note_obj = note_obj.tags(all_tags);
		let note_obj = note_obj.build();

		let new_id = client
			.notes()
			.add(note_obj)
			.await
			.map_err(|e| eyre::eyre!("Failed to add note: {}", e))?;

		Ok(new_id)
	}
}

pub async fn get_tagged_note_ids(client: &FlashClient, deck_uuid: &Uuid) -> Result<Vec<i64>> {
	let query = flash_tag_prefix(deck_uuid);
	let ids = client
		.notes()
		.find(&query)
		.await
		.map_err(|e| eyre::eyre!("Failed to find tagged notes: {}", e))?;
	Ok(ids)
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;

	use uuid::Uuid;

	use crate::note::{Identified, Note, NoteField, NoteModel, TextElement};

	use super::NoteSyncData;

	fn text_field(name: &str, value: &str) -> NoteField<'static> {
		NoteField {
			name: name.into(),
			content: vec![TextElement::Text(value.into())],
		}
	}

	#[test]
	fn from_identified_note_basic_fields() {
		let model = NoteModel::default();
		let note = Note {
			fields: vec![text_field("Front", "Hello"), text_field("Back", "World")],
			model: Cow::Owned(model.clone()),
			tags: vec!["tag1".into(), "tag2".into()],
		};
		let identified = Identified { id: Uuid::from_u128(1), inner: note };

		let data = NoteSyncData::from(&identified);

		assert_eq!(data.uuid, identified.id);
		assert_eq!(data.model_uuid, model.id);
		assert_eq!(data.model_name, model.name);
		assert_eq!(data.fields.len(), 2);
		assert_eq!(data.fields.get("Front"), Some(&"Hello".to_string()));
		assert_eq!(data.fields.get("Back"), Some(&"World".to_string()));
		assert_eq!(data.tags, vec!["tag1", "tag2"]);
	}

	#[test]
	fn from_identified_note_handles_empty_tags() {
		let note = Note {
			fields: vec![text_field("Front", "Test")],
			model: Cow::Owned(NoteModel::default()),
			tags: vec![],
		};
		let identified = Identified { id: Uuid::new_v4(), inner: note };

		let data = NoteSyncData::from(&identified);
		assert!(data.tags.is_empty());
	}

	#[test]
	fn from_identified_note_handles_multiple_fields() {
		let mut model = NoteModel::default();
		model.fields = vec![
			crate::note::Field {
				name:             "Front".into(),
				sticky:           None,
				associated_media: None,
			},
			crate::note::Field {
				name:             "Back".into(),
				sticky:           None,
				associated_media: None,
			},
			crate::note::Field {
				name:             "Extra".into(),
				sticky:           None,
				associated_media: None,
			},
		];

		let note = Note {
			fields: vec![
				text_field("Front", "Q"),
				text_field("Back", "A"),
				text_field("Extra", "Note"),
			],
			model: Cow::Owned(model),
			tags: vec![],
		};
		let identified = Identified { id: Uuid::new_v4(), inner: note };

		let data = NoteSyncData::from(&identified);
		assert_eq!(data.fields.len(), 3);
		assert_eq!(data.fields.get("Front"), Some(&"Q".to_string()));
		assert_eq!(data.fields.get("Back"), Some(&"A".to_string()));
		assert_eq!(data.fields.get("Extra"), Some(&"Note".to_string()));
	}
}
