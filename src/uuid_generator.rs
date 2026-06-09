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

#[cfg(test)]
mod tests {
	use super::{create_host_uuid, generate_core_identifier, generate_note_uuid};

	#[test]
	fn create_host_uuid_deterministic() {
		let a = create_host_uuid("alice".into(), 1000);
		let b = create_host_uuid("alice".into(), 1000);
		assert_eq!(a.0, b.0);
	}

	#[test]
	fn create_host_uuid_differs_for_different_inputs() {
		let a = create_host_uuid("alice".into(), 1000);
		let b = create_host_uuid("bob".into(), 1000);
		assert_ne!(a.0, b.0);
	}

	#[test]
	fn create_host_uuid_differs_for_different_times() {
		let a = create_host_uuid("alice".into(), 1000);
		let b = create_host_uuid("alice".into(), 2000);
		assert_ne!(a.0, b.0);
	}

	#[test]
	fn generate_note_uuid_deterministic() {
		let host = create_host_uuid("test".into(), 1);
		let a = generate_note_uuid(&host, "hello");
		let b = generate_note_uuid(&host, "hello");
		assert_eq!(a, b);
	}

	#[test]
	fn generate_note_uuid_differs_for_different_content() {
		let host = create_host_uuid("test".into(), 1);
		let a = generate_note_uuid(&host, "hello");
		let b = generate_note_uuid(&host, "world");
		assert_ne!(a, b);
	}

	#[test]
	fn generate_note_uuid_type_is_v5() {
		let host = create_host_uuid("test".into(), 1);
		let uuid = generate_note_uuid(&host, "content");
		assert_eq!(uuid.get_version(), Some(uuid::Version::Sha1));
	}

	#[test]
	fn generate_core_identifier_deterministic() {
		let a = generate_core_identifier(42, "alice", "abc123");
		let b = generate_core_identifier(42, "alice", "abc123");
		assert_eq!(a, b);
	}

	#[test]
	fn generate_core_identifier_differs_for_different_commit_id() {
		let a = generate_core_identifier(42, "alice", "abc");
		let b = generate_core_identifier(99, "alice", "abc");
		assert_ne!(a, b);
	}

	#[test]
	fn generate_core_identifier_differs_for_different_comitter() {
		let a = generate_core_identifier(42, "alice", "abc");
		let b = generate_core_identifier(42, "bob", "abc");
		assert_ne!(a, b);
	}

	#[test]
	fn generate_core_identifier_differs_for_different_other() {
		let a = generate_core_identifier(42, "alice", "abc");
		let b = generate_core_identifier(42, "alice", "xyz");
		assert_ne!(a, b);
	}
}
