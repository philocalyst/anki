use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum NoteModelType {
    Standard,
    Cloze,
}

impl<'de> Deserialize<'de> for NoteModelType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
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
        S: serde::Serializer,
    {
        let v = match self {
            NoteModelType::Standard => 0,
            NoteModelType::Cloze => 1,
        };
        serializer.serialize_i32(v)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[serde(tag = "__type__")]
pub enum CrowdAnkiEntity {
    Deck(Deck),
    Note(Note),
    NoteModel(NoteModel),
    DeckConfig(DeckConfig),
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Deck {
    pub name: String,
    pub id: i64,
    pub uuid: String,

    pub crowdanki_uuid: String,

    pub deck_config_uuid: String,
    pub desc: String,

    #[serde(rename = "dyn")]
    pub is_dynamic: i32,

    #[serde(rename = "extendNew")]
    pub extend_new: i32,

    #[serde(rename = "extendRev")]
    pub extend_rev: i32,

    pub note_models: Vec<NoteModel>,
    pub deck_configurations: Vec<DeckConfig>,
    pub notes: Vec<Note>,
    pub children: Vec<Deck>,
    pub media_files: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct NoteModel {
    pub id: i64,
    pub uuid: String,

    #[serde(rename = "crowdanki_uuid")]
    pub crowdanki_uuid: String,

    pub name: String,

    #[serde(rename = "type")]
    pub kind: NoteModelType,

    pub flds: Vec<Field>,
    pub tmpls: Vec<Template>,
    pub css: String,
    pub did: Option<String>,

    #[serde(rename = "latexPre")]
    pub latex_pre: Option<String>,

    #[serde(rename = "latexPost")]
    pub latex_post: Option<String>,

    pub req: Option<Vec<(i32, String, Vec<i32>)>>,
    pub sortf: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub vers: Option<Vec<String>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ord: i32,
    pub sticky: bool,
    pub rtl: bool,
    pub font: String,
    pub size: i32,
    pub media: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub ord: i32,
    pub qfmt: String,
    pub afmt: String,
    pub bafmt: Option<String>,
    pub bqfmt: Option<String>,
    pub did: Option<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Note {
    pub guid: String,
    pub uuid: Option<String>,

    #[serde(rename = "note_model_uuid")]
    pub note_model_uuid: String,

    pub fields: Vec<String>,
    pub tags: Vec<String>,
    pub flags: i32,

    #[serde(default)]
    pub newly_added: bool,

    #[serde(default)]
    pub data: Option<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct DeckConfig {
    pub uuid: String,

    #[serde(rename = "crowdanki_uuid")]
    pub crowdanki_uuid: String,

    pub name: String,
    pub id: i64,

    #[serde(rename = "dyn")]
    pub is_dynamic: bool,

    pub max_taken: Option<i32>,
    pub new: Option<NewConfig>,
    pub rev: Option<RevConfig>,
    pub lapse: Option<LapseConfig>,

    pub autoplay: Option<bool>,
    pub replayq: Option<bool>,
    pub timer: Option<i32>,
    pub another_retreat: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct NewConfig {
    pub delays: Vec<i32>,
    pub ints: Vec<i32>,
    pub initial_factor: i32,
    pub per_day: i32,
    pub order: Option<i32>,
    pub bury: Option<bool>,
    pub separate: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct RevConfig {
    pub per_day: i32,
    pub ease4: f32,
    pub ivl_fct: f32,
    pub fuzz: Option<f32>,
    pub hard_factor: Option<f32>,
    pub max_ivl: Option<i32>,
    pub min_space: Option<i32>,
    pub bury: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct LapseConfig {
    pub delays: Vec<i32>,
    pub mult: f32,
    pub min_int: i32,
    pub leech_action: Option<i32>,
    pub leech_fails: Option<i32>,
}
