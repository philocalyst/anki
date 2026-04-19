use tracing::{debug, instrument, warn};
use uuid::Uuid;

#[derive(Default)]
pub struct HostUuid(Uuid);

/// Creates the main UUID based on the author of the initial commit and the
/// time
#[instrument]
pub fn create_host_uuid(author: String, time: i64) -> HostUuid {
	debug!("Creating host UUID for author: {}, time: {}", author, time);

	// Note: This is fragile and will break under rebase conditions
	// This is inherent to the design for deterministic generation
	let namespace = format!("{}{}", author, time);
	HostUuid(Uuid::new_v5(&Uuid::NAMESPACE_DNS, namespace.as_bytes()))
}

/// Generate a UUID for a specific note based on its content
pub fn generate_note_uuid(uuid: &HostUuid, content: &str) -> Uuid {
	Uuid::new_v5(&uuid.0, content.as_bytes())
}

pub fn generate_core_identifier(commit_id: i64, comitter: &str, other: &str) -> Uuid {
	let sample = format!("{commit_id}{comitter}{other}");

	Uuid::new_v5(&Uuid::NAMESPACE_DNS, sample.as_bytes())
}
