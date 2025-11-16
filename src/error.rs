use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeckError {
	#[error("No .deck directory found")]
	NoDeckFound,

	#[error("Model '{0}' not found")]
	ModelNotFound(String),

	#[error("File '{0}' not found in history")]
	FileNotInHistory(String),

	#[error("Invalid tree entry")]
	InvalidEntry,
}
