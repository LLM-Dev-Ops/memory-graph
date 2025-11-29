//! Integration tests for export commands

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

mod common;
use common::{assert_valid_json, TestDb};

#[tokio::test]
async fn test_export_session_json() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt").await;
    db.add_test_response(prompt.id.into(), "Test response").await;
    db.flush().await;

    let output_file = db.path().join("export.json");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("session")
        .arg(session_id.to_string())
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .assert()
        .success();

    // Verify file was created
    assert!(output_file.exists());

    // Verify file content
    let content = fs::read_to_string(&output_file).unwrap();
    let json = assert_valid_json(&content);

    assert!(json.get("session").is_some());
    assert!(json.get("nodes").is_some());
    assert!(json.get("node_count").is_some());
    assert!(json.get("exported_at").is_some());
}

#[tokio::test]
async fn test_export_session_msgpack() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let output_file = db.path().join("export.msgpack");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("session")
        .arg(session_id.to_string())
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .arg("--export-format")
        .arg("msgpack")
        .assert()
        .success();

    // Verify file was created
    assert!(output_file.exists());

    // Verify file is not empty
    let metadata = fs::metadata(&output_file).unwrap();
    assert!(metadata.len() > 0);
}

#[tokio::test]
async fn test_export_session_json_format_output() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let output_file = db.path().join("export.json");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("export")
        .arg("session")
        .arg(session_id.to_string())
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert_eq!(json["status"].as_str().unwrap(), "success");
    assert!(json.get("output_file").is_some());
    assert!(json.get("node_count").is_some());
}

#[tokio::test]
async fn test_export_session_invalid_uuid() {
    let db = TestDb::new().await;
    let output_file = db.path().join("export.json");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("session")
        .arg("invalid-uuid")
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .assert()
        .failure();
}

#[tokio::test]
async fn test_export_session_nonexistent() {
    let db = TestDb::new().await;
    let output_file = db.path().join("export.json");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("session")
        .arg("00000000-0000-0000-0000-000000000000")
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .assert()
        .failure();
}

#[tokio::test]
async fn test_export_database_json() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let output_file = db.path().join("database_export.json");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("database")
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .assert()
        .success();

    // Verify file was created
    assert!(output_file.exists());

    // Verify file content
    let content = fs::read_to_string(&output_file).unwrap();
    let json = assert_valid_json(&content);

    assert!(json.get("version").is_some());
    assert!(json.get("stats").is_some());
    assert!(json.get("exported_at").is_some());
}

#[tokio::test]
async fn test_export_database_msgpack() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let output_file = db.path().join("database_export.msgpack");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("database")
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .arg("--export-format")
        .arg("msgpack")
        .assert()
        .success();

    // Verify file was created
    assert!(output_file.exists());
}

#[tokio::test]
async fn test_export_session_with_multiple_nodes() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;

    // Add multiple prompts and responses
    for i in 0..3 {
        let prompt = db.add_test_prompt(session_id, &format!("Prompt {}", i)).await;
        db.add_test_response(prompt.id.into(), &format!("Response {}", i))
            .await;
    }
    db.flush().await;

    let output_file = db.path().join("export.json");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("session")
        .arg(session_id.to_string())
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .assert()
        .success();

    // Verify file content
    let content = fs::read_to_string(&output_file).unwrap();
    let json = assert_valid_json(&content);

    // Should have 6 nodes: 3 prompts + 3 responses
    assert_eq!(json["node_count"].as_u64().unwrap(), 6);
}

#[tokio::test]
async fn test_export_invalid_format() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.flush().await;

    let output_file = db.path().join("export.json");

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("session")
        .arg(session_id.to_string())
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .arg("--export-format")
        .arg("invalid")
        .assert()
        .failure();
}
