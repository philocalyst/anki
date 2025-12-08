use serde::{Deserialize, Serialize};

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
	pub new:       Option<NewConfig>,
	pub rev:       Option<RevConfig>,
	pub lapse:     Option<LapseConfig>,

	// These keys are usually lowercase in Anki anyway, but camelCase covers strictness
	pub autoplay:        Option<bool>,
	pub replayq:         Option<bool>,
	pub timer:           Option<i32>,
	pub another_retreat: Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NewConfig {
	pub delays:         Vec<i32>,
	pub ints:           Vec<i32>,
	pub initial_factor: Option<i32>,
	pub per_day:        Option<i32>,
	pub order:          Option<i32>,
	pub bury:           Option<bool>,
	pub separate:       Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RevConfig {
	pub per_day:     Option<i32>,
	pub ease4:       Option<f32>,
	pub ivl_fct:     Option<f32>,
	pub fuzz:        Option<f32>,
	pub hard_factor: Option<f32>,
	pub max_ivl:     Option<i32>,
	pub min_space:   Option<i32>,
	pub bury:        Option<bool>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LapseConfig {
	pub delays:       Vec<i32>,
	pub mult:         f32,
	pub min_int:      Option<i32>,
	pub leech_action: Option<i32>,
	pub leech_fails:  Option<i32>,
}
