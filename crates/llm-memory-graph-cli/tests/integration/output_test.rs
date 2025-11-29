//! Integration tests for output formatting across all commands

use assert_cmd::Command;

mod common;
use common::{assert_valid_json, assert_valid_yaml, TestDb};

#[tokio::test]
async fn test_stats_all_output_formats() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test").await;
    db.flush().await;

    // Test text format
    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("text")
        .arg("stats")
        .assert()
        .success();

    // Test json format
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
    assert_valid_json(&String::from_utf8(output).unwrap());

    // Test yaml format
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
    assert_valid_yaml(&String::from_utf8(output).unwrap());

    // Test table format
    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("table")
        .arg("stats")
        .assert()
        .success();
}

#[tokio::test]
async fn test_session_get_all_output_formats() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.flush().await;

    for format in &["text", "json", "yaml", "table"] {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        cmd.arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg(format)
            .arg("session")
            .arg("get")
            .arg(session_id.to_string())
            .assert()
            .success();
    }
}

#[tokio::test]
async fn test_query_all_output_formats() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.add_test_prompt(session_id, "Test").await;
    db.flush().await;

    for format in &["text", "json", "yaml", "table"] {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        cmd.arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg(format)
            .arg("query")
            .arg("--session")
            .arg(session_id.to_string())
            .assert()
            .success();
    }
}

#[tokio::test]
async fn test_template_list_all_output_formats() {
    let db = TestDb::new().await;
    db.create_test_template("test", "content").await;
    db.flush().await;

    for format in &["text", "json", "yaml", "table"] {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        cmd.arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg(format)
            .arg("template")
            .arg("list")
            .assert()
            .success();
    }
}

#[tokio::test]
async fn test_agent_list_all_output_formats() {
    let db = TestDb::new().await;
    db.create_test_agent("test", None).await;
    db.flush().await;

    for format in &["text", "json", "yaml", "table"] {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        cmd.arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg(format)
            .arg("agent")
            .arg("list")
            .assert()
            .success();
    }
}

#[tokio::test]
async fn test_verify_all_output_formats() {
    let db = TestDb::new().await;
    db.create_test_session(None).await;
    db.flush().await;

    for format in &["text", "json", "yaml", "table"] {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        cmd.arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg(format)
            .arg("verify")
            .assert()
            .success();
    }
}

#[tokio::test]
async fn test_invalid_output_format() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("invalid_format")
        .arg("stats")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_json_format_consistency() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    let prompt = db.add_test_prompt(session_id, "Test").await;
    db.flush().await;

    // All JSON outputs should be valid JSON
    let commands = vec![
        vec!["stats"],
        vec!["session", "get", &session_id.to_string()],
        vec!["node", "get", &prompt.id.to_string()],
        vec!["query", "--session", &session_id.to_string()],
        vec!["verify"],
    ];

    for cmd_args in commands {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        let mut cmd = cmd
            .arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg("json");

        for arg in cmd_args {
            cmd = cmd.arg(arg);
        }

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8(output).unwrap();
        assert_valid_json(&output_str);
    }
}

#[tokio::test]
async fn test_yaml_format_consistency() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.flush().await;

    // All YAML outputs should be valid YAML
    let commands = vec![vec!["stats"], vec!["session", "get", &session_id.to_string()], vec!["verify"]];

    for cmd_args in commands {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        let mut cmd = cmd
            .arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg("yaml");

        for arg in cmd_args {
            cmd = cmd.arg(arg);
        }

        let output = cmd.assert().success().get_output().stdout.clone();
        let output_str = String::from_utf8(output).unwrap();
        assert_valid_yaml(&output_str);
    }
}

#[tokio::test]
async fn test_export_output_formats() {
    let db = TestDb::new().await;
    let session_id = db.create_test_session(None).await;
    db.flush().await;

    let export_file = db.path().join("export.json");

    for format in &["text", "json", "yaml"] {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        cmd.arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg(format)
            .arg("export")
            .arg("session")
            .arg(session_id.to_string())
            .arg("--output")
            .arg(export_file.to_str().unwrap())
            .assert()
            .success();
    }
}

#[tokio::test]
async fn test_flush_all_output_formats() {
    let db = TestDb::new().await;
    db.create_test_session(None).await;

    for format in &["text", "json", "yaml", "table"] {
        let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
        cmd.arg("--db-path")
            .arg(db.path_str())
            .arg("--format")
            .arg(format)
            .arg("flush")
            .assert()
            .success();
    }
}
