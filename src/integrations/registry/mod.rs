//! LLM-Registry integration module
//!
//! Provides client functionality for integrating with the LLM-Registry service,
//! including session registration, model metadata retrieval, and usage tracking.

pub mod client;
pub mod types;

pub use client::RegistryClient;
pub use types::{
    ModelListResponse, ModelMetadata, ModelParameters, RegistryConfig, SessionInfo,
    SessionListResponse, SessionRegistration, SessionStatus, UsageReport, UsageStats,
};
