use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Template {
	pub name: String,

	#[serde(skip)]
	pub order: i32,

	#[serde(skip)]
	pub question_format: String,
	#[serde(skip)]
	pub answer_format:   String,

	#[serde(skip)]
	pub browser_question_format: String,
	#[serde(skip)]
	pub browser_answer_format:   String,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
pub struct Defaults {
	pub font: String,
	pub size: u32,
	pub rtl:  bool,
}
