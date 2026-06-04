use gix::object::tree::Entry;

use crate::error::DeckError;

/// A reference to an entry that is validated as a Blob
#[derive(Debug)]
pub struct BEntry<'entry, 'repo>(&'entry Entry<'repo>);

impl<'entry, 'repo> TryFrom<&'entry Entry<'repo>> for BEntry<'entry, 'repo> {
	type Error = DeckError;

	fn try_from(entry: &'entry Entry<'repo>) -> Result<Self, Self::Error> {
		if !entry.mode().is_blob() {
			return Err(DeckError::InvalidEntry);
		}
		Ok(BEntry(entry))
	}
}

impl<'entry, 'repo> BEntry<'entry, 'repo> {
	pub fn new(entry: &'entry Entry<'repo>) -> Result<Self, DeckError> { Self::try_from(entry) }

	/// Access the underlying entry
	pub fn entry(&self) -> &Entry<'repo> { self.0 }
}
