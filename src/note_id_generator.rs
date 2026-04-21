use gix::{Commit, Repository, object::tree::Entry};
use uuid::Uuid;

use crate::{error::DeckError, note::{Note, NoteModel}};

pub trait NoteIdGenerator {
	fn generate_note_ids_for_revision(
		&self,
		models: &[NoteModel],
		backing_vcs: &Repository,
		target: (Entry, Commit),
	) -> Result<Vec<Uuid>, DeckError>;

	fn generate_note_id_for_added_note(&self, note: &Note) -> Uuid;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GitNoteIdGenerator;
