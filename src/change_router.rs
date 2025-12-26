use std::collections::HashSet;

use crate::{error::DeckError, types::note::Note};

#[derive(Debug, Clone)]
pub enum Transforms<'a> {
	Additions(Vec<(usize, &'a Note<'a>)>),
	Deletions(Vec<usize>),
	Modifications(Vec<(usize, &'a Note<'a>)>),
	Reorders(HashSet<(usize, usize)>),
}

/// Determines the kinds of changes that have occured between two decks. The
/// returned vector is compromised of just one ChangeType. Errors are returned
/// when the algorithim detects more than one kind of change.
pub fn determine_changes<'b>(
	deck_1: &[Note], // The old deck is MORE disposable
	deck_2: &'b [Note],
	// Transforms are relevant only to the new deck
) -> Result<Option<Transforms<'b>>, DeckError> {
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
