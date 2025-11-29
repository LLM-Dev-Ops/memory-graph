//! Integration tests for import command

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

mod common;
use common::{assert_valid_json, create_sample_export, TestDb};
use llm_memory_graph_types::SessionId;

#[tokio::test]
async fn test_import_json_dry_run() {
    let db = TestDb::new().await;
    let session_id = SessionId::new();

    let import_file = db.path().join("import.json");
    create_sample_export(&import_file, session_id).await.unwrap();

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("import")
        .arg("--input")
        .arg(import_file.to_str().unwrap())
        .arg("--dry-run")
        .assert()
        .success();
}

#[tokio::test]
async fn test_import_json_format_output() {
    let db = TestDb::new().await;
    let session_id = SessionId::new();

    let import_file = db.path().join("import.json");
    create_sample_export(&import_file, session_id).await.unwrap();

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("import")
        .arg("--input")
        .arg(import_file.to_str().unwrap())
        .arg("--dry-run")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert!(json.get("type").is_some());
    assert_eq!(json["dry_run"].as_bool().unwrap(), true);
}

#[tokio::test]
async fn test_import_session_export() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt").await;
    db.add_test_response(prompt.id.into(), "Test response").await;
    db.flush().await;

    // Export first
    let export_file = db.path().join("export.json");
    let mut export_cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    export_cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("export")
        .arg("session")
        .arg(session_id.to_string())
        .arg("--output")
        .arg(export_file.to_str().unwrap())
        .assert()
        .success();

    // Then try to import
    let mut import_cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    import_cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("import")
        .arg("--input")
        .arg(export_file.to_str().unwrap())
        .arg("--dry-run")
        .assert()
        .success();
}

#[tokio::test]
async fn test_import_msgpack_dry_run() {
    let db = TestDb::new().await;
    let session_id = SessionId::new();

    // Create a valid msgpack export
    let data = serde_json::json!({
        "session": {
            "id": session_id.to_string(),
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "metadata": {},
            "tags": []
        },
        "nodes": [],
        "node_count": 0
    });

    let import_file = db.path().join("import.msgpack");
    let msgpack = rmp_serde::to_vec(&data).unwrap();
    fs::write(&import_file, msgpack).unwrap();

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("import")
        .arg("--input")
        .arg(import_file.to_str().unwrap())
        .arg("--import-format")
        .arg("msgpack")
        .arg("--dry-run")
        .assert()
        .success();
}

#[tokio::test]
async fn test_import_invalid_file() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("import")
        .arg("--input")
        .arg("/nonexistent/file.json")
        .arg("--dry-run")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_import_invalid_json() {
    let db = TestDb::new().await;

    let import_file = db.path().join("invalid.json");
    fs::write(&import_file, "{ invalid json ").unwrap();

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("import")
        .arg("--input")
        .arg(import_file.to_str().unwrap())
        .arg("--dry-run")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_import_yaml_format_output() {
    let db = TestDb::new().await;
    let session_id = SessionId::new();

    let import_file = db.path().join("import.json");
    create_sample_export(&import_file, session_id).await.unwrap();

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("yaml")
        .arg("import")
        .arg("--input")
        .arg(import_file.to_str().unwrap())
        .arg("--dry-run")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    // YAML output should be valid
    assert!(output_str.contains("type:"));
}

#[tokio::test]
async fn test_import_without_dry_run() {
    let db = TestDb::new().await;
    let session_id = SessionId::new();

    let import_file = db.path().join("import.json");
    create_sample_export(&import_file, session_id).await.unwrap();

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("import")
        .arg("--input")
        .arg(import_file.to_str().unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    // Should contain warning about not being fully implemented
    assert!(output_str.contains("not yet fully implemented"));
}
