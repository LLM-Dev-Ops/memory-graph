//! Edge Function Handler
//!
//! HTTP handler for Google Cloud Edge Functions deployment.
//! Provides the main entry point for the agent.

use crate::agent::{AgentConfig, DecisionMemoryAgent};
use crate::contracts::DecisionMemoryInput;
use crate::error::AgentError;
use crate::ruvector::{DecisionEventQuery, RuVectorClient, RuVectorConfig, RuVectorService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, instrument};
use uuid::Uuid;
use warp::Filter;

/// Request for decision capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRequest {
    /// Decision memory input
    pub input: DecisionMemoryInput,
}

/// Request for decision retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveRequest {
    /// Execution reference to retrieve
    pub execution_ref: Uuid,
}

/// Request for decision inspection (query)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectRequest {
    /// Query parameters
    pub query: DecisionEventQuery,
}

/// Request for decision replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRequest {
    /// Execution reference to replay
    pub execution_ref: Uuid,
    /// Optional modifications to the input
    pub modifications: Option<serde_json::Value>,
}

/// Response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiResponse<T> {
    /// Success response
    Success {
        /// Response data
        data: T,
        /// Execution reference
        execution_ref: Uuid,
    },
    /// Error response
    Error {
        /// Error code
        error_code: String,
        /// Error message
        message: String,
        /// Execution reference
        execution_ref: Uuid,
    },
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a success response
    pub fn success(data: T, execution_ref: Uuid) -> Self {
        Self::Success {
            data,
            execution_ref,
        }
    }
}

impl ApiResponse<()> {
    /// Create an error response
    pub fn error(err: &AgentError) -> Self {
        Self::Error {
            error_code: err.error_code.to_string(),
            message: err.message.clone(),
            execution_ref: err.execution_ref,
        }
    }
}

/// Edge Function Handler
///
/// Handles HTTP requests for the Decision Memory Agent.
pub struct EdgeFunctionHandler {
    agent: DecisionMemoryAgent,
    ruvector: Arc<dyn RuVectorService>,
}

impl EdgeFunctionHandler {
    /// Create a new handler from configuration
    pub fn new(ruvector_config: RuVectorConfig, agent_config: AgentConfig) -> Result<Self, String> {
        let ruvector = RuVectorClient::new(ruvector_config)
            .map_err(|e| format!("Failed to create RuVector client: {}", e))?;
        let ruvector = Arc::new(ruvector);

        let agent = DecisionMemoryAgent::new(ruvector.clone(), agent_config);

        Ok(Self { agent, ruvector })
    }

    /// Create a handler from environment variables
    pub fn from_env() -> Result<Self, String> {
        Self::new(RuVectorConfig::default(), AgentConfig::default())
    }

    /// Handle capture request
    #[instrument(skip(self, request))]
    pub async fn handle_capture(
        &self,
        request: CaptureRequest,
    ) -> Result<warp::reply::Json, warp::Rejection> {
        info!("Handling capture request");

        match self.agent.capture(request.input).await {
            Ok(event) => {
                let response = ApiResponse::success(event.outputs.clone(), event.execution_ref);
                Ok(warp::reply::json(&response))
            }
            Err(err) => {
                error!(?err, "Capture failed");
                let response: ApiResponse<()> = ApiResponse::error(&err);
                Ok(warp::reply::json(&response))
            }
        }
    }

    /// Handle retrieve request
    #[instrument(skip(self))]
    pub async fn handle_retrieve(
        &self,
        request: RetrieveRequest,
    ) -> Result<warp::reply::Json, warp::Rejection> {
        info!(execution_ref = %request.execution_ref, "Handling retrieve request");

        match self
            .ruvector
            .retrieve_decision_event(&request.execution_ref)
            .await
        {
            Ok(response) => {
                let api_response =
                    ApiResponse::success(response.data.clone(), response.data.execution_ref);
                Ok(warp::reply::json(&api_response))
            }
            Err(err) => {
                error!(?err, "Retrieve failed");
                let response: ApiResponse<()> = ApiResponse::error(&err);
                Ok(warp::reply::json(&response))
            }
        }
    }

