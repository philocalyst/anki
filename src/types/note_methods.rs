use std::{error::Error, fs, path::Path};

use crate::types::{config::Template, note::NoteModel};

impl NoteModel {
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
					let tmpl = templates.iter_mut().find(|t: &&mut Template| t.name == template_name);

					let tmpl = if let Some(t) = tmpl {
						t
					} else {
						templates.push(Template {
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
