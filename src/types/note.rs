use std::{borrow::Cow, path::PathBuf};

use evalexpr::Node;
use semver::Version;
use serde::Deserialize;
use uuid::Uuid;

use crate::types::{config::{Defaults, Template}, note_methods::Identifiable};

// Wrapper that adds an ID to any type
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Identified<T> {
	pub id:    Uuid,
	pub inner: T,
}

#[derive(Debug, PartialOrd, Ord, Clone, Eq, PartialEq)]
pub struct Note<'a> {
	pub fields: Vec<NoteField>,
	pub model:  Cow<'a, NoteModel>,
	pub tags:   Vec<String>,
}

// All notes can be identified
impl Identifiable for Note<'_> {}

#[derive(Debug, PartialOrd, Ord, Default, Eq, Clone, PartialEq)]
pub struct NoteField {
	pub name:    String,
	pub content: Vec<TextElement>,
}

#[derive(Debug, Eq, PartialOrd, Ord, Hash, Deserialize, Clone, PartialEq)]
pub struct NoteModel {
	pub name: String,

	pub id: Uuid,

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
	pub required: Node,
}

#[derive(Debug, Ord, PartialOrd, Eq, Clone, PartialEq)]
pub struct Cloze {
	pub id:     u32,
	pub answer: String,
	pub hint:   Option<String>,
}

#[derive(Debug, PartialOrd, Ord, Eq, Clone, PartialEq)]
pub enum TextElement {
	Text(String),
	Cloze(Cloze),
}

#[derive(Deserialize, Ord, PartialOrd, Eq, Hash, Clone, PartialEq, Debug)]
pub struct Field {
	pub name:             String,
	pub sticky:           Option<bool>,
	pub associated_media: Option<Vec<PathBuf>>,
}
