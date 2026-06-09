use uuid::Uuid;

pub const FLASH_DECK_UUID_KEY: &str = "flash_uuid";

pub fn make_flash_tag(deck_uuid: &Uuid, note_uuid: &Uuid) -> String {
	format!("flash::{}::{}", deck_uuid, note_uuid)
}

pub fn parse_note_uuid_from_tag(tag: &str) -> Option<Uuid> {
	let parts: Vec<&str> = tag.split("::").collect();
	if parts.len() == 3 && parts[0] == "flash" { Uuid::parse_str(parts[2]).ok() } else { None }
}

pub fn parse_deck_uuid_from_tag(tag: &str) -> Option<Uuid> {
	let parts: Vec<&str> = tag.split("::").collect();
	if parts.len() >= 2 && parts[0] == "flash" { Uuid::parse_str(parts[1]).ok() } else { None }
}

pub fn make_model_uuid_comment(uuid: &Uuid) -> String { format!("/* flash-uuid: {} */", uuid) }

pub fn parse_model_uuid_from_css(css: &str) -> Option<Uuid> {
	let prefix = "/* flash-uuid: ";
	let suffix = " */";
	css.find(prefix).and_then(|start| {
		let rest = &css[start + prefix.len()..];
		rest.find(suffix).and_then(|end| Uuid::parse_str(&rest[..end]).ok())
	})
}

pub fn flash_tag_prefix(deck_uuid: &Uuid) -> String { format!("tag:flash::{}::", deck_uuid) }

pub fn flash_tag_wildcard(deck_uuid: &Uuid) -> String { format!("tag:flash::{}::*", deck_uuid) }

#[cfg(test)]
mod tests {
	use uuid::Uuid;

	use super::{flash_tag_prefix, flash_tag_wildcard, make_flash_tag, make_model_uuid_comment, parse_deck_uuid_from_tag, parse_model_uuid_from_css, parse_note_uuid_from_tag};

	#[test]
	fn make_flash_tag_correct_format() {
		let deck = Uuid::from_u128(1);
		let note = Uuid::from_u128(42);
		let tag = make_flash_tag(&deck, &note);
		assert_eq!(
			tag,
			"flash::00000000-0000-0000-0000-000000000001::00000000-0000-0000-0000-00000000002a"
		);
		assert!(tag.starts_with("flash::"));
		assert!(tag.len() > 20);
	}

	#[test]
	fn parse_note_uuid_round_trips() {
		let deck = Uuid::new_v4();
		let note = Uuid::new_v4();
		let tag = make_flash_tag(&deck, &note);
		let parsed = parse_note_uuid_from_tag(&tag);
		assert_eq!(parsed, Some(note));
	}

	#[test]
	fn parse_note_uuid_from_random_tag_returns_none() {
		assert_eq!(parse_note_uuid_from_tag("random-tag"), None);
		assert_eq!(parse_note_uuid_from_tag("flash::invalid"), None);
		assert_eq!(parse_note_uuid_from_tag("flash::a::b::c"), None);
		assert_eq!(parse_note_uuid_from_tag(""), None);
	}

	#[test]
	fn parse_deck_uuid_from_flash_tag() {
		let deck = Uuid::new_v4();
		let note = Uuid::new_v4();
		let tag = make_flash_tag(&deck, &note);
		let parsed = parse_deck_uuid_from_tag(&tag);
		assert_eq!(parsed, Some(deck));
	}

	#[test]
	fn parse_deck_uuid_malformed_returns_none() {
		assert_eq!(parse_deck_uuid_from_tag("flash"), None);
		assert_eq!(parse_deck_uuid_from_tag("not-flash::uuid"), None);
		assert_eq!(parse_deck_uuid_from_tag(""), None);
	}

	#[test]
	fn make_model_uuid_comment_contains_uuid() {
		let uuid = Uuid::new_v4();
		let comment = make_model_uuid_comment(&uuid);
		assert!(comment.contains(&uuid.to_string()));
		assert!(comment.starts_with("/*"));
		assert!(comment.ends_with("*/"));
	}

	#[test]
	fn parse_model_uuid_round_trips() {
		let uuid = Uuid::new_v4();
		let css = make_model_uuid_comment(&uuid);
		let parsed = parse_model_uuid_from_css(&css);
		assert_eq!(parsed, Some(uuid));
	}

