//! Integration tests for agent commands

use assert_cmd::Command;
use predicates::prelude::*;

mod common;
use common::{assert_output_contains, assert_valid_json, assert_valid_yaml, TestDb};

#[tokio::test]
async fn test_agent_create() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("create")
        .arg("--name")
        .arg("test_agent")
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_create_with_description() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("create")
        .arg("--name")
        .arg("chatbot")
        .arg("--description")
        .arg("A helpful chatbot agent")
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_create_with_model() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("create")
        .arg("--name")
        .arg("gpt_agent")
        .arg("--model")
        .arg("gpt-4")
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_get_text_format() {
    let db = TestDb::new().await;
    let agent = db.create_test_agent("test_agent", Some("gpt-4")).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("get")
        .arg(agent.id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["Agent:", &agent.id.to_string()]);
}

#[tokio::test]
async fn test_agent_get_json_format() {
    let db = TestDb::new().await;
    let agent = db.create_test_agent("test_agent", Some("claude-3-opus")).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("agent")
        .arg("get")
        .arg(agent.id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert_eq!(json["id"].as_str().unwrap(), agent.id.to_string());
    assert_eq!(json["name"].as_str().unwrap(), "test_agent");
}

#[tokio::test]
async fn test_agent_list() {
    let db = TestDb::new().await;
    db.create_test_agent("agent1", Some("gpt-4")).await;
    db.create_test_agent("agent2", Some("claude-3-opus")).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("list")
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_list_json_format() {
    let db = TestDb::new().await;
    db.create_test_agent("agent1", None).await;
    db.create_test_agent("agent2", None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("agent")
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert!(json.get("agents").is_some() || json.is_array());
}

#[tokio::test]
async fn test_agent_list_yaml_format() {
    let db = TestDb::new().await;
    db.create_test_agent("agent1", None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("yaml")
        .arg("agent")
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let _yaml = assert_valid_yaml(&output_str);
}

#[tokio::test]
async fn test_agent_update_name() {
    let db = TestDb::new().await;
    let agent = db.create_test_agent("old_name", None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("update")
        .arg(agent.id.to_string())
        .arg("--name")
        .arg("new_name")
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_update_description() {
    let db = TestDb::new().await;
    let agent = db.create_test_agent("test_agent", None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("update")
        .arg(agent.id.to_string())
        .arg("--description")
        .arg("Updated description")
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_update_model() {
    let db = TestDb::new().await;
    let agent = db.create_test_agent("test_agent", Some("gpt-3.5-turbo")).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("update")
        .arg(agent.id.to_string())
        .arg("--model")
        .arg("gpt-4")
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_update_temperature() {
    let db = TestDb::new().await;
    let agent = db.create_test_agent("test_agent", None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("update")
        .arg(agent.id.to_string())
        .arg("--temperature")
        .arg("0.7")
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_assign() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test prompt").await;
    let agent = db.create_test_agent("test_agent", None).await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("assign")
        .arg(agent.id.to_string())
        .arg(prompt.id.to_string())
        .assert()
        .success();
}

#[tokio::test]
async fn test_agent_get_invalid_uuid() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("get")
        .arg("invalid-uuid")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_agent_get_nonexistent() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("get")
        .arg("00000000-0000-0000-0000-000000000000")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_agent_update_nonexistent() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("agent")
        .arg("update")
        .arg("00000000-0000-0000-0000-000000000000")
        .arg("--name")
        .arg("new_name")
        .assert()
        .failure();
}
