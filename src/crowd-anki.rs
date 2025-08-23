use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum NoteModelType {
    Standard,
    Cloze,
}

impl<'de> serde::Deserialize<'de> for NoteModelType {
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

impl serde::Serialize for NoteModelType {
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
    pub deck_config_uuid: String,

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
    pub name: String,
    pub r#type: NoteModelType, // 0 = standard, 1 = cloze
    pub flds: Vec<Field>,
    pub tmpls: Vec<Template>,
    pub css: String,
    pub did: Option<String>, // deck uuid
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
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Note {
    pub guid: String,
    pub uuid: String,
    pub note_model_uuid: String,
    pub fields: Vec<String>,
    pub tags: Vec<String>,
    pub flags: i32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct DeckConfig {
    pub uuid: String,
    pub name: String,
    pub id: i64,

    pub maxTaken: Option<i32>,
    pub new: Option<NewConfig>,
    pub rev: Option<RevConfig>,
    pub lapse: Option<LapseConfig>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct NewConfig {
    pub delays: Vec<i32>,
    pub ints: Vec<i32>,
    pub initialFactor: i32,
    pub perDay: i32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct RevConfig {
    pub perDay: i32,
    pub ease4: f32,
    pub ivlFct: f32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct LapseConfig {
    pub delays: Vec<i32>,
    pub mult: f32,
    pub minInt: i32,
}
