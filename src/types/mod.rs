use gix::object::tree::Entry;

use crate::error::DeckError;

pub mod config;
pub mod crowd_anki_config;
pub mod crowd_anki_models;
pub mod deck;
pub mod note;
pub mod note_methods;
pub mod parser;

/// A reference to an entry that is validated as a Blob
#[derive(Debug)]
pub struct BEntry<'a, 'repo>(&'a Entry<'repo>);

impl<'a, 'repo> TryFrom<&'a Entry<'repo>> for BEntry<'a, 'repo> {
	type Error = DeckError;

	fn try_from(entry: &'a Entry<'repo>) -> Result<Self, Self::Error> {
		if !entry.mode().is_blob() {
			return Err(DeckError::InvalidEntry);
		}
		Ok(BEntry(entry))
	}
}

impl<'a, 'repo> BEntry<'a, 'repo> {
	pub fn new(entry: &'a Entry<'repo>) -> Result<Self, DeckError> { Self::try_from(entry) }

	/// Access the underlying entry
	pub fn entry(&self) -> &Entry<'repo> { self.0 }
}
