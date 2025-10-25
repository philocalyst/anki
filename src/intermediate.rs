pub struct NoteRecord<'a> {
	associated_note: &'a Note,
	uuid:            u128,
}

pub struct Lock {
	notes:   Vec<NoteRecord>,
	history: Vec<Operation>,
}

pub enum Operation {
	Added { note: &NoteRecord },
	Deleted { note: &NoteRecord },

	// To is the position in the notes list
	Moved { note: &NoteRecord, to: usize },
}
