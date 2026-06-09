use uuid::Uuid;

use crate::note::Identified;

// Extension trait to add .identified() method
pub trait Identifiable: Sized {
	fn identified(self, id: Uuid) -> Identified<Self> { Identified { id, inner: self } }

	fn with_new_id(self) -> Identified<Self> { Identified { id: Uuid::new_v4(), inner: self } }
}

#[cfg(test)]
mod tests {
	use uuid::Uuid;

	use super::Identifiable;

	#[derive(Debug, Clone, PartialEq, Eq)]
	struct TestItem(i32);

	impl Identifiable for TestItem {}

	#[test]
	fn identified_sets_id_and_inner() {
		let item = TestItem(42);
		let identified = item.identified(Uuid::from_u128(1));
		assert_eq!(identified.id, Uuid::from_u128(1));
		assert_eq!(identified.inner.0, 42);
	}

	#[test]
	fn identified_round_trips_id() {
		let uuid = Uuid::new_v4();
		let item = TestItem(99);
		let identified = item.identified(uuid);
		assert_eq!(identified.id, uuid);
	}

	#[test]
	fn with_new_id_generates_id() {
		let item = TestItem(7);
		let identified = item.with_new_id();
		assert_eq!(identified.inner.0, 7);
	}

	#[test]
	fn with_new_id_generates_unique_ids() {
		let a = TestItem(1).with_new_id();
		let b = TestItem(2).with_new_id();
		assert_ne!(a.id, b.id);
	}

	#[test]
	fn identified_uses_provided_id() {
		let uuid = Uuid::from_u128(0);
		let item = TestItem(0);
		let identified = item.identified(uuid);
		assert_eq!(identified.id, uuid);
	}

	#[test]
	fn identified_for_note() {
		use std::borrow::Cow;

		use crate::note::{Note, NoteModel};
		let note =
			Note { fields: Vec::new(), model: Cow::Owned(NoteModel::default()), tags: Vec::new() };
		let identified = note.identified(Uuid::from_u128(5));
		assert_eq!(identified.id, Uuid::from_u128(5));
	}
}
