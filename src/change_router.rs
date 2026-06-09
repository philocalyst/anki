use std::collections::HashSet;

use crate::{error::DeckError, note::Note};

#[derive(Debug, Clone)]
pub enum Transforms<'borrow, 'content> {
	Additions(Vec<(usize, &'borrow Note<'content>)>),
	Deletions(Vec<usize>),
	Modifications(Vec<(usize, &'borrow Note<'content>)>),
	Reorders(HashSet<(usize, usize)>),
}

/// Determines the kinds of changes that have occured between two decks. The
/// returned vector is compromised of just one ChangeType. Errors are returned
/// when the algorithim detects more than one kind of change.
pub fn determine_changes<'borrow, 'content>(
	deck_1: &[Note],
	deck_2: &'borrow [Note<'content>],
	// Transforms are relevant only to the new deck
) -> Result<Option<Transforms<'borrow, 'content>>, DeckError> {
	// Early return if decks are identical - no changes needed
	if deck_1 == deck_2 {
		return Ok(None);
	}

	// Case 1: Different lengths - either all additions or all deletions
	// We can't mix these types because indices would become inconsistent
	if deck_1.len() != deck_2.len() {
		if deck_2.len() > deck_1.len() {
			// Deck grew - find all additions by walking both decks
			let mut additions = Vec::new();
			let mut deck_1_idx = 0;
			let mut deck_2_idx = 0;

			while deck_2_idx < deck_2.len() {
				if deck_1_idx < deck_1.len() && deck_1[deck_1_idx] == deck_2[deck_2_idx] {
					// Cards match, advance both pointers
					deck_1_idx += 1;
					deck_2_idx += 1;
				} else {
					// Card at deck_2_idx is new - record the addition
					additions.push((deck_2_idx, &deck_2[deck_2_idx]));
					deck_2_idx += 1;
				}
			}
			return Ok(Some(Transforms::Additions(additions)));
		} else {
			// Deck shrank - find all deletions by walking both decks
			let mut deletions = Vec::new();
			let mut deck_1_idx = 0;
			let mut deck_2_idx = 0;

			while deck_1_idx < deck_1.len() {
				if deck_2_idx < deck_2.len() && deck_1[deck_1_idx] == deck_2[deck_2_idx] {
					// Cards match, advance both pointers
					deck_1_idx += 1;
					deck_2_idx += 1;
				} else {
					// Card at deck_1_idx was deleted - record the deletion
					deletions.push(deck_1_idx);
					deck_1_idx += 1;
				}
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
	let mut sorted_1 = deck_1.to_vec();
	let mut sorted_2 = deck_2.to_vec();
	sorted_1.sort();
	sorted_2.sort();

	if sorted_1 == sorted_2 {
		// Same cards, different order - this is a reordering
		// Find all positions where cards differ
		let mut reorderings = HashSet::new();
		for ((idx1, card1), (_, card2)) in deck_1.iter().enumerate().zip(deck_2.iter().enumerate()) {
			if *card1 != *card2
				&& let Some(idx2) = deck_2.iter().position(|cur| cur == card1)
			{
				// Track where each card moved from -> to
				let swap = if idx1 < idx2 { (idx1, idx2) } else { (idx2, idx1) };
				reorderings.insert(swap);
			}
		}
		Ok(Some(Transforms::Reorders(reorderings)))
	} else {
		// Different cards at same positions - these are modifications
		// Find all positions where content changed
		let mut modifications = Vec::new();
		for (index, (card1, card2)) in deck_1.iter().zip(deck_2.iter()).enumerate() {
			if card1 != card2 {
				modifications.push((index, card2));
			}
		}
		Ok(Some(Transforms::Modifications(modifications)))
	}
}

#[cfg(test)]
mod tests {
	use std::borrow::Cow;

	use super::determine_changes;
	use crate::note::{Note, NoteField, NoteModel, TextElement};

	fn note(front: &str, back: &str) -> Note<'static> {
		Note {
			fields: vec![
				NoteField { name: "Front".into(), content: vec![TextElement::Text(front.into())] },
				NoteField { name: "Back".into(), content: vec![TextElement::Text(back.into())] },
			],
			model:  Cow::Owned(NoteModel::default()),
			tags:   Vec::new(),
		}
	}

	#[test]
	fn identical_decks_returns_none() {
		let a = vec![note("Q1", "A1"), note("Q2", "A2")];
		let result = determine_changes(&a, &a).unwrap();
		assert!(result.is_none());
	}

	#[test]
	fn additions_when_deck_grows() {
		let old = vec![note("Q1", "A1")];
		let new = vec![note("Q1", "A1"), note("Q2", "A2")];
		let result = determine_changes(&old, &new).unwrap();
		assert!(matches!(result, Some(super::Transforms::Additions(_))));
	}

	#[test]
	fn additions_at_start() {
		let old = vec![note("Q1", "A1")];
		let new = vec![note("Q0", "A0"), note("Q1", "A1")];
		let result = determine_changes(&old, &new).unwrap();
		assert!(matches!(result, Some(super::Transforms::Additions(_))));
	}

	#[test]
	fn additions_in_middle() {
		let old = vec![note("Q1", "A1"), note("Q3", "A3")];
		let new = vec![note("Q1", "A1"), note("Q2", "A2"), note("Q3", "A3")];
		let result = determine_changes(&old, &new).unwrap();
		assert!(matches!(result, Some(super::Transforms::Additions(_))));
	}

	#[test]
	fn deletions_when_deck_shrinks() {
		let old = vec![note("Q1", "A1"), note("Q2", "A2")];
		let new = vec![note("Q1", "A1")];
		let result = determine_changes(&old, &new).unwrap();
		assert!(matches!(result, Some(super::Transforms::Deletions(_))));
	}

	#[test]
	fn deletions_at_start() {
		let old = vec![note("Q0", "A0"), note("Q1", "A1")];
		let new = vec![note("Q1", "A1")];
		let result = determine_changes(&old, &new).unwrap();
		assert!(matches!(result, Some(super::Transforms::Deletions(_))));
	}

	#[test]
	fn deletions_in_middle() {
		let old = vec![note("Q1", "A1"), note("Q2", "A2"), note("Q3", "A3")];
		let new = vec![note("Q1", "A1"), note("Q3", "A3")];
		let result = determine_changes(&old, &new).unwrap();
		assert!(matches!(result, Some(super::Transforms::Deletions(_))));
	}

	#[test]
	fn reorder_when_same_cards_different_order() {
		let n1 = note("Q1", "A1");
		let n2 = note("Q2", "A2");
		let a = vec![n1.clone(), n2.clone()];
		let b = vec![n2, n1];
		let result = determine_changes(&a, &b).unwrap();
		assert!(matches!(result, Some(super::Transforms::Reorders(_))));
	}

	#[test]
	fn reorder_of_three() {
		let n1 = note("Q1", "A1");
		let n2 = note("Q2", "A2");
		let n3 = note("Q3", "A3");
		let a = vec![n1.clone(), n2.clone(), n3.clone()];
		let b = vec![n3, n1, n2];
		let result = determine_changes(&a, &b).unwrap();
		assert!(matches!(result, Some(super::Transforms::Reorders(_))));
	}

	#[test]
	fn modifications_when_same_length_different_content() {
		let a = vec![note("Q1", "A1")];
		let b = vec![note("Q1", "A1_changed")];
		let result = determine_changes(&a, &b).unwrap();
		assert!(matches!(result, Some(super::Transforms::Modifications(_))));
	}

	#[test]
	fn modifications_multiple() {
		let a = vec![note("Q1", "A1"), note("Q2", "A2")];
		let b = vec![note("Q1_x", "A1_x"), note("Q2", "A2_y")];
		let result = determine_changes(&a, &b).unwrap();
		assert!(matches!(result, Some(super::Transforms::Modifications(_))));
	}

	#[test]
	fn empty_decks_identical() {
		let empty: Vec<Note<'static>> = Vec::new();
		let result = determine_changes(&empty, &empty).unwrap();
		assert!(result.is_none());
	}

	#[test]
	fn empty_to_non_empty_is_addition() {
		let empty: Vec<Note<'static>> = Vec::new();
		let new = vec![note("Q1", "A1")];
		let result = determine_changes(&empty, &new).unwrap();
		assert!(matches!(result, Some(super::Transforms::Additions(_))));
	}

	#[test]
	fn non_empty_to_empty_is_deletion() {
		let old = vec![note("Q1", "A1")];
		let empty: Vec<Note<'static>> = Vec::new();
		let result = determine_changes(&old, &empty).unwrap();
		assert!(matches!(result, Some(super::Transforms::Deletions(_))));
	}

	#[test]
	fn addition_and_identical_prefix() {
		let old = vec![note("Q1", "A1"), note("Q2", "A2")];
		let new = vec![note("Q1", "A1"), note("Q2", "A2"), note("Q3", "A3"), note("Q4", "A4")];
		let result = determine_changes(&old, &new).unwrap();
		assert!(matches!(result, Some(super::Transforms::Additions(_))));
	}
}
