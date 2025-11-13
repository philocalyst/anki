use core::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum DeckError {
	NoDeckFound,
	ModelNotFound(String),
	FileNotInHistory(String),
	InvalidEntry,
}

impl fmt::Display for DeckError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NoDeckFound => write!(f, "No .deck directory found"),
			Self::ModelNotFound(name) => write!(f, "Model '{}' not found", name),
			Self::FileNotInHistory(path) => write!(f, "File '{}' not found in history", path),
			Self::InvalidEntry => write!(f, "Invalid tree entry"),
		}
	}
}

impl Error for DeckError {}
