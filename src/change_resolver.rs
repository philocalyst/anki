//! This file defines the resolution algorithim, which, given a commit object,
//! traces its history up to the present commit on the present branch, all the
//! while comparing iterations of the object over time in an attempt to preserve
//! the identifers made at each notes first declaration, enabling stable
//! representations through deletions, reorders, additions, and modifications
//! within data models that need to track state (Like Anki); Provided that the
//! user doesn't attempt more than one change operation at a time (Following
//! typical Git commit standards)

use std::borrow::Cow;

use crate::{change_router::Transforms::{self, Additions, Deletions, Modifications, Reorders}, note::{Identified, Note}, note_id_generator::NoteIdGenerator};

/// This function takes a set of transformations, in order from earliest to
/// latest, and applies them to the original notes within a deck. It is tracking
/// the state of the list over time, and returning its stable representation.
pub fn resolve_changes<'borrow, 'content>(
	transformations: &Transforms<'borrow, 'content>,
	substrate: &mut Vec<Identified<Note<'content>>>,
	note_id_generator: &impl NoteIdGenerator,
) {
	match transformations {
		Additions(additions) => {
			for (idx, new_note) in additions {
				let base_uuid = note_id_generator.generate_note_id_for_added_note(new_note);
				substrate.insert(*idx, Identified {
					id:    base_uuid,
					inner: Note {
						fields: new_note.fields.clone(),
						model:  Cow::Owned(new_note.model.clone().into_owned()),
						tags:   new_note.tags.clone(),
					},
				});
			}
		}
		Deletions(deletions) => {
			// Deletions are reversed during change vector creation
			for idx in deletions {
				substrate.remove(*idx);
			}
		}
		Modifications(modifications) => {
			for (idx, modified_note) in modifications {
				let existing_id = substrate[*idx].id;
				substrate[*idx] = Identified {
					id:    existing_id,
					inner: Note {
						fields: modified_note.fields.clone(),
						model:  Cow::Owned(modified_note.model.clone().into_owned()),
						tags:   modified_note.tags.clone(),
					},
				};
			}
		}
		Reorders(mappings) => {
			for (from, to) in mappings {
				substrate.swap(*from, *to);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;

	use uuid::Uuid;

	use super::resolve_changes;
	use crate::{change_router::Transforms, note::{Identified, Note, NoteField, NoteModel, TextElement}, note_id_generator::GitNoteIdGenerator};

	fn text_field(name: &str, value: &str) -> NoteField<'static> {
		NoteField { name: name.into(), content: vec![TextElement::Text(value.into())] }
	}

	fn make_note(front: &str, back: &str) -> Note<'static> {
		Note {
			fields: vec![text_field("Front", front), text_field("Back", back)],
			model:  Cow::Owned(NoteModel::default()),
			tags:   Vec::new(),
		}
	}

	fn make_identified(id: u128, front: &str, back: &str) -> Identified<Note<'static>> {
		Identified { id: Uuid::from_u128(id), inner: make_note(front, back) }
	}

	#[test]
	fn additions_inserts_at_correct_index() {
		let mut cards = vec![make_identified(1, "Q1", "A1"), make_identified(3, "Q3", "A3")];
		let new_note = make_note("Q2", "A2");
		let additions = Transforms::Additions(vec![(1, &new_note)]);
		resolve_changes(&additions, &mut cards, &GitNoteIdGenerator);
		assert_eq!(cards.len(), 3);
		assert_eq!(cards[0].id, Uuid::from_u128(1));
		assert_eq!(cards[2].id, Uuid::from_u128(3));
	}

	#[test]
	fn additions_multiple() {
		let mut cards: Vec<Identified<Note<'static>>> = Vec::new();
		let n1 = make_note("Q1", "A1");
		let n2 = make_note("Q2", "A2");
		let additions = Transforms::Additions(vec![(0, &n1), (1, &n2)]);
		resolve_changes(&additions, &mut cards, &GitNoteIdGenerator);
		assert_eq!(cards.len(), 2);
	}

	#[test]
	fn deletions_removes_at_index() {
		let mut cards = vec![
			make_identified(1, "Q1", "A1"),
			make_identified(2, "Q2", "A2"),
			make_identified(3, "Q3", "A3"),
		];
		let deletions = Transforms::Deletions(vec![1]);
		resolve_changes(&deletions, &mut cards, &GitNoteIdGenerator);
		assert_eq!(cards.len(), 2);
		assert_eq!(cards[0].id, Uuid::from_u128(1));
		assert_eq!(cards[1].id, Uuid::from_u128(3));
	}

	#[test]
	fn deletions_multiple_reversed() {
		let mut cards = vec![
			make_identified(1, "Q1", "A1"),
			make_identified(2, "Q2", "A2"),
			make_identified(3, "Q3", "A3"),
		];
		let deletions = Transforms::Deletions(vec![2, 0]);
		resolve_changes(&deletions, &mut cards, &GitNoteIdGenerator);
		assert_eq!(cards.len(), 1);
		assert_eq!(cards[0].id, Uuid::from_u128(2));
	}

	#[test]
	fn modifications_preserve_id() {
		let mut cards = vec![make_identified(42, "Q1", "A1")];
		let modified = make_note("Q1_changed", "A1_changed");
		let modifications = Transforms::Modifications(vec![(0, &modified)]);
		resolve_changes(&modifications, &mut cards, &GitNoteIdGenerator);
		assert_eq!(cards.len(), 1);
		assert_eq!(cards[0].id, Uuid::from_u128(42));
	}

	#[test]
	fn modifications_multiple() {
		let mut cards = vec![make_identified(1, "Q1", "A1"), make_identified(2, "Q2", "A2")];
		let m1 = make_note("Q1x", "A1x");
		let m2 = make_note("Q2x", "A2x");
		let modifications = Transforms::Modifications(vec![(0, &m1), (1, &m2)]);
		resolve_changes(&modifications, &mut cards, &GitNoteIdGenerator);
		assert_eq!(cards[0].id, Uuid::from_u128(1));
		assert_eq!(cards[1].id, Uuid::from_u128(2));
	}

	#[test]
	fn reorders_swaps_positions() {
		let mut cards = vec![make_identified(1, "Q1", "A1"), make_identified(2, "Q2", "A2")];
		let reorders = Transforms::Reorders([(0, 1)].into());
		resolve_changes(&reorders, &mut cards, &GitNoteIdGenerator);
		assert_eq!(cards[0].id, Uuid::from_u128(2));
		assert_eq!(cards[1].id, Uuid::from_u128(1));
	}
}
