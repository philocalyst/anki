use gix::bstr::ByteSlice;
use tracing::{debug, instrument, warn};
use uuid::Uuid;

/// Creates the main UUID based on the author of the initial commit and the
/// time
#[instrument]
pub fn create_host_uuid(author: String, time: i64) -> Uuid {
	debug!("Creating host UUID for author: {}, time: {}", author, time);

	// Note: This is fragile and will break under rebase conditions
	// This is inherent to the design for deterministic generation
	let namespace = format!("{}{}", author, time);
	Uuid::new_v5(&Uuid::NAMESPACE_DNS, namespace.as_bytes())
}

/// Generate a UUID for a specific note based on its content
#[instrument(skip(content))]
pub fn generate_note_uuid(host_uuid: &Uuid, content: &str) -> Uuid {
	Uuid::new_v5(host_uuid, content.as_bytes())
}
