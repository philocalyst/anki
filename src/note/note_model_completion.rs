use std::{fs, path::Path};

use tracing::{debug, info};

use crate::{error::DeckError, note::{Complete, NoteModel, Partial}};

impl NoteModel<Partial> {
	pub fn complete(self, dir: &Path) -> Result<NoteModel<Complete>, DeckError> {
		let mut css = String::new();
		let css_path = dir.join("style.css");
		if css_path.exists() {
			debug!("Loading CSS");
			css = fs::read_to_string(css_path)?;
		}

		let name =
			dir.file_name().unwrap().to_string_lossy().rsplit_once(".model").unwrap().0.to_string();

		let mut latex_pre = None;
		let pre_path = dir.join("pre.tex");
		if pre_path.exists() {
			debug!("Loading pre TEX");
			latex_pre = Some(fs::read_to_string(pre_path)?);
		}

		let mut latex_post = None;
		let post_path = dir.join("post.tex");
		if post_path.exists() {
			debug!("Loading post TEX");
			latex_post = Some(fs::read_to_string(post_path)?);
		}

		info!("Loading templates");
		// Load templates from .hbs files
		let mut templates = Vec::new();
		for entry in fs::read_dir(dir)? {
			let entry = entry?;
			let path = entry.path();

			if let Some(ext) = path.extension().and_then(|e| e.to_str())
				&& ext == "hbs"
			{
				let filename = path.file_stem().unwrap().to_string_lossy().to_string();

				// Parse naming convention: NAME+front.hbs, NAME+back.browser.hbs, etc.
				let parts: Vec<&str> = filename.split('+').collect();
				if parts.len() != 2 {
					return Err(DeckError::InvalidTemplateFilename(filename));
				}

				let template_name = parts[0].to_string();
				let side = parts[1];

				// Find or create template
				let tmpl = templates
					.iter_mut()
					.find(|t: &&mut crate::note::model_config::Template| t.name == template_name);

				let tmpl = if let Some(t) = tmpl {
					t
				} else {
					templates.push(crate::note::model_config::Template {
						name:                    template_name.clone(),
						order:                   templates.len() as i32,
						question_format:         String::new(),
						answer_format:           String::new(),
						browser_question_format: String::new(),
						browser_answer_format:   String::new(),
					});
					templates.last_mut().unwrap()
				};

				let content =
					fs::read_to_string(&path).map_err(|_| DeckError::TemplateNotFound(path.clone()))?;

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

		Ok(NoteModel {
			name,
			id: self.id,
			templates,
			schema_version: self.schema_version,
			defaults: self.defaults,
			css,
			fields: self.fields,
			latex_pre,
			latex_post,
			sort_field: self.sort_field,
			tags: self.tags,
			required: self.required,
		})
	}
}

#[cfg(test)]
mod tests {
	use std::fs;

	use crate::note::NoteModel;

	#[test]
	fn complete_without_optional_files() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("Test.model");
		fs::create_dir(&model_path).unwrap();
		// Create style.css and an .hbs file so we get valid completion
		fs::write(model_path.join("style.css"), "body { color: red; }").unwrap();
		fs::write(model_path.join("Front+front.hbs"), "{{Front}}").unwrap();

		let partial = NoteModel { name: Some("Test".into()), ..NoteModel::default() };
		let complete = partial.complete(&model_path).unwrap();
		assert_eq!(complete.name, "Test");
		assert_eq!(complete.css, "body { color: red; }");
	}

	#[test]
	fn complete_loads_name_from_directory() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("MyModel.model");
		fs::create_dir(&model_path).unwrap();
		fs::write(model_path.join("style.css"), "").unwrap();
		fs::write(model_path.join("Q+front.hbs"), "{{Q}}").unwrap();

		let partial = NoteModel::default();
		let complete = partial.complete(&model_path).unwrap();
		assert_eq!(complete.name, "MyModel");
	}

	#[test]
	fn complete_loads_style_css() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("M.model");
		fs::create_dir(&model_path).unwrap();
		fs::write(model_path.join("style.css"), ".card { font-size: 14px; }").unwrap();
		fs::write(model_path.join("F+front.hbs"), "{{F}}").unwrap();

