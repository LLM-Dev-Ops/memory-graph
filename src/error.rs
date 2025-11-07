//! Error types for LLM-Memory-Graph operations

/// Result type alias for LLM-Memory-Graph operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur during graph operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Storage backend error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Node not found error
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Session not found error
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Invalid node type error
    #[error("Invalid node type: expected {expected}, got {actual}")]
    InvalidNodeType {
        /// Expected node type
        expected: String,
        /// Actual node type encountered
        actual: String,
    },

    /// Schema validation error
    #[error("Schema validation error: {0}")]
    ValidationError(String),

    /// Graph traversal error
    #[error("Graph traversal error: {0}")]
    TraversalError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Async runtime error
    #[error("Runtime error: {0}")]
    RuntimeError(String),

    /// Async operation timeout
    #[error("Operation timed out after {0}ms")]
    Timeout(u64),

    /// Concurrent modification conflict
    #[error("Concurrent modification detected: {0}")]
    ConcurrentModification(String),

    /// Connection pool exhausted
    #[error("Connection pool exhausted")]
    PoolExhausted,

    /// Metrics error
    #[error("Metrics error: {0}")]
    Metrics(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Self {
        Error::Storage(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<rmp_serde::encode::Error> for Error {
    fn from(err: rmp_serde::encode::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<rmp_serde::decode::Error> for Error {
    fn from(err: rmp_serde::decode::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<prometheus::Error> for Error {
    fn from(err: prometheus::Error) -> Self {
        Error::Metrics(err.to_string())
    }
}

impl Error {
    /// Create a timeout error with the specified duration in milliseconds
    pub fn timeout(duration_ms: u64) -> Self {
        Error::Timeout(duration_ms)
    }

    /// Create a concurrent modification error with context
    pub fn concurrent_modification<S: Into<String>>(context: S) -> Self {
        Error::ConcurrentModification(context.into())
    }

    /// Create a pool exhausted error
    pub fn pool_exhausted() -> Self {
        Error::PoolExhausted
    }

    /// Check if this error is a timeout
    pub fn is_timeout(&self) -> bool {
        matches!(self, Error::Timeout(_))
    }

    /// Check if this error is a concurrent modification
    pub fn is_concurrent_modification(&self) -> bool {
        matches!(self, Error::ConcurrentModification(_))
    }

    /// Check if this error is pool exhausted
    pub fn is_pool_exhausted(&self) -> bool {
        matches!(self, Error::PoolExhausted)
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Timeout(_) | Error::PoolExhausted | Error::ConcurrentModification(_)
        )
    }
}

/// Extension trait for adding context to Results in async operations
pub trait ResultExt<T> {
    /// Add context to an error
    fn context<S: Into<String>>(self, context: S) -> Result<T>;

    /// Add context to an error using a closure (lazy evaluation)
    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| Error::Other(format!("{}: {}", context.into(), e)))
    }

    fn with_context<F, S>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> S,
        S: Into<String>,
    {
        self.map_err(|e| Error::Other(format!("{}: {}", f().into(), e)))
    }
}

/// Async timeout utilities
pub mod timeout {
    use super::{Error, Result};
    use std::future::Future;
    use std::time::Duration;

    /// Execute an async operation with a timeout
    ///
    /// Returns `Error::Timeout` if the operation doesn't complete within the specified duration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::error::timeout::with_timeout;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = with_timeout(
    ///     Duration::from_secs(5),
    ///     async {
    ///         // Your async operation here
    ///         Ok::<_, llm_memory_graph::error::Error>(42)
    ///     }
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        match tokio::time::timeout(duration, future).await {
            Ok(result) => result,
            Err(_) => Err(Error::timeout(duration.as_millis() as u64)),
        }
    }

    /// Execute an async operation with a timeout in milliseconds
    pub async fn with_timeout_ms<F, T>(timeout_ms: u64, future: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        with_timeout(Duration::from_millis(timeout_ms), future).await
    }

    /// Execute an async operation with a timeout in seconds
    pub async fn with_timeout_secs<F, T>(timeout_secs: u64, future: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        with_timeout(Duration::from_secs(timeout_secs), future).await
    }
}

/// Retry utilities for handling transient async errors
pub mod retry {
    use super::Result;
    use std::future::Future;
    use std::time::Duration;

    /// Retry configuration
    #[derive(Debug, Clone)]
    pub struct RetryConfig {
        /// Maximum number of retry attempts
        pub max_attempts: usize,
        /// Initial delay between retries
        pub initial_delay: Duration,
        /// Maximum delay between retries
        pub max_delay: Duration,
        /// Backoff multiplier (exponential backoff)
        pub backoff_multiplier: f64,
    }

    impl Default for RetryConfig {
        fn default() -> Self {
            Self {
                max_attempts: 3,
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(5),
                backoff_multiplier: 2.0,
            }
        }
    }

    impl RetryConfig {
        /// Create a new retry configuration
        pub fn new() -> Self {
            Self::default()
        }

        /// Set maximum number of retry attempts
        pub fn with_max_attempts(mut self, max_attempts: usize) -> Self {
            self.max_attempts = max_attempts;
            self
        }

        /// Set initial delay between retries
        pub fn with_initial_delay(mut self, delay: Duration) -> Self {
            self.initial_delay = delay;
            self
        }

        /// Set maximum delay between retries
        pub fn with_max_delay(mut self, delay: Duration) -> Self {
            self.max_delay = delay;
            self
        }

        /// Set backoff multiplier
        pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
            self.backoff_multiplier = multiplier;
            self
        }
    }

    /// Execute an async operation with retry logic
    ///
    /// Retries the operation if it returns a retryable error (timeout, pool exhausted, or concurrent modification).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::error::retry::{with_retry, RetryConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = RetryConfig::new().with_max_attempts(5);
    /// let result = with_retry(config, || async {
    ///     // Your async operation here
    ///     Ok::<_, llm_memory_graph::error::Error>(42)
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_retry<F, Fut, T>(config: RetryConfig, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = config.initial_delay;

        loop {
            attempt += 1;
            match operation().await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    // Only retry if the error is retryable and we haven't exceeded max attempts
                    if !err.is_retryable() || attempt >= config.max_attempts {
                        return Err(err);
                    }

                    // Wait before retrying with exponential backoff
                    tokio::time::sleep(delay).await;

                    // Calculate next delay with exponential backoff
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * config.backoff_multiplier) as u64,
                        ),
                        config.max_delay,
                    );
                }
            }
        }
    }

    /// Execute an async operation with retry using default configuration
    pub async fn with_default_retry<F, Fut, T>(operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        with_retry(RetryConfig::default(), operation).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::retry::RetryConfig;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_timeout_error_creation() {
        let err = Error::timeout(5000);
        assert!(err.is_timeout());
        assert!(err.is_retryable());
        assert_eq!(format!("{}", err), "Operation timed out after 5000ms");
    }

    #[test]
    fn test_concurrent_modification_error() {
        let err = Error::concurrent_modification("node was modified by another operation");
        assert!(err.is_concurrent_modification());
        assert!(err.is_retryable());
        assert_eq!(
            format!("{}", err),
            "Concurrent modification detected: node was modified by another operation"
        );
    }

    #[test]
    fn test_pool_exhausted_error() {
        let err = Error::pool_exhausted();
        assert!(err.is_pool_exhausted());
        assert!(err.is_retryable());
        assert_eq!(format!("{}", err), "Connection pool exhausted");
    }

    #[test]
    fn test_error_type_checks() {
        let timeout = Error::timeout(1000);
        let concurrent = Error::concurrent_modification("test");
        let pool = Error::pool_exhausted();
        let storage = Error::Storage("test".to_string());

        assert!(timeout.is_timeout());
        assert!(!timeout.is_concurrent_modification());
        assert!(!timeout.is_pool_exhausted());

        assert!(!concurrent.is_timeout());
        assert!(concurrent.is_concurrent_modification());
        assert!(!concurrent.is_pool_exhausted());

        assert!(!pool.is_timeout());
        assert!(!pool.is_concurrent_modification());
        assert!(pool.is_pool_exhausted());

        assert!(!storage.is_timeout());
        assert!(!storage.is_concurrent_modification());
        assert!(!storage.is_pool_exhausted());
    }

    #[test]
    fn test_retryable_errors() {
        assert!(Error::timeout(1000).is_retryable());
        assert!(Error::concurrent_modification("test").is_retryable());
        assert!(Error::pool_exhausted().is_retryable());

        assert!(!Error::Storage("test".to_string()).is_retryable());
        assert!(!Error::NodeNotFound("test".to_string()).is_retryable());
        assert!(!Error::Serialization("test".to_string()).is_retryable());
    }

    #[test]
    fn test_result_context() {
        let result: Result<i32> = Err(Error::Storage("disk full".to_string()));
        let with_context = result.context("Failed to save node");

        assert!(with_context.is_err());
        let err = with_context.unwrap_err();
        assert!(format!("{}", err).contains("Failed to save node"));
        assert!(format!("{}", err).contains("disk full"));
    }

    #[test]
    fn test_result_with_context_lazy() {
        let result: Result<i32> = Err(Error::Storage("connection lost".to_string()));
        let with_context = result.with_context(|| format!("Operation failed at {}", 42));

        assert!(with_context.is_err());
        let err = with_context.unwrap_err();
        assert!(format!("{}", err).contains("Operation failed at 42"));
        assert!(format!("{}", err).contains("connection lost"));
    }

    #[tokio::test]
    async fn test_timeout_with_fast_operation() {
        use super::timeout::with_timeout_ms;

        let result = with_timeout_ms(1000, async {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            Ok::<_, Error>(42)
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_timeout_with_slow_operation() {
        use super::timeout::with_timeout_ms;

        let result = with_timeout_ms(100, async {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            Ok::<_, Error>(42)
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_timeout());
    }

    #[tokio::test]
    async fn test_timeout_with_error() {
        use super::timeout::with_timeout_ms;

        let result = with_timeout_ms(1000, async {
            Err::<i32, Error>(Error::Storage("test error".to_string()))
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_timeout());
        assert!(matches!(err, Error::Storage(_)));
    }

    #[tokio::test]
    async fn test_timeout_secs() {
        use super::timeout::with_timeout_secs;

        let result = with_timeout_secs(1, async {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            Ok::<_, Error>(100)
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        use super::retry::{with_retry, RetryConfig};

        let config = RetryConfig::new().with_max_attempts(3);
        let result = with_retry(config, || async { Ok::<_, Error>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        use super::retry::{with_retry, RetryConfig};

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let config = RetryConfig::new()
            .with_max_attempts(5)
            .with_initial_delay(std::time::Duration::from_millis(10));

        let result = with_retry(config, || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(Error::timeout(100))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_fails_after_max_attempts() {
        use super::retry::{with_retry, RetryConfig};

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let config = RetryConfig::new()
            .with_max_attempts(3)
            .with_initial_delay(std::time::Duration::from_millis(10));

        let result = with_retry(config, || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<i32, Error>(Error::pool_exhausted())
            }
        })
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().is_pool_exhausted());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_no_retry_for_non_retryable_error() {
        use super::retry::{with_retry, RetryConfig};

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let config = RetryConfig::new().with_max_attempts(5);

        let result = with_retry(config, || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<i32, Error>(Error::Storage("permanent error".to_string()))
            }
        })
        .await;

        assert!(result.is_err());
        // Should only try once since storage errors are not retryable
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_with_concurrent_modification() {
        use super::retry::{with_retry, RetryConfig};

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let config = RetryConfig::new()
            .with_max_attempts(4)
            .with_initial_delay(std::time::Duration::from_millis(10));

        let result = with_retry(config, || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(Error::concurrent_modification("version mismatch"))
                } else {
                    Ok(100)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exponential_backoff() {
        use super::retry::{with_retry, RetryConfig};
        use std::time::Instant;

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let config = RetryConfig::new()
            .with_max_attempts(3)
            .with_initial_delay(std::time::Duration::from_millis(50))
            .with_backoff_multiplier(2.0);

        let start = Instant::now();
        let result = with_retry(config, || {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<i32, Error>(Error::timeout(100))
            }
        })
        .await;

        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
        // Should take at least 50ms + 100ms = 150ms (first delay + second delay)
        assert!(elapsed.as_millis() >= 150);
    }

    #[tokio::test]
    async fn test_default_retry() {
        use super::retry::with_default_retry;

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let result = with_default_retry(|| {
            let count = Arc::clone(&attempt_count_clone);
            async move {
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(Error::pool_exhausted())
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new()
            .with_max_attempts(10)
            .with_initial_delay(std::time::Duration::from_millis(200))
            .with_max_delay(std::time::Duration::from_secs(30))
            .with_backoff_multiplier(3.0);

        assert_eq!(config.max_attempts, 10);
        assert_eq!(config.initial_delay, std::time::Duration::from_millis(200));
        assert_eq!(config.max_delay, std::time::Duration::from_secs(30));
        assert_eq!(config.backoff_multiplier, 3.0);
    }
}
