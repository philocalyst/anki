use gix::Repository;

use crate::{crowd_anki::DeckConfig, note::{Identified, Note, NoteModel}};

#[derive(Clone)]
pub struct Deck<'model> {
	pub models:        Vec<NoteModel>,
	pub backing_vcs:   Repository,
	pub cards:         Vec<Identified<Note<'model>>>,
	pub configuration: DeckConfig,
}
