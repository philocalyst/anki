use gix::Repository;

use crate::types::{crowd_anki_config::DeckConfig, note::{Identified, Note, NoteModel}};

#[derive(Clone)]
pub struct Deck<'a> {
	pub models:        Vec<NoteModel>,
	pub backing_vcs:   Repository,
	pub cards:         Vec<Identified<Note<'a>>>,
	pub configuration: DeckConfig,
}
