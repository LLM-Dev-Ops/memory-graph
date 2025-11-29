//! Integration tests for template commands

use assert_cmd::Command;
use predicates::prelude::*;

mod common;
use common::{assert_output_contains, assert_valid_json, assert_valid_yaml, TestDb};

#[tokio::test]
async fn test_template_create() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("create")
        .arg("--name")
        .arg("test_template")
        .arg("--content")
        .arg("Hello {{name}}, welcome to {{place}}!")
        .assert()
        .success();
}

#[tokio::test]
async fn test_template_create_with_description() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("create")
        .arg("--name")
        .arg("greeting")
        .arg("--content")
        .arg("Hello {{name}}!")
        .arg("--description")
        .arg("A simple greeting template")
        .assert()
        .success();
}

#[tokio::test]
async fn test_template_create_with_category() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("create")
        .arg("--name")
        .arg("email_template")
        .arg("--content")
        .arg("Dear {{recipient}}, ...")
        .arg("--category")
        .arg("email")
        .assert()
        .success();
}

#[tokio::test]
async fn test_template_get_text_format() {
    let db = TestDb::new().await;
    let template = db.create_test_template("test_template", "Content {{var}}").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("get")
        .arg(template.id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert_output_contains(&output_str, &["Template:", &template.id.to_string()]);
}

#[tokio::test]
async fn test_template_get_json_format() {
    let db = TestDb::new().await;
    let template = db.create_test_template("test_template", "Content {{var}}").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("template")
        .arg("get")
        .arg(template.id.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert_eq!(json["id"].as_str().unwrap(), template.id.to_string());
    assert_eq!(json["name"].as_str().unwrap(), "test_template");
}

#[tokio::test]
async fn test_template_list() {
    let db = TestDb::new().await;
    db.create_test_template("template1", "Content 1").await;
    db.create_test_template("template2", "Content 2").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("list")
        .assert()
        .success();
}

#[tokio::test]
async fn test_template_list_json_format() {
    let db = TestDb::new().await;
    db.create_test_template("template1", "Content 1").await;
    db.create_test_template("template2", "Content 2").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("json")
        .arg("template")
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    let json = assert_valid_json(&output_str);

    assert!(json.get("templates").is_some() || json.is_array());
}

#[tokio::test]
async fn test_template_list_yaml_format() {
    let db = TestDb::new().await;
    db.create_test_template("template1", "Content 1").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    let output = cmd
        .arg("--db-path")
        .arg(db.path_str())
        .arg("--format")
        .arg("yaml")
        .arg("template")
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
async fn test_template_instantiate() {
    let db = TestDb::new().await;
    let template = db.create_test_template("greeting", "Hello {{name}}!").await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("instantiate")
        .arg(template.id.to_string())
        .arg("--variables")
        .arg("name=World")
        .assert()
        .success();
}

#[tokio::test]
async fn test_template_instantiate_multiple_vars() {
    let db = TestDb::new().await;
    let template = db
        .create_test_template("greeting", "Hello {{name}} from {{place}}!")
        .await;
    db.flush().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("instantiate")
        .arg(template.id.to_string())
        .arg("--variables")
        .arg("name=Alice")
        .arg("--variables")
        .arg("place=Wonderland")
        .assert()
        .success();
}

#[tokio::test]
async fn test_template_get_invalid_uuid() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("get")
        .arg("invalid-uuid")
        .assert()
        .failure();
}

#[tokio::test]
async fn test_template_get_nonexistent() {
    let db = TestDb::new().await;

    let mut cmd = Command::cargo_bin("llm-memory-graph").unwrap();
    cmd.arg("--db-path")
        .arg(db.path_str())
        .arg("template")
        .arg("get")
        .arg("00000000-0000-0000-0000-000000000000")
        .assert()
        .failure();
}
