use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::types::crowd_anki_config::DeckConfig;

fn serialize_option_string<S>(val: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	serializer.serialize_str(val.as_deref().unwrap_or(""))
}

fn serialize_option_i32<S>(val: &Option<i32>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	serializer.serialize_i32(val.unwrap_or(0))
}

fn serialize_option_complex<S>(
	val: &Option<Vec<(i32, String, Vec<i32>)>>,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	use serde::Serialize;
	let default = vec![(0, String::new(), vec![])];
	let data = val.as_ref().unwrap_or(&default);
	data.serialize(serializer)
}

#[derive(Debug, Clone)]
pub enum NoteModelType {
	Standard,
	Cloze,
}

impl<'de> Deserialize<'de> for NoteModelType {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let v = i32::deserialize(deserializer)?;
		match v {
			0 => Ok(NoteModelType::Standard),
			1 => Ok(NoteModelType::Cloze),
			_ => Err(serde::de::Error::custom(format!("invalid type: {}", v))),
		}
	}
}

impl Serialize for NoteModelType {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let v = match self {
			NoteModelType::Standard => 0,
			NoteModelType::Cloze => 1,
		};
		serializer.serialize_i32(v)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "__type__")]
pub enum CrowdAnkiEntity {
	Deck(Deck),
	Note(Note),
	NoteModel(NoteModel),
	DeckConfig(DeckConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
	pub name:             String,
	pub crowdanki_uuid:   String,
	pub deck_config_uuid: String,
	pub desc:             String,

	#[serde(rename = "dyn")]
	pub is_dynamic: i32,

	#[serde(rename = "extendNew")]
	pub extend_new: i32,

	#[serde(rename = "extendRev")]
	pub extend_rev: i32,

	pub note_models:         Vec<NoteModel>,
	pub deck_configurations: Vec<DeckConfig>,
	pub notes:               Vec<Note>,
	pub children:            Vec<Deck>,
	pub media_files:         Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteModel {
	pub crowdanki_uuid: String,
	pub name:           String,

	#[serde(rename = "type")]
	pub kind: NoteModelType,

	pub flds:  Vec<Field>,
	pub tmpls: Vec<Template>,
	pub css:   String,

	#[serde(default)]
	pub did: Option<i64>,

	#[serde(rename = "latexPre")]
	#[serde(serialize_with = "serialize_option_string")]
	pub latex_pre: Option<String>,

	#[serde(rename = "latexPost")]
	#[serde(serialize_with = "serialize_option_string")]
	pub latex_post: Option<String>,

	#[serde(serialize_with = "serialize_option_complex")]
	pub req: Option<Vec<(i32, String, Vec<i32>)>>,

	#[serde(serialize_with = "serialize_option_i32")]
	pub sortf: Option<i32>,

	pub tags: Option<Vec<String>>,
	pub vers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
	pub name:   String,
	pub ord:    i32,
	pub sticky: bool,
	pub rtl:    bool,
	pub font:   String,
	pub size:   i32,
	pub media:  Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
	pub name:  String,
	pub ord:   i32,
	pub qfmt:  String,
	pub afmt:  String,
	pub bafmt: Option<String>,
	pub bqfmt: Option<String>,

	#[serde(default)]
	pub did: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
	pub guid:            String,
	pub note_model_uuid: String,
	pub fields:          Vec<String>,
	pub tags:            Vec<String>,
	pub flags:           i32,

	#[serde(default)]
	#[serde(rename = "newlyAdded")]
	pub newly_added: bool,

	#[serde(default)]
	pub data: Option<String>,
}