	#[test]
	fn parse_model_uuid_from_css_with_surrounding_content() {
		let uuid = Uuid::new_v4();
		let css = format!(".card {{\n  /* flash-uuid: {} */\n  font-family: sans-serif;\n}}\n", uuid);
		let parsed = parse_model_uuid_from_css(&css);
		assert_eq!(parsed, Some(uuid));
	}

	#[test]
	fn parse_model_uuid_no_comment_returns_none() {
		assert_eq!(parse_model_uuid_from_css(".card { color: red; }"), None);
		assert_eq!(parse_model_uuid_from_css(""), None);
		assert_eq!(parse_model_uuid_from_css("/* flash-uuid: not-a-uuid */"), None);
	}

	#[test]
	fn flash_tag_prefix_format() {
		let deck = Uuid::from_u128(99);
		assert_eq!(flash_tag_prefix(&deck), "tag:flash::00000000-0000-0000-0000-000000000063::");
	}

	#[test]
	fn flash_tag_wildcard_format() {
		let deck = Uuid::from_u128(99);
		assert_eq!(flash_tag_wildcard(&deck), "tag:flash::00000000-0000-0000-0000-000000000063::*");
	}

	#[test]
	fn uuid_uniqueness_across_tags() {
		let deck1 = Uuid::new_v4();
		let deck2 = Uuid::new_v4();
		let note1 = Uuid::new_v4();
		let note2 = Uuid::new_v4();

		let tag1 = make_flash_tag(&deck1, &note1);
		let tag2 = make_flash_tag(&deck2, &note2);

		assert_ne!(tag1, tag2);
		assert_eq!(parse_note_uuid_from_tag(&tag1), Some(note1));
		assert_eq!(parse_note_uuid_from_tag(&tag2), Some(note2));
		assert_eq!(parse_deck_uuid_from_tag(&tag1), Some(deck1));
		assert_eq!(parse_deck_uuid_from_tag(&tag2), Some(deck2));
	}

	#[test]
	fn parse_model_uuid_invalid_uuid_in_comment() {
		let css = "/* flash-uuid: not-a-real-uuid-here */";
		assert_eq!(parse_model_uuid_from_css(css), None);
	}

	#[test]
	fn parse_note_uuid_from_tag_exact_three_parts() {
		let parts = "flash::a::b";
		assert_eq!(parse_note_uuid_from_tag(parts), None);
	}

	#[test]
	fn parse_note_uuid_from_tag_not_flash_prefix() {
		let tag = "other::00000000-0000-0000-0000-000000000001::00000000-0000-0000-0000-00000000002a";
		assert_eq!(parse_note_uuid_from_tag(tag), None);
	}

	#[test]
	fn parse_deck_uuid_from_tag_requires_at_least_two_parts() {
		assert_eq!(parse_deck_uuid_from_tag("flash"), None);
	}

	#[test]
	fn parse_deck_uuid_from_tag_not_flash_prefix() {
		assert_eq!(parse_deck_uuid_from_tag("other::uuid"), None);
	}

	#[test]
	fn parse_model_uuid_from_css_empty_string() {
		assert_eq!(parse_model_uuid_from_css(""), None);
	}

	#[test]
	fn parse_model_uuid_from_css_no_comment() {
		assert_eq!(parse_model_uuid_from_css(".card { color: red; }"), None);
	}

	#[test]
	fn parse_model_uuid_from_css_with_trailing_content() {
		let css = "/* flash-uuid: 550e8400-e29b-41d4-a716-446655440000 */\n.card { }";
		let parsed = parse_model_uuid_from_css(css);
		assert!(parsed.is_some());
	}

	#[test]
	fn parse_model_uuid_from_css_incomplete_suffix() {
		let css = "/* flash-uuid: 550e8400-e29b-41d4-a716-446655440000";
		assert_eq!(parse_model_uuid_from_css(css), None);
	}

	#[test]
	fn flash_tag_prefix_ends_with_double_colon() {
		let deck = Uuid::new_v4();
		let prefix = flash_tag_prefix(&deck);
		assert!(prefix.ends_with("::"));
	}

	#[test]
	fn flash_tag_wildcard_ends_with_star() {
		let deck = Uuid::new_v4();
		let wildcard = flash_tag_wildcard(&deck);
		assert!(wildcard.ends_with("*"));
	}
}
