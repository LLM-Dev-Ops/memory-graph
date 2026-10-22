//! Integration clients for external services
//!
//! This crate provides client libraries for integrating with:
//! - **LLM-Registry**: Model registry and metadata tracking
//! - **Data-Vault**: Long-term session archival and retrieval
//!
//! # Examples
//!
//! ## LLM-Registry Client
//!
//! ```no_run
//! use llm_memory_graph_integrations::registry::{Registry, RegistryClient, RegistryConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = RegistryConfig::new("https://registry.example.com", "api-key");
//!     let client = RegistryClient::new(config)?;
//!
//!     // Register a model
//!     client.register_model("gpt-4", "1.0.0", serde_json::json!({
//!         "provider": "OpenAI",
//!         "context_window": 8192
//!     })).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Data-Vault Client
//!
//! ```no_run
//! use llm_memory_graph_integrations::vault::{Vault, VaultClient, VaultConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = VaultConfig::new("https://vault.example.com", "api-key");
//!     let client = VaultClient::new(config)?;
//!
//!     // Archive a session
//!     client.archive_session("session-123", b"session data").await?;
//!
//!     // Retrieve a session
//!     let data = client.retrieve_session("session-123").await?;
//!
//!     Ok(())
//! }
//! ```

#[cfg(feature = "registry")]
pub mod registry;

#[cfg(feature = "vault")]
pub mod vault;

pub mod error;

pub use error::{Error, Result};

#[cfg(feature = "registry")]
pub use registry::{RegistryClient, RegistryConfig};

#[cfg(feature = "vault")]
pub use vault::{VaultClient, VaultConfig};
