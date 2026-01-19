//! Telemetry module for LLM-Observatory integration
//!
//! Emits telemetry compatible with LLM-Observatory for monitoring and analysis.

use crate::contracts::{DecisionEvent, DecisionEventTelemetry};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, span, Level};
use uuid::Uuid;

/// Telemetry event types emitted by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    /// Agent invocation started
    InvocationStarted {
        execution_ref: Uuid,
        agent_id: String,
        agent_version: String,
        timestamp: DateTime<Utc>,
    },
    /// Agent invocation completed
    InvocationCompleted {
        execution_ref: Uuid,
        success: bool,
        duration_ms: u64,
        nodes_created: usize,
        edges_created: usize,
        artifacts_stored: usize,
        timestamp: DateTime<Utc>,
    },
    /// Agent invocation failed
    InvocationFailed {
        execution_ref: Uuid,
        error_code: String,
        error_message: String,
        duration_ms: u64,
        timestamp: DateTime<Utc>,
    },
    /// Decision captured
    DecisionCaptured {
        execution_ref: Uuid,
        decision_id: Uuid,
        session_id: Uuid,
        confidence: f64,
        timestamp: DateTime<Utc>,
    },
    /// Artifact stored
    ArtifactStored {
        execution_ref: Uuid,
        artifact_id: Uuid,
        artifact_type: String,
        content_size_bytes: usize,
        timestamp: DateTime<Utc>,
    },
    /// RuVector operation
    RuVectorOperation {
        execution_ref: Uuid,
        operation: String,
        success: bool,
        latency_ms: u64,
        timestamp: DateTime<Utc>,
    },
}

impl TelemetryEvent {
    /// Get the execution reference for this event
    pub fn execution_ref(&self) -> Uuid {
        match self {
            Self::InvocationStarted { execution_ref, .. }
            | Self::InvocationCompleted { execution_ref, .. }
            | Self::InvocationFailed { execution_ref, .. }
            | Self::DecisionCaptured { execution_ref, .. }
            | Self::ArtifactStored { execution_ref, .. }
            | Self::RuVectorOperation { execution_ref, .. } => *execution_ref,
        }
    }

    /// Get the timestamp for this event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::InvocationStarted { timestamp, .. }
            | Self::InvocationCompleted { timestamp, .. }
            | Self::InvocationFailed { timestamp, .. }
            | Self::DecisionCaptured { timestamp, .. }
            | Self::ArtifactStored { timestamp, .. }
            | Self::RuVectorOperation { timestamp, .. } => *timestamp,
        }
    }
}

/// Telemetry collector for tracking agent metrics
#[derive(Debug)]
pub struct TelemetryCollector {
    execution_ref: Uuid,
    start_time: Instant,
    events: Vec<TelemetryEvent>,
    ruvector_latencies: Vec<u64>,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new(execution_ref: Uuid) -> Self {
        let collector = Self {
            execution_ref,
            start_time: Instant::now(),
            events: Vec::new(),
            ruvector_latencies: Vec::new(),
        };

        // Emit start event
        let start_event = TelemetryEvent::InvocationStarted {
            execution_ref,
            agent_id: crate::constants::AGENT_ID.to_string(),
            agent_version: crate::constants::AGENT_VERSION.to_string(),
            timestamp: Utc::now(),
        };

        info!(
            event = ?start_event,
            "Agent invocation started"
        );

        collector
    }

    /// Record a decision capture
    pub fn record_decision_capture(&mut self, decision_id: Uuid, session_id: Uuid, confidence: f64) {
        let event = TelemetryEvent::DecisionCaptured {
            execution_ref: self.execution_ref,
            decision_id,
            session_id,
            confidence,
            timestamp: Utc::now(),
        };

        info!(
            event = ?event,
            "Decision captured"
        );

        self.events.push(event);
    }

    /// Record an artifact storage
    pub fn record_artifact_stored(
        &mut self,
        artifact_id: Uuid,
        artifact_type: &str,
        content_size_bytes: usize,
    ) {
        let event = TelemetryEvent::ArtifactStored {
            execution_ref: self.execution_ref,
            artifact_id,
            artifact_type: artifact_type.to_string(),
            content_size_bytes,
            timestamp: Utc::now(),
        };

        info!(
            event = ?event,
            "Artifact stored"
        );

        self.events.push(event);
    }

    /// Record a RuVector operation
    pub fn record_ruvector_operation(&mut self, operation: &str, success: bool, latency_ms: u64) {
        let event = TelemetryEvent::RuVectorOperation {
            execution_ref: self.execution_ref,
            operation: operation.to_string(),
            success,
            latency_ms,
            timestamp: Utc::now(),
        };

        info!(
            event = ?event,
            "RuVector operation completed"
        );

        self.events.push(event);
        self.ruvector_latencies.push(latency_ms);
    }

