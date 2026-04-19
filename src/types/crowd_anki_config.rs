use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Clone)]
pub enum ConfigType {
	#[default]
	DeckConfig,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct DeckConfig {
	// Note: Python uses UUID_FIELD_NAME. If that constant is "crowdanki_uuid", this works.
	#[serde(skip)]
	pub crowdanki_uuid: String,

	#[serde(rename = "type")]
	#[serde(skip)]
	#[serde(default)]
	pub kind: ConfigType,

	pub name: String,

	#[serde(rename = "dyn")]
	#[serde(skip)]
	pub is_dynamic: bool,

	// Anki key: "maxTaken"
	#[serde(rename = "camelCase")]
	pub max_taken: Option<i32>,
	pub new: Option<NewConfig>,
	pub rev: Option<RevConfig>,
	pub lapse: Option<LapseConfig>,

	// These keys are usually lowercase in Anki anyway, but camelCase covers strictness
	pub autoplay: Option<bool>,
	pub replayq: Option<bool>,
	pub timer: Option<i32>,
	pub another_retreat: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NewConfig {
	pub delays: Vec<i32>,
	pub ints: Vec<i32>,
	pub initial_factor: Option<i32>,
	pub per_day: Option<i32>,
	pub order: Option<i32>,
	pub bury: Option<bool>,
	pub separate: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RevConfig {
	pub per_day: Option<i32>,
	pub ease4: Option<f32>,
	pub ivl_fct: Option<f32>,
	pub fuzz: Option<f32>,
	pub hard_factor: Option<f32>,
	pub max_ivl: Option<i32>,
	pub min_space: Option<i32>,
	pub bury: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LapseConfig {
	pub delays: Vec<i32>,
	pub mult: f32,
	pub min_int: Option<i32>,
	pub leech_action: Option<i32>,
	pub leech_fails: Option<i32>,
}

impl DeckConfig {
	pub fn blank(uuid: Uuid) -> Self {
		Self {
			crowdanki_uuid: uuid.to_string(),
			kind: ConfigType::default(),
			name: "regex".to_string(),
			is_dynamic: false,
			max_taken: Some(100),
			new: Some(NewConfig::default()),
			rev: Some(RevConfig::default()),
			lapse: Some(LapseConfig::default()),
			autoplay: Some(true),
			replayq: Some(true),
			timer: Some(0),
			another_retreat: Some(false),
		}
	}
}

impl Default for NewConfig {
	fn default() -> Self {
		Self {
			delays: vec![1, 10],
			ints: vec![1, 4, 7],
			initial_factor: Some(2500),
			per_day: Some(20),
			order: Some(1),
			bury: Some(true),
			separate: Some(false),
		}
	}
}

impl Default for RevConfig {
	fn default() -> Self {
		Self {
			per_day: Some(200),
			ease4: Some(1.3),
			ivl_fct: Some(1.0),
			fuzz: Some(0.05),
			hard_factor: Some(1.2),
			max_ivl: Some(36500),
			min_space: Some(1),
			bury: Some(true),
		}
	}
}

impl Default for LapseConfig {
	fn default() -> Self {
		Self {
			delays: vec![10],
			mult: 0.0,
			min_int: Some(1),
			leech_action: Some(0),
			leech_fails: Some(8),
		}
	}
}
