use std::{error::Error, fs, path::Path};

use crate::types::{crowd_anki_config::{DeckConfig, LapseConfig, NewConfig, RevConfig}, crowd_anki_models::{CrowdAnkiEntity, Deck, Field, Note, NoteModelType}, note::Cloze};

impl super::note::NoteModel {
	pub fn complete(&mut self, dir: &Path) -> Result<(), Box<dyn Error>> {
		// Load CSS if present
		let css_path = dir.join("style.css");
		if css_path.exists() {
			self.css = fs::read_to_string(css_path)?;
		}

		// Load LaTeX pre/post if present
		let pre_path = dir.join("pre.tex");
		if pre_path.exists() {
			self.latex_pre = Some(fs::read_to_string(pre_path)?);
		}

		let post_path = dir.join("post.tex");
		if post_path.exists() {
			self.latex_post = Some(fs::read_to_string(post_path)?);
		}

		// Load templates from .hbs files
		let mut templates = Vec::new();
		for entry in fs::read_dir(dir)? {
			let entry = entry?;
			let path = entry.path();

			if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
				if ext == "hbs" {
					let filename = path.file_stem().unwrap().to_string_lossy().to_string();

					// Parse naming convention: NAME+front.hbs, NAME+back.browser.hbs, etc.
					let parts: Vec<&str> = filename.split('+').collect();
					if parts.len() != 2 {
						continue; // skip malformed
					}

					let template_name = parts[0].to_string();
					let side = parts[1];

					// Find or create template
					let tmpl =
						templates.iter_mut().find(|t: &&mut super::config::Template| t.name == template_name);

					let tmpl = if let Some(t) = tmpl {
						t
					} else {
						templates.push(super::config::Template {
							name:                    template_name.clone(),
							order:                   templates.len() as i32,
							question_format:         String::new(),
							answer_format:           String::new(),
							browser_question_format: String::new(),
							browser_answer_format:   String::new(),
						});
						templates.last_mut().unwrap()
					};

					let content = fs::read_to_string(&path)?;

					// Assign based on side
					if side.starts_with("front") {
						if side.contains("browser") {
							tmpl.browser_question_format = content;
						} else {
							tmpl.question_format = content;
						}
					} else if side.starts_with("back") {
						if side.contains("browser") {
							tmpl.browser_answer_format = content;
						} else {
							tmpl.answer_format = content;
						}
					}
				}
			}
		}

		self.templates = templates;
		Ok(())
	}
}

use uuid::Uuid;

impl<'a> From<Vec<crate::types::note::Note<'a>>> for CrowdAnkiEntity {
	fn from(notes: Vec<crate::types::note::Note<'a>>) -> Self {
		// Extract unique note models from the notes
		let note_models: Vec<crate::types::crowd_anki_models::NoteModel> = notes
			.iter()
			.map(|note| note.model)
			.collect::<std::collections::HashSet<_>>()
			.into_iter()
			.map(|model| model.into())
			.collect();

		// Convert notes to CrowdAnki format
		let crowd_anki_notes: Vec<Note> = notes.into_iter().map(|note| note.into()).collect();

		// Create a default deck configuration
		let deck_config = DeckConfig {
			crowdanki_uuid:  Uuid::new_v4().to_string(),
			name:            "Default".to_string(),
			is_dynamic:      false,
			max_taken:       Some(20),
			new:             Some(NewConfig {
				delays:         vec![1, 10],
				ints:           vec![1, 4, 0],
				initial_factor: Some(2500),
				per_day:        Some(20),
				order:          Some(1),
				bury:           Some(false),
				separate:       Some(true),
			}),
			rev:             Some(RevConfig {
				per_day:     Some(200),
				ease4:       Some(1.3),
				ivl_fct:     Some(1.0),
				fuzz:        Some(0.05),
				hard_factor: Some(1.2),
				max_ivl:     Some(36500),
				min_space:   Some(1),
				bury:        Some(false),
			}),
			lapse:           Some(LapseConfig {
				delays:       vec![10],
				mult:         0.0,
				min_int:      Some(1),
				leech_action: Some(0),
				leech_fails:  Some(8),
			}),
			autoplay:        Some(true),
			replayq:         Some(true),
			timer:           Some(0),
			another_retreat: Some(false),
		};

		let deck_config_uuid = deck_config.crowdanki_uuid.clone();

		CrowdAnkiEntity::Deck(Deck {
			name: "Generated Deck".to_string(),
			crowdanki_uuid: Uuid::new_v4().to_string(),
			deck_config_uuid,
			desc: String::new(),
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

impl<'a> From<&'a crate::types::note::NoteModel> for super::crowd_anki_models::NoteModel {
	fn from(model: &'a crate::types::note::NoteModel) -> Self {
		super::crowd_anki_models::NoteModel {
			crowdanki_uuid: Uuid::new_v4().to_string(),
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
				.map(|(idx, tmpl)| super::crowd_anki_models::Template {
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

impl<'a> From<Cloze> for ClozeString {
	fn from(cloze: Cloze) -> Self {
		if let Some(hint) = cloze.hint {
			ClozeString(format!("{{{{c{}::{}::{}}}}}", cloze.id, cloze.answer, hint))
		} else {
			ClozeString(format!("{{{{c{}::{}}}}}", cloze.id, cloze.answer))
		}
	}
}

impl<'a> From<crate::types::note::Note<'a>> for Note {
	fn from(note: crate::types::note::Note<'a>) -> Self {
		Note {
			guid:            Uuid::new_v4().to_string(),
			note_model_uuid: Uuid::new_v4().to_string(),
			fields:          note
				.fields
				.into_iter()
				.map(|field| {
					field
						.content
						.into_iter()
						.map(|elem| match elem {
							crate::types::note::TextElement::Text(s) => s,
							crate::types::note::TextElement::Cloze(c) => {
								// Turn into cloze string
								let clozed: ClozeString = c.into();

								clozed.0
							}
						})
						.collect::<String>()
				})
				.collect(),
			tags:            note.tags,
			flags:           0,
			newly_added:     true,
			data:            None,
		}
	}
}
