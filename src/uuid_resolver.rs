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

use crate::{change_router::ChangeType, types::note::Note};

pub(crate) struct IdentifiedNote<'a> {
	pub id:   Uuid,
	pub note: &'a Note<'a>,
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
			ChangeType::Addition(_) => todo!(),
			ChangeType::Deletion(_) => todo!(),
			ChangeType::Modification(_) => todo!(),
			ChangeType::Reordering(_) => todo!(),
		}
	}

	result
}
