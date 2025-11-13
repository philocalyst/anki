use std::error::Error;

use crate::types::{deck::Deck, note::Note};

#[derive(Debug)]
pub(crate) enum ChangeType {
	Addition(usize),
	Deletion(usize),
	Modification(usize),
	Reordering(usize),
}

/// Determines the kind of change occured between two decks. A None value is
/// return when no change has occured.
pub(crate) fn determine_change(
	deck_1: &Vec<Note>,
	deck_2: &Vec<Note>,
) -> Result<Option<ChangeType>, Box<dyn Error>> {
	if deck_1 == deck_2 {
		return Ok(None);
	}

	if deck_1.len() != deck_2.len() {
		for (index, (card1, card2)) in deck_1.iter().zip(deck_2.iter()).enumerate() {
			if card1 != card2 {
				// We're checking if this card is now out of order or no longer exists
				if deck_2.iter().find(|n| *n == card1).is_some() {
					// It does exist, this means that there was an addition
					return Ok(Some(ChangeType::Addition(index)));
				} else {
					// This is the case where it no longer exists
					return Ok(Some(ChangeType::Deletion(index)));
				}
			}
		}

		// Handle trailing additions/deletions
		if deck_1.len() < deck_2.len() {
			return Ok(Some(ChangeType::Addition(deck_1.len())));
		} else {
			return Ok(Some(ChangeType::Deletion(deck_2.len())));
		}
	} else {
		// Same length - could be modification or reordering
		// Check if it's a reorder (same cards, different order)
		let mut sorted_1 = deck_1.clone();
		let mut sorted_2 = deck_2.clone();
		sorted_1.sort();
		sorted_2.sort();

		if sorted_1 == sorted_2 {
			// It's a reorder - find first difference
			for (index, (card1, card2)) in deck_1.iter().zip(deck_2.iter()).enumerate() {
				if card1 != card2 {
					// TODO: Fix this to return a precise mapping of old to new, index isn't enough.
					return Ok(Some(ChangeType::Reordering(index)));
				}
			}
		} else {
			// It's a modification - find first difference among the cards
			for (index, (card1, card2)) in deck_1.iter().zip(deck_2.iter()).enumerate() {
				if card1 != card2 {
					return Ok(Some(ChangeType::Modification(index)));
				}
			}
		}
	}

	Ok(None)
}
