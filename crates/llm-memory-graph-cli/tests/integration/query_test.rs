//! Integration tests for the query command

use assert_cmd::Command;
use predicates::prelude::*;

mod common;
use common::{assert_output_contains, assert_valid_json, assert_valid_yaml, TestDb};

#[tokio::test]
async fn test_query_with_session_filter() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt").await;
    db.add_test_response(prompt.id.into(), "Test response").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["Query Results:", "nodes"]);
}

#[tokio::test]
async fn test_query_with_session_filter_json() {
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
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert!(json.get("count").is_some());
    assert!(json.get("nodes").is_some());
    // Should have at least 2 nodes: prompt and response
    assert!(json["count"].as_u64().unwrap() >= 2);
}

#[tokio::test]
async fn test_query_with_node_type_filter() {
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
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .arg("--node-type")
        .arg("prompt")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    // Should only have prompt nodes
    assert_eq!(json["count"].as_u64().unwrap(), 1);
}

#[tokio::test]
async fn test_query_with_node_type_response() {
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
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .arg("--node-type")
        .arg("response")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    // Should only have response nodes
    assert_eq!(json["count"].as_u64().unwrap(), 1);
}

#[tokio::test]
async fn test_query_with_limit() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;

    // Add multiple prompts
    for i in 0..5 {
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
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .arg("--limit")
        .arg("3")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    // Should be limited to 3 nodes
    assert_eq!(json["count"].as_u64().unwrap(), 3);
}

#[tokio::test]
async fn test_query_invalid_node_type() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .arg("--node-type")
        .arg("invalid_type")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_query_invalid_session_uuid() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("query")
        .arg("--session")
        .arg("not-a-uuid")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_query_no_session_filter() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("query")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_query_empty_results() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .arg("--node-type")
        .arg("prompt")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert_eq!(json["count"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn test_query_yaml_format() {
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
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let yaml = assert_valid_yaml(&output_str);

    assert!(yaml.get("count").is_some());
    assert!(yaml.get("nodes").is_some());
}

#[tokio::test]
async fn test_query_table_format() {
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
        .arg("query")
        .arg("--session")
        .arg(session_id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["ID", "Type", "Created", "Session"]);
}
