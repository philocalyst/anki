mod common;

use std::{error::Error, fs};
use serde_json::Value;
use common::*;

#[test]
fn cli_handles_multiple_decks() -> Result<(), Box<dyn Error>> {
    let data_home = prepare_data_home()?;
    let deck_root = tempfile::tempdir()?;

    let deck_a = make_deck_repo(deck_root.path(), "deck-a", MINIMAL_DECK_CONTENT)?;
    let deck_b = make_deck_repo(deck_root.path(), "deck-b", MINIMAL_DECK_CONTENT)?;
    let output_dir = tempfile::tempdir()?;
    let output_path = output_dir.path().join("multi.json");

    let output = run_cli(&[deck_a, deck_b], &output_path, data_home.path())?;
    assert!(
        output.status.success(),
        "flash CLI failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
    assert_eq!(json.len(), 2);
    assert!(json[0]["flash_uuid"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
    assert!(json[1]["flash_uuid"].as_str().map(|s| !s.is_empty()).unwrap_or(false));
    // Both decks have identical content — deterministic UUIDs should match
    assert_eq!(json[0]["flash_uuid"], json[1]["flash_uuid"]);

    Ok(())
}

#[test]
fn cli_handles_deck_with_one_note() -> Result<(), Box<dyn Error>> {
    let data_home = prepare_data_home()?;
    let deck_root = tempfile::tempdir()?;
    let content = "/ Basic /\nalias Question to Q\n\n(Q) Just one?\n";
    let deck_repo = make_deck_repo(deck_root.path(), "lonely.deck", content)?;
    let output_dir = tempfile::tempdir()?;
    let output_path = output_dir.path().join("out.json");

    let output = run_cli(&[deck_repo], &output_path, data_home.path())?;
    assert!(output.status.success());

    let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
    assert_eq!(json.len(), 1);
    assert_eq!(json[0]["note_count"].as_i64().unwrap_or(0), 1);
    assert_eq!(json[0]["card_ids"].as_array().map(|a| a.len()).unwrap_or(0), 1);

    Ok(())
}

#[test]
fn cli_handles_deck_with_many_notes() -> Result<(), Box<dyn Error>> {
    let data_home = prepare_data_home()?;
    let deck_root = tempfile::tempdir()?;

    // Build a deck with 50 notes
    let mut content = String::from("/ Basic /\nalias Question to Q\nalias Answer to A\n\n");
    for i in 0..50 {
        content.push_str(&format!("(Q) Question {i}?\n(A) Answer {i}\n\n"));
    }
    let deck_repo = make_deck_repo(deck_root.path(), "big.deck", &content)?;
    let output_dir = tempfile::tempdir()?;
    let output_path = output_dir.path().join("out.json");

    let output = run_cli(&[deck_repo], &output_path, data_home.path())?;
    assert!(output.status.success());

    let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
    assert_eq!(json.len(), 1);
    assert_eq!(json[0]["note_count"].as_i64().unwrap_or(0), 50);
    assert_eq!(json[0]["card_ids"].as_array().map(|a| a.len()).unwrap_or(0), 50);

    // All UUIDs must be unique
    let ids: Vec<&str> =
        json[0]["card_ids"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    let unique: std::collections::HashSet<&&str> = ids.iter().collect();
    assert_eq!(unique.len(), 50, "All 50 UUIDs should be unique");

    Ok(())
}

#[test]
fn cli_handles_deck_with_model_directory() -> Result<(), Box<dyn Error>> {
    let data_home = prepare_data_home()?;
    let deck_root = tempfile::tempdir()?;
    let micro_repo = borrow_deck_repo(
        deck_root.path(),
        "micro.deck",
        "out/Micro.deck/index.flash",
        Some("/ Basic /\nalias Question to Q\nalias Answer to A\n\n"),
    )?;
    let output_dir = tempfile::tempdir()?;
    let output_path = output_dir.path().join("model-deck.json");

    let output = run_cli(&[micro_repo], &output_path, data_home.path())?;
    assert!(
        output.status.success(),
        "CLI failed on deck with model directory:\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Vec<Value> = serde_json::from_str(&fs::read_to_string(&output_path)?)?;
    assert_eq!(json.len(), 1);
    assert!(json[0]["note_count"].as_i64().unwrap_or(0) > 0);
    assert_eq!(json[0]["model_count"].as_i64().unwrap_or(0), 1);

    Ok(())
}
