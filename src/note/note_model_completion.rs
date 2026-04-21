use std::{fs, path::Path};

use tracing::{debug, info};

use crate::{
	error::DeckError,
	note::{Complete, NoteModel, Partial},
};

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
						name: template_name.clone(),
						order: templates.len() as i32,
						question_format: String::new(),
						answer_format: String::new(),
						browser_question_format: String::new(),
						browser_answer_format: String::new(),
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
