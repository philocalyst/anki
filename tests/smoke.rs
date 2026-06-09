mod common;

use std::{error::Error, fs};
use serde_json::Value;
use common::*;

#[test]
fn cli_outputs_expected_shape() -> Result<(), Box<dyn Error>> {
    let data_home = prepare_data_home()?;
    let deck_root = tempfile::tempdir()?;
    let deck_repo = make_deck_repo(deck_root.path(), "grammar.deck", MINIMAL_DECK_CONTENT)?;
    let output_dir = tempfile::tempdir()?;
    let output_path = output_dir.path().join("out.json");

    let output = run_cli(&[deck_repo], &output_path, data_home.path())?;
    assert!(
        output.status.success(),
        "flash CLI failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;

    assert_eq!(json.len(), 1);
    assert_eq!(json[0]["name"], "regex");
    assert!(json[0]["flash_uuid"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
    assert_eq!(json[0]["note_count"].as_i64().unwrap_or(0), 2);
    assert_eq!(json[0]["model_count"].as_i64().unwrap_or(0), 1);
    assert_eq!(
        json[0]["card_ids"].as_array().map(|a| a.len()).unwrap_or(0),
        2,
        "Should have 2 card IDs for 2 notes"
    );

    Ok(())
}

#[test]
fn cli_outputs_include_note_uuids() -> Result<(), Box<dyn Error>> {
    let data_home = prepare_data_home()?;
    let deck_root = tempfile::tempdir()?;
    let deck_repo = make_deck_repo(deck_root.path(), "test.deck", MINIMAL_DECK_CONTENT)?;
    let output_dir = tempfile::tempdir()?;
    let output_path = output_dir.path().join("ids.json");

    let output = run_cli(&[deck_repo], &output_path, data_home.path())?;
    assert!(
        output.status.success(),
        "flash CLI failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
    let card_ids = json[0]["card_ids"].as_array().unwrap();

    assert_eq!(card_ids.len(), 2, "Should have 2 card IDs");
    for id in card_ids {
        let s = id.as_str().unwrap();
        assert_eq!(s.len(), 36, "Each card ID should be a UUID string");
        assert_eq!(s.chars().filter(|&c| c == '-').count(), 4, "UUID format check");
    }

    // UUIDs should all be different
    let unique: std::collections::HashSet<&str> =
        card_ids.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(unique.len(), card_ids.len(), "All UUIDs should be unique");

    Ok(())
}

#[test]
fn cli_uuids_are_consistent_across_runs() -> Result<(), Box<dyn Error>> {
    let data_home = prepare_data_home()?;
    let deck_root = tempfile::tempdir()?;

    let deck_repo = make_deck_repo(deck_root.path(), "test.deck", MINIMAL_DECK_CONTENT)?;
    let output_dir = tempfile::tempdir()?;
    let output_path = output_dir.path().join("run1.json");

    run_cli(&[deck_repo.clone()], &output_path, data_home.path())?;
    let json1: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
    let ids1: Vec<String> = json1[0]["card_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let output_path2 = output_dir.path().join("run2.json");
    run_cli(&[deck_repo], &output_path2, data_home.path())?;
    let json2: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path2)?)?;
    let ids2: Vec<String> = json2[0]["card_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    assert_eq!(ids1, ids2, "UUIDs should be deterministic across runs");
    assert_eq!(json1[0]["flash_uuid"], json2[0]["flash_uuid"], "Deck UUID should be deterministic");

    Ok(())
}
