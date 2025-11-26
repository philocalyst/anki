use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeckError<'a> {
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

    #[error("Failed to parse flashcard file: {0:?}.")]
    Parse(Vec<chumsky::error::Simple<'a, String>>),

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
    TreeFromEntries(#[from] gix::object::tree::Error),

    #[error("Failed to write git object.")]
    WriteObject(#[from] gix::object::write::Error),

    #[error("Failed to commit changes to git.")]
    Commit(#[from] gix::object::commit::Error),
}

impl<'a> From<gix::object::find::existing::Error> for DeckError<'a> {
    fn from(err: gix::object::find::existing::Error) -> Self {
        DeckError::Git(err.to_string())
    }
}

impl<'a> From<gix::traverse::commit::simple::Error> for DeckError<'a> {
    fn from(err: gix::traverse::commit::simple::Error) -> Self {
        DeckError::Git(err.to_string())
    }
}
