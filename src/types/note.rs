use std::{borrow::Cow, fmt::Debug, hash::Hash, path::PathBuf};

use evalexpr::Node;
use semver::Version;
use serde::Deserialize;
use uuid::Uuid;

use crate::types::{
	config::{Defaults, Template},
	note_methods::Identifiable,
};

// Wrapper that adds an ID to any type
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Identified<T> {
	pub id: Uuid,
	pub inner: T,
}

#[derive(Debug, PartialOrd, Ord, Clone, Eq, PartialEq)]
pub struct Note<'a> {
	pub fields: Vec<NoteField>,
	pub model: Cow<'a, NoteModel>,
	pub tags: Vec<String>,
}

// All notes can be identified
impl Identifiable for Note<'_> {}

#[derive(Debug, PartialOrd, Ord, Default, Eq, Clone, PartialEq)]
pub struct NoteField {
	pub name: String,
	pub content: Vec<TextElement>,
}

pub trait ModelStage: Default + Clone + Debug + PartialEq + Eq + PartialOrd + Ord + Hash {
	type Name: Clone + Debug + PartialEq + Eq + PartialOrd + Ord + Hash + Default;
	type Css: Clone + Debug + PartialEq + Eq + PartialOrd + Ord + Hash + Default;
	type Templates: Clone + Debug + PartialEq + Eq + PartialOrd + Ord + Hash + Default;
	type Latex: Clone + Debug + PartialEq + Eq + PartialOrd + Ord + Hash + Default;
}

#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Partial;

#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Complete;

impl ModelStage for Partial {
	type Name = Option<String>;
	type Css = String;
	type Templates = Vec<Template>;
	type Latex = Option<String>;
}

impl ModelStage for Complete {
	type Name = String;
	type Css = String;
	type Templates = Vec<Template>;
	type Latex = Option<String>;
}

#[derive(Debug, Eq, PartialOrd, Ord, Hash, Deserialize, Clone, PartialEq)]
#[serde(
	bound = "Stage::Name: Deserialize<'de>, Stage::Templates: Deserialize<'de>, Stage::Css: Deserialize<'de>, Stage::Latex: Deserialize<'de>"
)]
pub struct NoteModel<Stage: ModelStage = Complete> {
	/// Filled in through the deck folder name
	pub name: Stage::Name,

	pub id: Uuid,

	// The available templates
	pub templates: Stage::Templates,

	// The version of the schema that we're on
	pub schema_version: Version,

	// The default field configuration
	pub defaults: Option<Defaults>,

	// Anything with serde skip means I don't want it to be possible to be included in the TOML
	// representation
	#[serde(skip)]
	pub css: Stage::Css,

	pub fields: Vec<Field>,

	#[serde(skip)]
	pub latex_pre: Stage::Latex,
	#[serde(skip)]
	pub latex_post: Stage::Latex,

	// The field to sort around
	pub sort_field: Option<String>,
	pub tags: Option<Vec<String>>,

	// The required fields are determined at runtime, this String holds a boolean expression that
	// affirms this.
	pub required: Node,
}

impl Default for NoteModel<Complete> {
	fn default() -> Self {
		Self {
			name: String::from("Default Model"),
			id: Uuid::new_v4(),
			templates: Vec::new(),
			// semver::Version doesn't have Default. 0.1.0 is the standard starting point.
			schema_version: Version::new(0, 1, 0),
			defaults: None,
			css: String::new(),
			fields: Vec::new(),
			latex_pre: None,
			latex_post: None,
			sort_field: None,
			tags: None,
			// evalexpr::Node doesn't have Default.
			// Parsing "true" ensures validation passes if no constraints are set.
			required: evalexpr::build_operator_tree("true").unwrap(),
		}
	}
}

impl Default for NoteModel<Partial> {
	fn default() -> Self {
		Self {
			name: Some(String::from("Default Model")),
			id: Uuid::new_v4(),
			templates: Vec::new(),
			schema_version: Version::new(0, 1, 0),
			defaults: None,
			css: String::new(),
			fields: Vec::new(),
			latex_pre: None,
			latex_post: None,
			sort_field: None,
			tags: None,
			// evalexpr::Node doesn't have Default.
			// Parsing "true" ensures validation passes if no constraints are set.
			required: evalexpr::build_operator_tree("true").unwrap(),
		}
	}
}

#[derive(Debug, Ord, PartialOrd, Eq, Clone, PartialEq)]
pub struct Cloze {
	pub id: u32,
	pub answer: String,
	pub hint: Option<String>,
}

#[derive(Debug, PartialOrd, Ord, Eq, Clone, PartialEq)]
pub enum TextElement {
	Text(String),
	Cloze(Cloze),
}

#[derive(Deserialize, Ord, PartialOrd, Eq, Hash, Clone, PartialEq, Debug)]
pub struct Field {
	pub name: String,
	pub sticky: Option<bool>,
	pub associated_media: Option<Vec<PathBuf>>,
}
