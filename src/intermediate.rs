pub struct NoteRecord<'a> {
    associated_note: &'a Note,
    uuid: u128,
}

pub struct Lock {
    notes: Vec<NoteRecord>,
    history: Vec<Operation>,
}

