//! Integration tests for session commands

use assert_cmd::Command;
use predicates::prelude::*;

mod common;
use common::{assert_output_contains, assert_valid_json, assert_valid_yaml, TestDb};

#[tokio::test]
async fn test_session_get_text_format() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt").await;
    db.add_test_response(prompt.id.into(), "Test response").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("session")
        .arg("get")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["Session:", &session_id.to_string(), "Created:", "Updated:", "Nodes:"]);
}

#[tokio::test]
async fn test_session_get_json_format() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("session")
        .arg("get")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert_eq!(json["id"].as_str().unwrap(), session_id.to_string());
    assert!(json.get("created_at").is_some());
    assert!(json.get("updated_at").is_some());
    assert!(json.get("node_count").is_some());
}

#[tokio::test]
async fn test_session_get_yaml_format() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("yaml")
        .arg("session")
        .arg("get")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let yaml = assert_valid_yaml(&output_str);

    assert_eq!(yaml["id"].as_str().unwrap(), session_id.to_string());
}

#[tokio::test]
async fn test_session_get_table_format() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("table")
        .arg("session")
        .arg("get")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["Field", "Value", "ID", "Created", "Updated", "Nodes"]);
}

#[tokio::test]
async fn test_session_get_with_metadata() {
    let db = TestDb::new().await;

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("user".to_string(), "test_user".to_string());
    metadata.insert("app".to_string(), "test_app".to_string());

    let session_id = db.create_test_session(Some(metadata)).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("session")
        .arg("get")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert_eq!(json["metadata"]["user"].as_str().unwrap(), "test_user");
    assert_eq!(json["metadata"]["app"].as_str().unwrap(), "test_app");
}

#[tokio::test]
async fn test_session_get_invalid_uuid() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("session")
        .arg("get")
        .arg("not-a-valid-uuid")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_session_get_nonexistent() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("session")
        .arg("get")
        .arg("00000000-0000-0000-0000-000000000000")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_node_get_text_format() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt content").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("node")
        .arg("get")
        .arg(prompt.id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["Node:", &prompt.id.to_string(), "Type:"]);
}

#[tokio::test]
async fn test_node_get_json_format() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("node")
        .arg("get")
        .arg(prompt.id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert!(json.get("Prompt").is_some() || json.get("content").is_some());
}

#[tokio::test]
async fn test_node_get_invalid_uuid() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("node")
        .arg("get")
        .arg("invalid-uuid")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_flush_command() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("flush")
        .assert()
        .success();
}

#[tokio::test]
async fn test_verify_command() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("verify")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["verified", "Database verification complete"]);
}

#[tokio::test]
async fn test_verify_json_format() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("verify")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert_eq!(json["status"].as_str().unwrap(), "verified");
    assert!(json.get("nodes").is_some());
    assert!(json.get("edges").is_some());
    assert!(json.get("sessions").is_some());
}
