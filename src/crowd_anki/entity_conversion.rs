use crate::{crowd_anki::{CrowdAnkiEntity, Deck as CrowdAnkiDeck, Field, Note, NoteModelType}, deck::Deck, note::{Cloze, Identified, Note as FlashNote, NoteModel, TextElement}};

impl<'model> From<Deck<'model>> for CrowdAnkiEntity {
	fn from(deck: Deck<'model>) -> Self {
		// Convert note models from deck to CrowdAnki format
		let note_models: Vec<crate::crowd_anki::NoteModel> =
			deck.models.iter().map(|model| model.into()).collect();

		// Convert notes to CrowdAnki format
		let crowd_anki_notes: Vec<Note> = deck.cards.into_iter().map(|note| note.into()).collect();

		// Use the deck's configuration
		let deck_config = deck.configuration;
		let deck_config_uuid = deck_config.crowdanki_uuid.clone();
		let deck_uuid = deck_config.crowdanki_uuid.clone();
		let deck_name = deck_config.name.clone();

		CrowdAnkiEntity::Deck(CrowdAnkiDeck {
			name: deck_name,
			crowdanki_uuid: deck_uuid,
			deck_config_uuid,
			desc: String::new(), // Could be extended to read from deck metadata
			is_dynamic: 0,
			extend_new: 0,
			extend_rev: 0,
			note_models,
			deck_configurations: vec![deck_config],
			notes: crowd_anki_notes,
			children: Vec::new(),
			media_files: Vec::new(),
		})
	}
}

impl<'model> From<&'model NoteModel> for crate::crowd_anki::NoteModel {
	fn from(model: &'model NoteModel) -> Self {
		crate::crowd_anki::NoteModel {
			crowdanki_uuid: model.id.to_string(),
			name:           model.name.clone(),
			kind:           NoteModelType::Standard,
			flds:           model
				.fields
				.iter()
				.enumerate()
				.map(|(idx, field)| Field {
					name:   field.name.clone(),
					ord:    idx as i32,
					sticky: field.sticky.unwrap_or(false),
					rtl:    model.defaults.as_ref().map(|d| d.rtl).unwrap_or(false),
					font:   model
						.defaults
						.as_ref()
						.map(|d| d.font.clone())
						.unwrap_or_else(|| "Arial".to_string()),
					size:   model.defaults.as_ref().map(|d| d.size).unwrap_or(20) as i32,
					media:  Vec::new(),
				})
				.collect(),
			tmpls:          model
				.templates
				.iter()
				.enumerate()
				.map(|(idx, tmpl)| crate::crowd_anki::Template {
					name:  tmpl.name.clone(),
					ord:   idx as i32,
					qfmt:  tmpl.question_format.clone(),
					afmt:  tmpl.answer_format.clone(),
					bafmt: Some(tmpl.browser_answer_format.clone()),
					bqfmt: Some(tmpl.browser_question_format.clone()),
					did:   None,
				})
				.collect(),
			css:            model.css.clone(),
			did:            None,
			latex_pre:      model.latex_pre.clone(),
			latex_post:     model.latex_post.clone(),
			req:            None,
			sortf:          model
				.sort_field
				.as_ref()
				.and_then(|sf| model.fields.iter().position(|f| f.name == *sf))
				.map(|pos| pos as i32),
			tags:           model.tags.clone(),
			vers:           None,
		}
	}
}

/// This type represents Cloze's as anki expects them in note fields
pub struct ClozeString(String);

impl<'content> From<Cloze<'content>> for ClozeString {
	fn from(cloze: Cloze<'content>) -> Self {
		let answer = crate::note::djot_to_string(&cloze.answer);
		if let Some(hint) = cloze.hint {
			ClozeString(format!("{{{{c{}::{}::{}}}}}", cloze.id, answer, hint))
		} else {
			ClozeString(format!("{{{{c{}::{}}}}}", cloze.id, answer))
		}
	}
}

impl<'model> From<Identified<FlashNote<'model>>> for Note {
	fn from(note: Identified<FlashNote<'model>>) -> Self {
		let inner_note = note.inner;
		Note {
			guid:            note.id.to_string(),
			note_model_uuid: inner_note.model.id.to_string(),
			fields:          inner_note
				.fields
				.into_iter()
				.map(|field| {
					field
						.content
						.into_iter()
						.map(|elem| match elem {
							TextElement::Text(s) => s,
							TextElement::Cloze(c) => {
								// Turn into cloze string
								let clozed: ClozeString = c.into();
								clozed.0
							}
						})
						.collect::<String>()
				})
				.collect(),
			tags:            inner_note.tags,
			flags:           0,
			newly_added:     true,
			data:            None,
		}
	}
}
