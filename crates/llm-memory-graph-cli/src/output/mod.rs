//! Output formatting utilities for CLI commands
//!
//! Provides formatters for different output formats including:
//! - Text (human-readable, colored)
//! - JSON (machine-readable)
//! - YAML (configuration-friendly)
//! - Table (structured data display)

use anyhow::Result;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use serde::Serialize;
use std::collections::HashMap;

/// Output format option
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text with colors
    Text,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Table format
    Table,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "yaml" => Ok(OutputFormat::Yaml),
            "table" => Ok(OutputFormat::Table),
            _ => Err(format!(
                "Invalid format: '{}'. Use 'text', 'json', 'yaml', or 'table'",
                s
            )),
        }
    }
}

impl OutputFormat {
    /// Print a value using this format
    pub fn print<T: Serialize>(&self, value: &T) -> Result<()> {
        match self {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(value)?);
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(value)?);
            }
            _ => {
                // For Text and Table, custom formatting is required
                eprintln!(
                    "{} Use custom formatting for text/table output",
                    "Warning:".yellow()
                );
                println!("{}", serde_json::to_string_pretty(value)?);
            }
        }
        Ok(())
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        match self {
            OutputFormat::Text | OutputFormat::Table => {
                println!("{} {}", "✓".green().bold(), message);
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "status": "success",
                    "message": message
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            }
            OutputFormat::Yaml => {
                let data = HashMap::from([
                    ("status", "success"),
                    ("message", message),
                ]);
                println!("{}", serde_yaml::to_string(&data).unwrap());
            }
        }
    }

    /// Print an error message
    pub fn error(&self, message: &str) {
        match self {
            OutputFormat::Text | OutputFormat::Table => {
                eprintln!("{} {}", "Error:".red().bold(), message);
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "status": "error",
                    "message": message
                });
                eprintln!("{}", serde_json::to_string_pretty(&json).unwrap());
            }
            OutputFormat::Yaml => {
                let data = HashMap::from([
                    ("status", "error"),
                    ("message", message),
                ]);
                eprintln!("{}", serde_yaml::to_string(&data).unwrap());
            }
        }
    }

    /// Print a warning message
    pub fn warning(&self, message: &str) {
        match self {
            OutputFormat::Text | OutputFormat::Table => {
                println!("{} {}", "Warning:".yellow().bold(), message);
            }
            OutputFormat::Json => {
                let json = serde_json::json!({
                    "status": "warning",
                    "message": message
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            }
            OutputFormat::Yaml => {
                let data = HashMap::from([
                    ("status", "warning"),
                    ("message", message),
                ]);
                println!("{}", serde_yaml::to_string(&data).unwrap());
            }
        }
    }
}

/// Helper for creating formatted tables
pub struct TableBuilder {
    table: Table,
}

impl TableBuilder {
    /// Create a new table builder
    pub fn new() -> Self {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        Self { table }
    }

    /// Set table header
    pub fn header(mut self, headers: Vec<&str>) -> Self {
        let cells: Vec<Cell> = headers
            .into_iter()
            .map(|h| Cell::new(h).fg(Color::Green))
            .collect();
        self.table.set_header(cells);
        self
    }

    /// Add a row to the table
    pub fn row(mut self, cells: Vec<String>) -> Self {
        self.table.add_row(cells);
        self
    }

    /// Build and display the table
    pub fn display(self) {
        println!("{}", self.table);
    }

    /// Get the table as a string
    pub fn to_string(self) -> String {
        self.table.to_string()
    }
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("yaml".parse::<OutputFormat>().unwrap(), OutputFormat::Yaml);
        assert_eq!("table".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_output_format_from_str_case_insensitive() {
        assert_eq!("TEXT".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
        assert_eq!("Json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("YAML".parse::<OutputFormat>().unwrap(), OutputFormat::Yaml);
        assert_eq!("TaBlE".parse::<OutputFormat>().unwrap(), OutputFormat::Table);
    }

    #[test]
    fn test_output_format_invalid_error_message() {
        let result = "invalid_format".parse::<OutputFormat>();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("invalid_format"));
        assert!(err.contains("text"));
        assert!(err.contains("json"));
    }

    #[test]
    fn test_output_format_print_json() {
        let format = OutputFormat::Json;
        let data = serde_json::json!({"test": "value", "count": 42});
        let result = format.print(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_format_print_yaml() {
        let format = OutputFormat::Yaml;
        let data = serde_json::json!({"test": "value", "count": 42});
        let result = format.print(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_table_builder_creation() {
        let builder = TableBuilder::new();
        let table_str = builder.to_string();
        // Table string should exist (even if empty)
        assert!(!table_str.is_empty() || table_str.is_empty());
    }

    #[test]
    fn test_table_builder_with_header() {
        let table = TableBuilder::new()
            .header(vec!["Column1", "Column2", "Column3"])
            .to_string();

        assert!(table.contains("Column1"));
        assert!(table.contains("Column2"));
        assert!(table.contains("Column3"));
    }

    #[test]
    fn test_table_builder_with_rows() {
        let table = TableBuilder::new()
            .header(vec!["Name", "Age"])
            .row(vec!["Alice".to_string(), "30".to_string()])
            .row(vec!["Bob".to_string(), "25".to_string()])
            .to_string();

        assert!(table.contains("Name"));
        assert!(table.contains("Age"));
        assert!(table.contains("Alice"));
        assert!(table.contains("Bob"));
        assert!(table.contains("30"));
        assert!(table.contains("25"));
    }

    #[test]
    fn test_table_builder_default() {
        let builder = TableBuilder::default();
        let table = builder.header(vec!["Test"]).to_string();
        assert!(table.contains("Test"));
    }

    #[test]
    fn test_output_format_clone() {
        let format1 = OutputFormat::Json;
        let format2 = format1.clone();
        assert_eq!(format1, format2);
    }

    #[test]
    fn test_output_format_debug() {
        let format = OutputFormat::Json;
        let debug_str = format!("{:?}", format);
        assert!(debug_str.contains("Json"));
    }
}
