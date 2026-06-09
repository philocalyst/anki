use std::collections::HashSet;

use crate::{error::DeckError, note::Note};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Transforms<'borrow, 'content> {
	Additions(Vec<(usize, &'borrow Note<'content>)>),
	Deletions(Vec<usize>),
	Modifications(Vec<(usize, &'borrow Note<'content>)>),
	Reorders(HashSet<(usize, usize)>),
}

fn how_it_grew<'borrow, 'content>(
	before: &[Note],
	after: &'borrow [Note<'content>],
) -> (usize, Vec<(usize, &'borrow Note<'content>)>) {
	// Deck grew - find all additions by walking both decks
	let mut additions = Vec::new();
	let mut deck_1_idx = 0;
	let mut deck_2_idx = 0;

	while deck_2_idx < after.len() {
		if deck_1_idx < before.len() && before[deck_1_idx] == after[deck_2_idx] {
			// Cards match, advance both pointers
			deck_1_idx += 1;
			deck_2_idx += 1;
		} else {
			// Card at deck_2_idx is new - record the addition
			additions.push((deck_2_idx, &after[deck_2_idx]));
			deck_2_idx += 1;
		}
	}

	return (deck_1_idx, additions);
}

/// Determines the kinds of changes that have occured between two decks. The
/// returned vector is compromised of just one ChangeType. Errors are returned
/// when the algorithim detects more than one kind of change.
pub fn determine_changes<'borrow, 'content>(
	before: &[Note],
	after: &'borrow [Note<'content>],
	// Transforms are relevant only to the new deck
) -> Result<Option<Transforms<'borrow, 'content>>, DeckError> {
	// Early return if decks are identical - no changes needed
	if before == after {
		return Ok(None);
	}

	// Case 1: Different lengths - either all additions or all deletions
	// We can't mix these types because indices would become inconsistent
	if before.len() != after.len() {
		if after.len() > before.len() {
			// TODO: Cleaner refactor... Sort of frustrating to pass it up as a tuple here.
			let (final_intial_index, additions) = how_it_grew(before, after);

			if final_intial_index < before.len() {
				return Err(DeckError::MixedChanges);
			}

			return Ok(Some(Transforms::Additions(additions)));
		} else {
			// Deck shrank - find all deletions by walking both decks
			let mut deletions = Vec::new();
			let mut deck_1_idx = 0;
			let mut deck_2_idx = 0;

			while deck_1_idx < before.len() {
				if deck_2_idx < after.len() && before[deck_1_idx] == after[deck_2_idx] {
					// Cards match, advance both pointers
					deck_1_idx += 1;
					deck_2_idx += 1;
				} else {
					// Card at deck_1_idx was deleted - record the deletion
					deletions.push(deck_1_idx);
					deck_1_idx += 1;
				}
			}

			if deck_2_idx < after.len() {
				return Err(DeckError::MixedChanges);
			}

			// IMPORTANT: Deletions must be applied in reverse order to maintain
			// index consistency. When you delete at index 0, everything shifts down,
			// so we need to delete from the end first.
			deletions.reverse();
			return Ok(Some(Transforms::Deletions(deletions)));
		}
	}

	// Case 2: Same length - could be reordering or modifications
	// Check if it's a reorder by comparing sorted versions
	let mut sorted_1 = before.to_vec();
	let mut sorted_2 = after.to_vec();
	sorted_1.sort();
	sorted_2.sort();

	if sorted_1 == sorted_2 {
		// Same cards, different order - this is a reordering
		// Find all positions where cards differ
		let mut reorderings = HashSet::new();
		for ((idx1, card1), (_, card2)) in before.iter().enumerate().zip(after.iter().enumerate()) {
			if *card1 != *card2
				&& let Some(idx2) = after.iter().position(|cur| cur == card1)
			{
				// Track where each card moved from -> to
				let swap = if idx1 < idx2 { (idx1, idx2) } else { (idx2, idx1) };
				reorderings.insert(swap);
			}
		}
		Ok(Some(Transforms::Reorders(reorderings)))
	} else {
		// Different cards at same positions - these are modifications
		// BUT check if any card moved - if so, it's a mixed change
		let mut modifications = Vec::new();
		for (index, (card1, card2)) in before.iter().zip(after.iter()).enumerate() {
			if card1 != card2 {
				// If either card exists elsewhere in the other deck, it's a reorder too
				if after.iter().any(|c| c == card1) || before.iter().any(|c| c == card2) {
					return Err(DeckError::MixedChanges);
				}
				modifications.push((index, card2));
			}
		}
		Ok(Some(Transforms::Modifications(modifications)))
	}
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;

	use uuid::Uuid;

	use super::determine_changes;
	use crate::note::{Note, NoteField, NoteModel, TextElement};

	fn note(front: &str, back: &str) -> Note<'static> {
		let mut model = NoteModel::default();
		model.id = Uuid::nil();
		Note {
			fields: vec![
				NoteField { name: "Front".into(), content: vec![TextElement::Text(front.into())] },
				NoteField { name: "Back".into(), content: vec![TextElement::Text(back.into())] },
			],
			model: Cow::Owned(model),
			tags: Vec::new(),
		}
	}

	#[test]
	fn identical_decks_returns_none() {
		let a = vec![note("Q1", "A1"), note("Q2", "A2")];
		let result = determine_changes(&a, &a).unwrap();
		assert_eq!(result, None);
	}

	#[test]
	fn additions_when_deck_grows() {
		let old = vec![note("Q1", "A1")];
		let new = vec![note("Q1", "A1"), note("Q2", "A2")];
		let result = determine_changes(&old, &new).unwrap();
		assert_eq!(result, Some(super::Transforms::Additions(vec![(1, &new[1])])));
	}

	#[test]
	fn additions_at_start() {
		let old = vec![note("Q1", "A1")];
		let new = vec![note("Q0", "A0"), note("Q1", "A1")];
		let result = determine_changes(&old, &new).unwrap();
		assert_eq!(result, Some(super::Transforms::Additions(vec![(0, &new[0])])));
	}

	#[test]
	fn additions_in_middle() {
		let old = vec![note("Q1", "A1"), note("Q3", "A3")];
		let new = vec![note("Q1", "A1"), note("Q2", "A2"), note("Q3", "A3")];
		let result = determine_changes(&old, &new).unwrap();
		assert_eq!(result, Some(super::Transforms::Additions(vec![(1, &new[1])])));
	}

	#[test]
	fn deletions_when_deck_shrinks() {
		let old = vec![note("Q1", "A1"), note("Q2", "A2")];
		let new = vec![note("Q1", "A1")];
		let result = determine_changes(&old, &new).unwrap();
		assert_eq!(result, Some(super::Transforms::Deletions(vec![1])));
	}

	#[test]
	fn deletions_at_start() {
		let old = vec![note("Q0", "A0"), note("Q1", "A1")];
		let new = vec![note("Q1", "A1")];
		let result = determine_changes(&old, &new).unwrap();
		assert_eq!(result, Some(super::Transforms::Deletions(vec![0])));
	}

	#[test]
	fn deletions_in_middle() {
		let old = vec![note("Q1", "A1"), note("Q2", "A2"), note("Q3", "A3")];
		let new = vec![note("Q1", "A1"), note("Q3", "A3")];
		let result = determine_changes(&old, &new).unwrap();
		assert_eq!(result, Some(super::Transforms::Deletions(vec![1])));
	}

	#[test]
	fn reorder_when_same_cards_different_order() {
		let n1 = note("Q1", "A1");
		let n2 = note("Q2", "A2");
		let a = vec![n1.clone(), n2.clone()];
		let b = vec![n2, n1];
		let result = determine_changes(&a, &b).unwrap();
		let mut expected = std::collections::HashSet::new();
		expected.insert((0, 1));
		assert_eq!(result, Some(super::Transforms::Reorders(expected)));
	}

	#[test]
	fn reorder_of_three() {
		let n1 = note("Q1", "A1");
		let n2 = note("Q2", "A2");
		let n3 = note("Q3", "A3");
		let a = vec![n1.clone(), n2.clone(), n3.clone()];
		let b = vec![n3.clone(), n1.clone(), n2.clone()];
		let result = determine_changes(&a, &b).unwrap();
		let mut expected = std::collections::HashSet::new();
		expected.insert((0, 1));
		expected.insert((0, 2));
		expected.insert((1, 2));
		assert_eq!(result, Some(super::Transforms::Reorders(expected)));
	}

	#[test]
	fn modifications_when_same_length_different_content() {
		let a = vec![note("Q1", "A1")];
		let b = vec![note("Q1", "A1_changed")];
		let result = determine_changes(&a, &b).unwrap();
		assert_eq!(result, Some(super::Transforms::Modifications(vec![(0, &b[0])])));
	}

	#[test]
	fn modifications_multiple() {
		let a = vec![note("Q1", "A1"), note("Q2", "A2")];
		let b = vec![note("Q1_x", "A1_x"), note("Q2", "A2_y")];
		let result = determine_changes(&a, &b).unwrap();
		assert_eq!(result, Some(super::Transforms::Modifications(vec![(0, &b[0]), (1, &b[1])])));
	}

	#[test]
	fn empty_decks_identical() {
		let empty: Vec<Note<'static>> = Vec::new();
		let result = determine_changes(&empty, &empty).unwrap();
		assert_eq!(result, None);
	}

	#[test]
	fn empty_to_non_empty_is_addition() {
		let empty: Vec<Note<'static>> = Vec::new();
		let new = vec![note("Q1", "A1")];
		let result = determine_changes(&empty, &new).unwrap();
		assert_eq!(result, Some(super::Transforms::Additions(vec![(0, &new[0])])));
	}

	#[test]
	fn non_empty_to_empty_is_deletion() {
		let old = vec![note("Q1", "A1")];
		let empty: Vec<Note<'static>> = Vec::new();
		let result = determine_changes(&old, &empty).unwrap();
		assert_eq!(result, Some(super::Transforms::Deletions(vec![0])));
	}

	#[test]
	fn error_on_mixed_changes() {
		// Case 1: Same length, mix of reorder and modification
		let old = vec![note("Q1", "A1"), note("Q2", "A2")];
		let new = vec![note("Q2", "A2"), note("Q3", "A3")];
		assert!(determine_changes(&old, &new).is_err());

		// Case 2: Different length, mix of addition and deletion
		let old = vec![note("Q1", "A1"), note("Q2", "A2")];
		let new = vec![note("Q2", "A2"), note("Q3", "A3"), note("Q4", "A4")];
		assert!(determine_changes(&old, &new).is_err());
	}

	#[test]
	fn addition_and_identical_prefix() {
		let old = vec![note("Q1", "A1"), note("Q2", "A2")];
		let new = vec![note("Q1", "A1"), note("Q2", "A2"), note("Q3", "A3"), note("Q4", "A4")];
		let result = determine_changes(&old, &new).unwrap();
		assert_eq!(result, Some(super::Transforms::Additions(vec![(2, &new[2]), (3, &new[3])])));
	}
}
