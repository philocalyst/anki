use std::error::Error;

use crate::types::note::Note;

#[derive(Debug)]
pub enum ChangeType<'a> {
	Addition((usize, &'a Note<'a>)),
	Deletion(usize),
	Modification((usize, &'a Note<'a>)),
	Reordering(usize),
}

/// Determines the kind of change occured between two decks. A None value is
/// return when no change has occured.
pub fn determine_changes<'a>(
	deck_1: &'a Vec<Note<'a>>,
	deck_2: &'a Vec<Note<'a>>,
) -> Result<Vec<ChangeType<'a>>, Box<dyn Error>> {
	// Early return if decks are identical - no changes needed
	if deck_1 == deck_2 {
		return Ok(vec![]);
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
					additions.push(ChangeType::Addition((deck_2_idx, &deck_2[deck_2_idx])));
					deck_2_idx += 1;
				}
			}
			return Ok(additions);
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
					deletions.push(ChangeType::Deletion(deck_1_idx));
					deck_1_idx += 1;
				}
			}
			// IMPORTANT: Deletions must be applied in reverse order to maintain
			// index consistency. When you delete at index 0, everything shifts down,
			// so we need to delete from the end first.
			deletions.reverse();
			return Ok(deletions);
		}
	}

	// Case 2: Same length - could be reordering or modifications
	// Check if it's a reorder by comparing sorted versions
	let mut sorted_1 = deck_1.clone();
	let mut sorted_2 = deck_2.clone();
	sorted_1.sort();
	sorted_2.sort();

	if sorted_1 == sorted_2 {
		// Same cards, different order - this is a reordering
		// Find all positions where cards differ
		let mut reorderings = Vec::new();
		for (index, (card1, card2)) in deck_1.iter().zip(deck_2.iter()).enumerate() {
			if card1 != card2 {
				// TODO: This index alone isn't enough info for reordering.
				// We need to track where each card moved from/to.
				reorderings.push(ChangeType::Reordering(index));
			}
		}
		return Ok(reorderings);
	} else {
		// Different cards at same positions - these are modifications
		// Find all positions where content changed
		let mut modifications = Vec::new();
		for (index, (card1, card2)) in deck_1.iter().zip(deck_2.iter()).enumerate() {
			if card1 != card2 {
				modifications.push(ChangeType::Modification((index, card2)));
			}
		}
		return Ok(modifications);
	}
}
