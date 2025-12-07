use chumsky::Parser;
use gix::{Commit, Repository, Tree, object::tree::Entry};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{error::DeckError, parse::flash, types::{crowd_anki_config::DeckConfig, note::{Identified, Note, NoteModel}}, uuid_generator};

pub struct Deck<'a> {
	pub models:        Vec<NoteModel>,
	pub backing_vcs:   Repository,
	pub cards:         Vec<Identified<Note<'a>>>,
	pub configuration: DeckConfig,
}