		let partial = NoteModel::default();
		let complete = partial.complete(&model_path).unwrap();
		assert_eq!(complete.css, ".card { font-size: 14px; }");
	}

	#[test]
	fn complete_loads_tex_files() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("M.model");
		fs::create_dir(&model_path).unwrap();
		fs::write(model_path.join("style.css"), "").unwrap();
		fs::write(model_path.join("F+front.hbs"), "{{F}}").unwrap();
		fs::write(model_path.join("pre.tex"), "\\prelude").unwrap();
		fs::write(model_path.join("post.tex"), "\\postlude").unwrap();

		let partial = NoteModel::default();
		let complete = partial.complete(&model_path).unwrap();
		assert_eq!(complete.latex_pre, Some("\\prelude".into()));
		assert_eq!(complete.latex_post, Some("\\postlude".into()));
	}

	#[test]
	fn complete_without_style_css_uses_empty_string() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("M.model");
		fs::create_dir(&model_path).unwrap();
		fs::write(model_path.join("F+front.hbs"), "{{F}}").unwrap();

		let partial = NoteModel::default();
		let complete = partial.complete(&model_path).unwrap();
		assert_eq!(complete.css, "");
	}

	#[test]
	fn complete_loads_templates() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("M.model");
		fs::create_dir(&model_path).unwrap();
		fs::write(model_path.join("style.css"), "").unwrap();
		fs::write(model_path.join("Card1+front.hbs"), "{{Front}}").unwrap();
		fs::write(model_path.join("Card1+back.hbs"), "{{Back}}").unwrap();

		let partial = NoteModel::default();
		let complete = partial.complete(&model_path).unwrap();
		assert_eq!(complete.templates.len(), 1);
		assert_eq!(complete.templates[0].name, "Card1");
		assert_eq!(complete.templates[0].question_format, "{{Front}}");
		assert_eq!(complete.templates[0].answer_format, "{{Back}}");
	}

	#[test]
	fn complete_loads_browser_templates() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("M.model");
		fs::create_dir(&model_path).unwrap();
		fs::write(model_path.join("style.css"), "").unwrap();
		fs::write(model_path.join("C+front.hbs"), "Q").unwrap();
		fs::write(model_path.join("C+front.browser.hbs"), "Q_browser").unwrap();
		fs::write(model_path.join("C+back.hbs"), "A").unwrap();
		fs::write(model_path.join("C+back.browser.hbs"), "A_browser").unwrap();

		let partial = NoteModel::default();
		let complete = partial.complete(&model_path).unwrap();
		let tmpl = &complete.templates[0];
		assert_eq!(tmpl.question_format, "Q");
		assert_eq!(tmpl.browser_question_format, "Q_browser");
		assert_eq!(tmpl.answer_format, "A");
		assert_eq!(tmpl.browser_answer_format, "A_browser");
	}

	#[test]
	fn complete_rejects_invalid_template_filename() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("M.model");
		fs::create_dir(&model_path).unwrap();
		fs::write(model_path.join("style.css"), "").unwrap();
		fs::write(model_path.join("invalid.hbs"), "{{F}}").unwrap();

		let partial = NoteModel::default();
		let result = partial.complete(&model_path);
		assert!(result.is_err());
	}

	#[test]
	fn complete_preserves_partial_fields() {
		let dir = tempfile::tempdir().unwrap();
		let model_path = dir.path().join("M.model");
		fs::create_dir(&model_path).unwrap();
		fs::write(model_path.join("style.css"), "").unwrap();
		fs::write(model_path.join("F+front.hbs"), "{{F}}").unwrap();

		let partial_id;
		let partial_schema;
		let complete = {
			let partial = NoteModel { name: Some("CustomName".into()), ..NoteModel::default() };
			partial_id = partial.id;
			partial_schema = partial.schema_version.clone();
			partial.complete(&model_path).unwrap()
		};
		assert_eq!(complete.id, partial_id);
		assert_eq!(complete.schema_version, partial_schema);
	}
}
