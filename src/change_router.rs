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

	// This is likely to be a deletion or modification, the job now is to determine
	// which one and where
	if deck_1.len() != deck_2.len() {
		for (index, (card1, card2)) in deck_1.into_iter().zip(deck_2.into_iter()).enumerate() {
			// We've identified a divergence.
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
	} else {
		// If the two decks have the same amount of cards, and yet are not equal,
		// this means a modification has occured
		for (index, (card1, card2)) in deck_1.into_iter().zip(deck_2.into_iter()).enumerate() {
			if card1 != card2 {
				return Ok(Some(ChangeType::Modification(index)));
			}
		}
	}

	Ok(None)
}
