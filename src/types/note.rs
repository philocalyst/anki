use std::path::PathBuf;

use evalexpr::Node;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::types::config::{Defaults, Template};

#[derive(Debug, Clone, PartialEq)]
pub struct Note<'a> {
	pub fields: Vec<NoteField>,
	pub model:  &'a NoteModel,
	pub tags:   Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NoteField {
	pub name:    String,
	pub content: Vec<TextElement>,
}

#[derive(Debug, Hash, Eq, Deserialize, Clone, PartialEq)]
pub struct NoteModel {
	pub name: String,

	// The available templates
	pub templates: Vec<Template>,

	// The version of the schema that we're on
	pub schema_version: Version,

	// The default field configuration
	pub defaults: Option<Defaults>,

	// Anything with serde skip means I don't want it to be possible to be included in the TOML
	// representation
	#[serde(skip)]
	pub css: String,

	pub fields: Vec<Field>,

	#[serde(skip)]
	pub latex_pre:  Option<String>,
	#[serde(skip)]
	pub latex_post: Option<String>,

	// The field to sort around
	pub sort_field: Option<String>,
	pub tags:       Option<Vec<String>>,
	// The required fields are determined at runtime, this String holds a boolean expression that
	// affirms this.
	// pub required: Node,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cloze {
	pub id:     u32,
	pub answer: String,
	pub hint:   Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextElement {
	Text(String),
	Cloze(Cloze),
}

#[derive(Deserialize, Eq, Hash, Clone, PartialEq, Debug)]
pub struct Field {
	pub name:             String,
	pub sticky:           Option<bool>,
	pub associated_media: Option<Vec<PathBuf>>,
}