    /// Handle inspect (query) request
    #[instrument(skip(self))]
    pub async fn handle_inspect(
        &self,
        request: InspectRequest,
    ) -> Result<warp::reply::Json, warp::Rejection> {
        info!(?request.query, "Handling inspect request");

        match self.ruvector.query_decision_events(&request.query).await {
            Ok(events) => {
                let execution_ref = Uuid::new_v4();
                let api_response = ApiResponse::success(events, execution_ref);
                Ok(warp::reply::json(&api_response))
            }
            Err(err) => {
                error!(?err, "Inspect failed");
                let response: ApiResponse<()> = ApiResponse::error(&err);
                Ok(warp::reply::json(&response))
            }
        }
    }

    /// Handle replay request
    #[instrument(skip(self))]
    pub async fn handle_replay(
        &self,
        request: ReplayRequest,
    ) -> Result<warp::reply::Json, warp::Rejection> {
        info!(execution_ref = %request.execution_ref, "Handling replay request");

        // First, retrieve the original event
        let original = match self
            .ruvector
            .retrieve_decision_event(&request.execution_ref)
            .await
        {
            Ok(response) => response.data,
            Err(err) => {
                error!(?err, "Failed to retrieve original event for replay");
                let response: ApiResponse<()> = ApiResponse::error(&err);
                return Ok(warp::reply::json(&response));
            }
        };

        // Apply modifications if provided
        let input = if let Some(modifications) = request.modifications {
            // Merge modifications into the original input
            let mut input_json = serde_json::to_value(&original.input)
                .map_err(|_| warp::reject::reject())?;
            if let (serde_json::Value::Object(ref mut input_map), serde_json::Value::Object(mods)) =
                (&mut input_json, modifications)
            {
                for (key, value) in mods {
                    input_map.insert(key, value);
                }
            }
            serde_json::from_value(input_json).map_err(|_| warp::reject::reject())?
        } else {
            original.input
        };

        // Re-capture with the (modified) input
        match self.agent.capture(input).await {
            Ok(event) => {
                let response = ApiResponse::success(event.outputs.clone(), event.execution_ref);
                Ok(warp::reply::json(&response))
            }
            Err(err) => {
                error!(?err, "Replay capture failed");
                let response: ApiResponse<()> = ApiResponse::error(&err);
                Ok(warp::reply::json(&response))
            }
        }
    }

    /// Handle health check
    pub async fn handle_health() -> Result<warp::reply::Json, warp::Rejection> {
        let health = serde_json::json!({
            "status": "healthy",
            "agent_id": crate::constants::AGENT_ID,
            "agent_version": crate::constants::AGENT_VERSION,
            "classification": crate::constants::CLASSIFICATION,
        });
        Ok(warp::reply::json(&health))
    }

    /// Create warp routes for the handler
    pub fn routes(
        handler: Arc<Self>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let health = warp::path("health")
            .and(warp::get())
            .and_then(Self::handle_health);

        let capture = {
            let handler = handler.clone();
            warp::path("capture")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |request: CaptureRequest| {
                    let handler = handler.clone();
                    async move { handler.handle_capture(request).await }
                })
        };

        let retrieve = {
            let handler = handler.clone();
            warp::path("retrieve")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |request: RetrieveRequest| {
                    let handler = handler.clone();
                    async move { handler.handle_retrieve(request).await }
                })
        };

        let inspect = {
            let handler = handler.clone();
            warp::path("inspect")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |request: InspectRequest| {
                    let handler = handler.clone();
                    async move { handler.handle_inspect(request).await }
                })
        };

        let replay = {
            let handler = handler.clone();
            warp::path("replay")
                .and(warp::post())
                .and(warp::body::json())
                .and_then(move |request: ReplayRequest| {
                    let handler = handler.clone();
                    async move { handler.handle_replay(request).await }
                })
        };

        warp::path("api")
            .and(warp::path("v1"))
            .and(health.or(capture).or(retrieve).or(inspect).or(replay))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_serialization() {
        let response = ApiResponse::success("test data".to_string(), Uuid::new_v4());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test data"));
        assert!(json.contains("execution_ref"));
    }

    #[test]
    fn test_capture_request_deserialization() {
        let json = r#"{
            "input": {
                "decision_id": "550e8400-e29b-41d4-a716-446655440000",
                "decision_type": "test",
                "context": {
                    "session_id": "550e8400-e29b-41d4-a716-446655440001"
                }
            }
        }"#;

        let request: CaptureRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.input.decision_type, "test");
    }
}
