use std::{borrow::Cow, fmt::Debug, hash::Hash, path::PathBuf};

use evalexpr::Node;
use semver::Version;
use serde::Deserialize;
use uuid::Uuid;

use crate::note::{identifiable::Identifiable, model_config::{Defaults, Template}};

// Wrapper that adds an ID to any type
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Identified<T> {
	pub id:    Uuid,
	pub inner: T,
}

#[derive(Debug, PartialOrd, Ord, Clone, Eq, PartialEq)]
pub struct Note<'a> {
	pub fields: Vec<NoteField<'a>>,
	pub model:  Cow<'a, NoteModel>,
	pub tags:   Vec<String>,
}

// All notes can be identified
impl Identifiable for Note<'_> {}

#[derive(Debug, PartialOrd, Ord, Default, Eq, Clone, PartialEq)]
pub struct NoteField<'content> {
	pub name:    String,
	pub content: Vec<TextElement<'content>>,
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
	type Css = String;
	type Latex = Option<String>;
	type Name = Option<String>;
	type Templates = Vec<Template>;
}

impl ModelStage for Complete {
	type Css = String;
	type Latex = Option<String>;
	type Name = String;
	type Templates = Vec<Template>;
}

#[derive(Debug, Eq, PartialOrd, Ord, Hash, Deserialize, Clone, PartialEq)]
#[serde(
	bound = "Stage::Name: Deserialize<'de>, Stage::Templates: Deserialize<'de>, Stage::Css: Deserialize<'de>, Stage::Latex: Deserialize<'de>"
)]
pub struct NoteModel<Stage: ModelStage = Complete> {
	/// Filled in through the deck folder name
	#[serde(skip)]
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
	pub latex_pre:  Stage::Latex,
	#[serde(skip)]
	pub latex_post: Stage::Latex,

	// The field to sort around
	pub sort_field: Option<String>,
	pub tags:       Option<Vec<String>>,

	// The required fields are determined at runtime, this String holds a boolean expression that
	// affirms this.
	pub required: Node,
}

impl Default for NoteModel<Complete> {
	fn default() -> Self {
		Self {
			name:           String::from("Default Model"),
			id:             Uuid::new_v4(),
			templates:      Vec::new(),
			// semver::Version doesn't have Default. 0.1.0 is the standard starting point.
			schema_version: Version::new(0, 1, 0),
			defaults:       None,
			css:            String::new(),
			fields:         Vec::new(),
			latex_pre:      None,
			latex_post:     None,
			sort_field:     None,
			tags:           None,
			// evalexpr::Node doesn't have Default.
			// Parsing "true" ensures validation passes if no constraints are set.
			required:       evalexpr::build_operator_tree("true").unwrap(),
		}
	}
}

impl Default for NoteModel<Partial> {
	fn default() -> Self {
		Self {
			name:           Some(String::from("Default Model")),
			id:             Uuid::new_v4(),
			templates:      Vec::new(),
			schema_version: Version::new(0, 1, 0),
			defaults:       None,
			css:            String::new(),
			fields:         Vec::new(),
			latex_pre:      None,
			latex_post:     None,
			sort_field:     None,
			tags:           None,
			// evalexpr::Node doesn't have Default.
			// Parsing "true" ensures validation passes if no constraints are set.
			required:       evalexpr::build_operator_tree("true").unwrap(),
		}
	}
}

type Djot<'content> = Vec<jotdown::Event<'content>>;

/// Extract plain text from a Djot event stream
pub fn djot_to_string(events: &[jotdown::Event<'_>]) -> String {
	let mut s = String::new();
	for event in events {
		if let jotdown::Event::Str(text) = event {
			s.push_str(text);
		}
	}
	s
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Cloze<'content> {
	pub id:     u32,
	pub answer: Djot<'content>,
	pub hint:   Option<String>,
}

impl Ord for Cloze<'_> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.id.cmp(&other.id) }
}

impl PartialOrd for Cloze<'_> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

#[derive(Debug, PartialOrd, Ord, Eq, Clone, PartialEq)]
pub enum TextElement<'content> {
	Text(String),
	Cloze(Cloze<'content>),
}

#[derive(Deserialize, Ord, PartialOrd, Eq, Hash, Clone, PartialEq, Debug)]
pub struct Field {
	pub name:             String,
	pub sticky:           Option<bool>,
	pub associated_media: Option<Vec<PathBuf>>,
}
