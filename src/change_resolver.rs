//! This file defines the resolution algorithim, which, given a commit object,
//! traces its history up to the present commit on the present branch, all the
//! while comparing iterations of the object over time in an attempt to preserve
//! the identifers made at each notes first declaration, enabling stable
//! representations through deletions, reorders, additions, and modifications
//! within data models that need to track state (Like Anki); Provided that the
//! user doesn't attempt more than one change operation at a time (Following
//! typical Git commit standards)

use std::borrow::Cow;

use uuid::Uuid;

use crate::{change_router::Transforms::{self, Additions, Deletions, Modifications, Reorders}, types::{note::{Identified, Note}, note_methods::Identifiable}, uuid_generator};

/// This function takes a set of transformations, in order from earliest to
/// latest, and applies them to the original notes within a deck. It is tracking
/// the state of the list over time, and returning its stable representation.
pub fn resolve_changes<'a, 'b>(
	transformations: &Transforms<'a>,
	substrate: &mut Vec<Identified<Note<'b>>>,
	host_uuid: Uuid,
) {
	match transformations {
		Additions(additions) => {
			for (idx, new_note) in additions {
				let base_uuid =
					uuid_generator::generate_note_uuid(&host_uuid, &new_note.to_content_string());
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
