//! This file defines the resolution algorithim, which, given a commit object,
//! traces its history up to the present commit on the present branch, all the
//! while comparing iterations of the object over time in an attempt to preserve
//! the identifers made at each notes first declaration, enabling stable
//! representations through deletions, reorders, additions, and modifications
//! within data models that need to track state (Like Anki); Provided that the
//! user doesn't attempt more than one change operation at a time (Following
//! typical Git commit standards)

use uuid::Uuid;

use crate::{change_router::ChangeType, types::note::ONote, uuid_generator};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct IdentifiedNote {
	pub id:   Uuid,
	pub note: ONote,
}

impl<'a> IdentifiedNote {
	pub fn new(note: ONote, id: Uuid) -> Self { IdentifiedNote { id, note } }
}

/// This function takes a set of transformations, in order from earliest to
/// latest, and applies them to the original notes within a deck. It is tracking
/// the state of the list over time, and returning its stable representation.
pub fn resolve_changes<'a>(
	transformations: &'a [ChangeType],
	substrate: &mut Vec<IdentifiedNote>,
	host_uuid: Uuid,
) {
	for transformation in transformations {
		match transformation {
			ChangeType::Addition((idx, new_note)) => {
				let base_uuid =
					uuid_generator::generate_note_uuid(&host_uuid, &new_note.to_content_string());

				substrate.insert(*idx, IdentifiedNote::new(new_note.to_owned().to_owned(), base_uuid));
			}
			ChangeType::Deletion(idx) => {
				// Deletions are reversed during change vector creation, so think of this as
				// operating backwards.
				substrate.remove(*idx);
			}
			ChangeType::Modification((idx, modified_note)) => {
				substrate[*idx] =
					IdentifiedNote::new(modified_note.to_owned().to_owned(), substrate[*idx].id)
			}
			ChangeType::Reordering(_) => todo!(),
		}
	}
}
