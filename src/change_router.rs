use std::error::Error;

use crate::types::deck::Deck;

pub(crate) enum ChangeType {
	Addition,
	Deletion,
	Modification,
}

/// Determines the kind of change occured between two decks. A None value is
/// return when no change has occured.
pub(crate) fn determine_change(
	deck_1: &Deck,
	deck_2: &Deck,
) -> Result<Option<ChangeType>, Box<dyn Error>> {
	Ok(None)
}