    /// Complete the telemetry collection with success
    pub fn complete_success(
        &self,
        nodes_created: usize,
        edges_created: usize,
        artifacts_stored: usize,
    ) -> DecisionEventTelemetry {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;

        let event = TelemetryEvent::InvocationCompleted {
            execution_ref: self.execution_ref,
            success: true,
            duration_ms,
            nodes_created,
            edges_created,
            artifacts_stored,
            timestamp: Utc::now(),
        };

        info!(
            event = ?event,
            "Agent invocation completed successfully"
        );

        DecisionEventTelemetry {
            duration_ms: Some(duration_ms),
            memory_bytes: None, // Not tracking memory in this implementation
            ruvector_latency_ms: self.average_ruvector_latency(),
        }
    }

    /// Complete the telemetry collection with failure
    pub fn complete_failure(&self, error_code: &str, error_message: &str) {
        let duration_ms = self.start_time.elapsed().as_millis() as u64;

        let event = TelemetryEvent::InvocationFailed {
            execution_ref: self.execution_ref,
            error_code: error_code.to_string(),
            error_message: error_message.to_string(),
            duration_ms,
            timestamp: Utc::now(),
        };

        info!(
            event = ?event,
            "Agent invocation failed"
        );
    }

    /// Get all collected events
    pub fn events(&self) -> &[TelemetryEvent] {
        &self.events
    }

    /// Get the execution reference
    pub fn execution_ref(&self) -> Uuid {
        self.execution_ref
    }

    /// Calculate average RuVector latency
    fn average_ruvector_latency(&self) -> Option<u64> {
        if self.ruvector_latencies.is_empty() {
            None
        } else {
            let sum: u64 = self.ruvector_latencies.iter().sum();
            Some(sum / self.ruvector_latencies.len() as u64)
        }
    }
}

/// Trait for telemetry emitter (for testing/mocking)
#[async_trait::async_trait]
pub trait TelemetryEmitter: Send + Sync {
    /// Emit a telemetry event
    async fn emit(&self, event: TelemetryEvent) -> Result<(), String>;

    /// Flush all pending events
    async fn flush(&self) -> Result<(), String>;
}

/// Default telemetry emitter using tracing
#[derive(Debug, Default)]
pub struct TracingTelemetryEmitter;

#[async_trait::async_trait]
impl TelemetryEmitter for TracingTelemetryEmitter {
    async fn emit(&self, event: TelemetryEvent) -> Result<(), String> {
        let _span = span!(
            Level::INFO,
            "telemetry",
            execution_ref = %event.execution_ref(),
        );

        match &event {
            TelemetryEvent::InvocationStarted { agent_id, .. } => {
                info!(agent_id = %agent_id, "Emitting invocation started");
            }
            TelemetryEvent::InvocationCompleted {
                success,
                duration_ms,
                ..
            } => {
                info!(success = %success, duration_ms = %duration_ms, "Emitting invocation completed");
            }
            TelemetryEvent::InvocationFailed {
                error_code,
                error_message,
                ..
            } => {
                info!(error_code = %error_code, error_message = %error_message, "Emitting invocation failed");
            }
            TelemetryEvent::DecisionCaptured {
                decision_id,
                confidence,
                ..
            } => {
                info!(decision_id = %decision_id, confidence = %confidence, "Emitting decision captured");
            }
            TelemetryEvent::ArtifactStored {
                artifact_id,
                artifact_type,
                ..
            } => {
                info!(artifact_id = %artifact_id, artifact_type = %artifact_type, "Emitting artifact stored");
            }
            TelemetryEvent::RuVectorOperation {
                operation,
                success,
                latency_ms,
                ..
            } => {
                info!(operation = %operation, success = %success, latency_ms = %latency_ms, "Emitting ruvector operation");
            }
        }

        Ok(())
    }

    async fn flush(&self) -> Result<(), String> {
        // Tracing handles flushing automatically
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_collector_creation() {
        let execution_ref = Uuid::new_v4();
        let collector = TelemetryCollector::new(execution_ref);
        assert_eq!(collector.execution_ref(), execution_ref);
    }

    #[test]
    fn test_telemetry_collector_record_decision() {
        let execution_ref = Uuid::new_v4();
        let mut collector = TelemetryCollector::new(execution_ref);

        let decision_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        collector.record_decision_capture(decision_id, session_id, 0.95);

        assert_eq!(collector.events().len(), 1);
    }

    #[test]
    fn test_telemetry_event_serialization() {
        let event = TelemetryEvent::InvocationStarted {
            execution_ref: Uuid::new_v4(),
            agent_id: "test-agent".to_string(),
            agent_version: "1.0.0".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("invocation_started"));
        assert!(json.contains("test-agent"));
    }
}
