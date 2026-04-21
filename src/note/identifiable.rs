use uuid::Uuid;

use crate::note::Identified;

// Extension trait to add .identified() method
pub trait Identifiable: Sized {
	fn identified(self, id: Uuid) -> Identified<Self> { Identified { id, inner: self } }

	fn with_new_id(self) -> Identified<Self> { Identified { id: Uuid::new_v4(), inner: self } }
}
