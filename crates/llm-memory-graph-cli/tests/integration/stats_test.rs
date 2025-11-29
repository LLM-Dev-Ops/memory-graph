//! Integration tests for the stats command

use assert_cmd::Command;
use predicates::prelude::*;

mod common;
use common::{assert_output_contains, assert_valid_json, assert_valid_yaml, TestDb};

#[tokio::test]
async fn test_stats_empty_database() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("stats")
        .assert()
        .success();
}

#[tokio::test]
async fn test_stats_text_format() {
    let db = TestDb::new().await;

    // Create some test data
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt").await;
    db.add_test_response(prompt.id.into(), "Test response").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("text")
        .arg("stats")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["Database Statistics", "Total Nodes:", "Total Edges:", "Total Sessions:"]);
}

#[tokio::test]
async fn test_stats_json_format() {
    let db = TestDb::new().await;

    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt").await;
    db.add_test_response(prompt.id.into(), "Test response").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("stats")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert!(json.get("node_count").is_some());
    assert!(json.get("edge_count").is_some());
    assert!(json.get("session_count").is_some());

    // Should have 3 nodes: session, prompt, response
    assert!(json["node_count"].as_u64().unwrap() >= 3);
}

#[tokio::test]
async fn test_stats_yaml_format() {
    let db = TestDb::new().await;

    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("yaml")
        .arg("stats")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let yaml = assert_valid_yaml(&output_str);

    assert!(yaml.get("node_count").is_some());
    assert!(yaml.get("edge_count").is_some());
    assert!(yaml.get("session_count").is_some());
}

#[tokio::test]
async fn test_stats_table_format() {
    let db = TestDb::new().await;

    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test prompt").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("table")
        .arg("stats")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["Metric", "Count", "Total Nodes", "Total Edges", "Total Sessions"]);
}

#[tokio::test]
async fn test_stats_with_multiple_sessions() {
    let db = TestDb::new().await;

    // Create multiple sessions with data
    for i in 0..3 {
        let session_id = db.create_test_session(None).await;
        let prompt = db.add_test_prompt(session_id, &format!("Prompt {}", i)).await;
        db.add_test_response(prompt.id.into(), &format!("Response {}", i))
            .await;
    }
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("stats")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert_eq!(json["session_count"].as_u64().unwrap(), 3);
    // At least 9 nodes: 3 sessions + 3 prompts + 3 responses
    assert!(json["node_count"].as_u64().unwrap() >= 9);
}

#[tokio::test]
async fn test_stats_invalid_db_path() {
    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg("/nonexistent/path/to/db")
        .arg("stats")
        .assert()
        .failure();
}
