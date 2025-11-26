use std::path::PathBuf;

use gix::diff::tree;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeckError {
	#[error("No .deck directory found in the current directory or any parent directories.")]
	NoDeckFound,

	#[error("Model '{0}' not found.")]
	ModelNotFound(String),

	#[error("File '{0}' not found in history.")]
	FileNotInHistory(String),

	#[error("Invalid tree entry.")]
	InvalidEntry,

	#[error("I/O error.")]
	Io(#[from] std::io::Error),

	#[error("TOML deserialization error.")]
	Toml(#[from] toml::de::Error),

	#[error("Git error: {0}")]
	Git(String),

	#[error("Invalid UTF-8 in file: {0:?}.")]
	InvalidUtf8(PathBuf),

	#[error("Template file '{0}' has an invalid format.")]
	InvalidTemplateFilename(String),

	#[error("Model config file not found: {0:?}")]
	ModelConfigNotFound(PathBuf),

	#[error("Template file not found: {0:?}")]
	TemplateNotFound(PathBuf),

	#[error("UUID generation error.")]
	Uuid(#[from] uuid::Error),

	#[error("Failed to resolve git reference.")]
	Reference(#[from] gix::reference::find::existing::Error),

	#[error("Failed to create git tree from entries.")]
	TreeFromEntries(#[from] tree::Error),

	#[error("Failed to write git object.")]
	WriteObject(#[from] gix::object::write::Error),

	#[error("Failed to traverse tree")]
	TreeWalk(#[from] gix::revision::walk::Error),

	#[error("Failed to commit changes to git.")]
	Commit(#[from] gix::object::commit::Error),

	#[error("Failed to parse deck: {0}")]
	Parse(String),

	#[error("Could not find existing git object: {0}")]
	ObjectFind(#[from] gix::object::find::existing::Error),

	#[error("Failed to traverse git commit history: {0}")]
	CommitTraverse(#[from] gix::traverse::commit::simple::Error),

	#[error("Failed to peel git head reference to an object: {0}")]
	HeadPeelToObject(#[from] gix::head::peel::to_object::Error),

	#[error("Failed to walk git revisions: {0}")]
	RevisionWalk(#[from] gix::revision::walk::iter::Error),

	#[error("Failed to find existing git object with conversion: {0}")]
	ObjectFindConvert(#[from] gix::object::find::existing::with_conversion::Error),
}
