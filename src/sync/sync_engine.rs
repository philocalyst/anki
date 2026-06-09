use std::collections::HashSet;
use std::path::Path;

use eyre::Result;
use tracing::info;
use uuid::Uuid;

use crate::config::DeckConfig;
use crate::note::Identified;
use crate::note::Note as FlashNote;

use crate::sync::client::FlashClient;
use crate::sync::connection::{self, CollectionSnapshot};
use crate::sync::deck_sync::{self, DeckSyncData};
use crate::sync::media_sync;
use crate::sync::model_sync::{self, ModelSyncData, TemplateSyncData};
use crate::sync::note_sync::{self, NoteSyncData};
use crate::sync::reconcile;

pub struct SyncEngine {
	client:   FlashClient,
	snapshot: CollectionSnapshot,
}

impl SyncEngine {
	pub async fn new() -> Result<Self> {
		let client = FlashClient(ankit::AnkiClient::new());

		connection::check_connection(&client).await?;
		info!("Connection to Anki verified");

		let snapshot = connection::get_collection_snapshot(&client).await?;
		info!(
			"Collection snapshot: {} deck(s), {} model(s)",
			snapshot.decks.len(),
			snapshot.models.len()
		);

		Ok(Self { client, snapshot })
	}

	pub async fn sync(
		&mut self,
		deck_path: &Path,
		deck_uuid: &Uuid,
		deck_name: &str,
		models: &[crate::note::NoteModel],
		cards: &[Identified<FlashNote<'_>>],
		config: &DeckConfig,
	) -> Result<()> {
		info!("Syncing deck '{}' ({})", deck_name, deck_uuid);

		// 1. Sync deck metadata
		let deck_data =
			DeckSyncData { uuid: *deck_uuid, name: deck_name.to_string(), config: config.clone() };

		let resolved_deck_name = deck_sync::sync_deck(&self.client, &deck_data, &self.snapshot).await?;
		self.snapshot.decks.insert(resolved_deck_name.clone(), 0);
		info!("Deck '{}' synced", resolved_deck_name);

		// 2. Sync note models
		let mut model_name_map: Vec<(Uuid, String)> = Vec::new();
		for model in models {
			let model_data = ModelSyncData {
				uuid: model.id,
				name: model.name.clone(),
				fields: model.fields.iter().map(|f| f.name.clone()).collect(),
				templates: model
					.templates
					.iter()
					.map(|t| TemplateSyncData {
						name: t.name.clone(),
						front: t.question_format.clone(),
						back: t.answer_format.clone(),
					})
					.collect(),
				css: model.css.clone(),
				latex_pre: model.latex_pre.clone(),
				latex_post: model.latex_post.clone(),
			};
			let anki_name = model_sync::sync_model(&self.client, &model_data).await?;
			model_name_map.push((model.id, anki_name));
		}

		// Build model ID -> name lookup
		let model_name_lookup: std::collections::HashMap<Uuid, String> =
			model_name_map.into_iter().collect();

		// 3. Sync notes
		let current_uuids: HashSet<Uuid> = cards.iter().map(|c| c.id).collect();

		for card in cards {
			let model_name = match model_name_lookup.get(&card.inner.model.id) {
				Some(name) => name.clone(),
				None => card.inner.model.name.clone(),
			};

			let note_data = NoteSyncData {
				uuid: card.id,
				model_uuid: card.inner.model.id,
				model_name,
				fields: render_fields(card),
				tags: card.inner.tags.clone(),
			};

			note_sync::sync_note(&self.client, &note_data, deck_uuid, &resolved_deck_name).await?;
		}

		info!("Synced {} note(s)", cards.len());

		// 4. Reconcile deletions
		reconcile::reconcile_deletions(&self.client, &resolved_deck_name, deck_uuid, &current_uuids)
			.await?;

		// 5. Sync media
		media_sync::sync_media(&self.client, deck_path).await?;

		info!("Deck '{}' sync complete", resolved_deck_name);
		Ok(())
	}
}

fn render_fields(card: &Identified<FlashNote<'_>>) -> std::collections::HashMap<String, String> {
	use crate::note::{TextElement, djot_to_string};

	card
		.inner
		.fields
		.iter()
		.map(|field| {
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
			(field.name.clone(), value)
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;

	use uuid::Uuid;

	use crate::note::{Cloze, Identified, Note, NoteField, NoteModel, TextElement};

	use super::render_fields;

	fn text_field(name: &str, value: &str) -> NoteField<'static> {
		NoteField { name: name.into(), content: vec![TextElement::Text(value.into())] }
	}

	#[test]
	fn render_fields_plain_text() {
		let note = Note {
			fields: vec![text_field("Front", "Hello"), text_field("Back", "World")],
			model: Cow::Owned(NoteModel::default()),
			tags: vec![],
		};
		let card = Identified { id: Uuid::new_v4(), inner: note };

		let result = render_fields(&card);
		assert_eq!(result.len(), 2);
		assert_eq!(result.get("Front"), Some(&"Hello".to_string()));
		assert_eq!(result.get("Back"), Some(&"World".to_string()));
	}

	#[test]
	fn render_fields_empty_content() {
		let note = Note {
			fields: vec![text_field("Front", "")],
			model: Cow::Owned(NoteModel::default()),
			tags: vec![],
		};
		let card = Identified { id: Uuid::new_v4(), inner: note };

		let result = render_fields(&card);
		assert_eq!(result.get("Front"), Some(&"".to_string()));
	}

	#[test]
	fn render_fields_multiple_text_elements_in_field() {
		let field = NoteField {
			name: "Front".into(),
			content: vec![
				TextElement::Text("Hello ".into()),
				TextElement::Text("World ".into()),
				TextElement::Text("from Flash".into()),
			],
		};
		let note = Note { fields: vec![field], model: Cow::Owned(NoteModel::default()), tags: vec![] };
		let card = Identified { id: Uuid::new_v4(), inner: note };

		let result = render_fields(&card);
		assert_eq!(result.get("Front"), Some(&"Hello World from Flash".to_string()));
	}

	#[test]
	fn render_fields_cloze_without_hint() {
		let field = NoteField {
			name: "Text".into(),
			content: vec![
				TextElement::Text("The capital of France is ".into()),
				TextElement::Cloze(Cloze {
					id: 1,
					answer: vec![jotdown::Event::Str(std::borrow::Cow::Borrowed("Paris"))],
					hint: None,
				}),
				TextElement::Text(".".into()),
			],
		};
		let note = Note { fields: vec![field], model: Cow::Owned(NoteModel::default()), tags: vec![] };
		let card = Identified { id: Uuid::new_v4(), inner: note };

		let result = render_fields(&card);
		assert_eq!(result.get("Text"), Some(&"The capital of France is {{c1::Paris}}.".to_string()));
	}

	#[test]
	fn render_fields_cloze_with_hint() {
		let field = NoteField {
			name: "Text".into(),
			content: vec![
				TextElement::Text("A synonym for happy is ".into()),
				TextElement::Cloze(Cloze {
					id: 2,
					answer: vec![jotdown::Event::Str(std::borrow::Cow::Borrowed("joyful"))],
					hint: Some("starts with j".into()),
				}),
				TextElement::Text(".".into()),
			],
		};
		let note = Note { fields: vec![field], model: Cow::Owned(NoteModel::default()), tags: vec![] };
		let card = Identified { id: Uuid::new_v4(), inner: note };

		let result = render_fields(&card);
		assert_eq!(
			result.get("Text"),
			Some(&"A synonym for happy is {{c2::joyful::starts with j}}.".to_string())
		);
	}

	#[test]
	fn render_fields_mixed_cloze_and_text() {
		let field = NoteField {
			name: "Q".into(),
			content: vec![
				TextElement::Cloze(Cloze {
					id: 1,
					answer: vec![jotdown::Event::Str(std::borrow::Cow::Borrowed("Berlin"))],
					hint: None,
				}),
				TextElement::Text(" is the capital of ".into()),
				TextElement::Cloze(Cloze {
					id: 2,
					answer: vec![jotdown::Event::Str(std::borrow::Cow::Borrowed("Germany"))],
					hint: Some("European country".into()),
				}),
			],
		};
		let note = Note { fields: vec![field], model: Cow::Owned(NoteModel::default()), tags: vec![] };
		let card = Identified { id: Uuid::new_v4(), inner: note };

		let result = render_fields(&card);
		assert_eq!(
			result.get("Q"),
			Some(&"{{c1::Berlin}} is the capital of {{c2::Germany::European country}}".to_string())
		);
	}

	#[test]
	fn render_fields_returns_correct_field_names() {
		let note = Note {
			fields: vec![
				text_field("Question", "What?"),
				text_field("Answer", "That!"),
				text_field("Extra", "Details..."),
			],
			model: Cow::Owned(NoteModel::default()),
			tags: vec![],
		};
		let card = Identified { id: Uuid::new_v4(), inner: note };

		let result = render_fields(&card);
		let mut keys: Vec<_> = result.keys().cloned().collect();
		keys.sort();
		assert_eq!(keys, vec!["Answer", "Extra", "Question"]);
	}
}
