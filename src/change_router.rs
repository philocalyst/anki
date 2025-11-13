use std::error::Error;

use crate::types::{deck::Deck, note::Note};

pub(crate) enum ChangeType {
	Addition,
	Deletion,
	Modification,
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
		for ((index1, card1), (index2, card2)) in
			deck_1.into_iter().enumerate().zip(deck_2.into_iter().enumerate())
		{
			// We've identified a divergence.
			if card1 != card2 {
				// We're checking if this card is now out of order or no longer exists
				if exists_in_deck(deck_2, card1) {
					// If it does exist, it means that there was an addition
					return Ok(Some(ChangeType::Addition));
				} else {
					// This is the case where it no longer exists
					return Ok(Some(ChangeType::Deletion));
				}
			}
		}
	}

	Ok(None)
}

fn exists_in_deck(deck: &Vec<Note>, target: &Note) -> bool {
	for note in deck {
		if note == target {
			return true;
		}
	}
	return false;
}
