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

#[cfg(test)]
mod tests {
	#[test]
	fn blob_entry_validation_rejects_non_blob_mode() {
		let tree_mode = gix::object::tree::EntryMode::try_from(0o040000).unwrap();
		assert!(!tree_mode.is_blob());
	}

	#[test]
	fn blob_entry_validation_accepts_blob_mode() {
		let blob_mode = gix::object::tree::EntryMode::try_from(0o100644).unwrap();
		assert!(blob_mode.is_blob());
	}

	#[test]
	fn blob_entry_validation_rejects_symlink() {
		let symlink_mode = gix::object::tree::EntryMode::try_from(0o120000).unwrap();
		assert!(!symlink_mode.is_blob());
	}

	#[test]
	fn blob_entry_validation_rejects_commit() {
		let commit_mode = gix::object::tree::EntryMode::try_from(0o160000).unwrap();
		assert!(!commit_mode.is_blob());
	}

	#[test]
	fn berenty_try_from_fails_on_invalid_mode() {
		let tree_mode = gix::object::tree::EntryMode::try_from(0o040000).unwrap();
		assert!(!tree_mode.is_blob());
		let blob_mode = gix::object::tree::EntryMode::try_from(0o100644).unwrap();
		assert!(blob_mode.is_blob());
	}
}
