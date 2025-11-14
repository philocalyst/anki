//! This file defines the resolution algorithim, which, given a commit object,
//! traces its history up to the present commit on the present branch, all the
//! while comparing iterations of the object over time in an attempt to preserve
//! the identifers made at each notes first declaration, enabling stable
//! representations through deletions, reorders, additions, and modifications
//! within data models that need to track state (Like Anki); Provided that the
//! user doesn't attempt more than one change operation at a time (Following
//! typical Git commit standards)

use chumsky::input::Input;
use uuid::Uuid;

use crate::{change_router::ChangeType, types::note::Note, uuid_generator::UuidGenerator};

pub(crate) struct IdentifiedNote<'a> {
	pub id:   Uuid,
	pub note: &'a Note<'a>,
}

impl<'a> IdentifiedNote<'a> {
	pub fn new(note: &'a Note, id: Uuid) -> Self { IdentifiedNote { id, note } }
}

/// This function takes a set of transformations, in order from earliest to
/// latest, and applies them to the original notes within a deck. It is tracking
/// the state of the list over time, and returning its stable representation.
pub(crate) fn resolve_uuids<'a>(
	transformations: &'a [ChangeType],
	original: Vec<Note<'a>>,
) -> Vec<IdentifiedNote<'a>> {
	let mut result: Vec<IdentifiedNote> = Vec::with_capacity(original.len());

	for transformation in transformations {
		match transformation {
			ChangeType::Addition((idx, new_note)) => {
				// TODO: Consume a host UUID
				let base_uuid =
					UuidGenerator::generate_note_uuid(&Uuid::default(), &new_note.to_content_string());

				result.insert(*idx, IdentifiedNote::new(&new_note, base_uuid));
			}
			ChangeType::Deletion(idx) => {
				result.remove(*idx);
			}
			ChangeType::Modification((idx, modified_note)) => {
				result[*idx] = IdentifiedNote::new(modified_note, Uuid::default())
			}
			ChangeType::Reordering(_) => todo!(),
		}
	}

	result
}
