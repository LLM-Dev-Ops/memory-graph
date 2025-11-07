//! Hook execution framework
//!
//! This module provides infrastructure for managing and executing plugin hooks
//! at specific points in the system's execution flow.

use super::{Plugin, PluginContext, PluginError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Hook point identifier
///
/// Represents specific points in the execution flow where plugins can be invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookPoint {
    /// Before creating a node
    BeforeCreateNode,
    /// After creating a node
    AfterCreateNode,
    /// Before creating a session
    BeforeCreateSession,
    /// After creating a session
    AfterCreateSession,
    /// Before executing a query
    BeforeQuery,
    /// After executing a query
    AfterQuery,
    /// Before creating an edge
    BeforeCreateEdge,
    /// After creating an edge
    AfterCreateEdge,
    /// Before updating a node
    BeforeUpdateNode,
    /// After updating a node
    AfterUpdateNode,
    /// Before deleting a node
    BeforeDeleteNode,
    /// After deleting a node
    AfterDeleteNode,
    /// Before deleting a session
    BeforeDeleteSession,
    /// After deleting a session
    AfterDeleteSession,
}

impl HookPoint {
    /// Get the hook name as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BeforeCreateNode => "before_create_node",
            Self::AfterCreateNode => "after_create_node",
            Self::BeforeCreateSession => "before_create_session",
            Self::AfterCreateSession => "after_create_session",
            Self::BeforeQuery => "before_query",
            Self::AfterQuery => "after_query",
            Self::BeforeCreateEdge => "before_create_edge",
            Self::AfterCreateEdge => "after_create_edge",
            Self::BeforeUpdateNode => "before_update_node",
            Self::AfterUpdateNode => "after_update_node",
            Self::BeforeDeleteNode => "before_delete_node",
            Self::AfterDeleteNode => "after_delete_node",
            Self::BeforeDeleteSession => "before_delete_session",
            Self::AfterDeleteSession => "after_delete_session",
        }
    }

    /// Check if this is a "before" hook
    pub fn is_before(&self) -> bool {
        matches!(
            self,
            Self::BeforeCreateNode
                | Self::BeforeCreateSession
                | Self::BeforeQuery
                | Self::BeforeCreateEdge
                | Self::BeforeUpdateNode
                | Self::BeforeDeleteNode
                | Self::BeforeDeleteSession
        )
    }

    /// Check if this is an "after" hook
    pub fn is_after(&self) -> bool {
        !self.is_before()
    }

    /// Get all hook points
    pub fn all() -> Vec<Self> {
        vec![
            Self::BeforeCreateNode,
            Self::AfterCreateNode,
            Self::BeforeCreateSession,
            Self::AfterCreateSession,
            Self::BeforeQuery,
            Self::AfterQuery,
            Self::BeforeCreateEdge,
            Self::AfterCreateEdge,
            Self::BeforeUpdateNode,
            Self::AfterUpdateNode,
            Self::BeforeDeleteNode,
            Self::AfterDeleteNode,
            Self::BeforeDeleteSession,
            Self::AfterDeleteSession,
        ]
    }
}

impl std::fmt::Display for HookPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Hook registry
///
/// Maintains mappings between hook points and the plugins that should be
/// invoked at those points. This allows for efficient hook execution.
pub struct HookRegistry {
    hooks: HashMap<HookPoint, Vec<Arc<dyn Plugin>>>,
}

impl HookRegistry {
    /// Create a new hook registry
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    /// Register a plugin for a specific hook point
    pub fn register_hook(&mut self, hook: HookPoint, plugin: Arc<dyn Plugin>) {
        self.hooks.entry(hook).or_default().push(plugin);
    }

    /// Unregister a plugin from a specific hook point
    pub fn unregister_hook(&mut self, hook: HookPoint, plugin_name: &str) {
        if let Some(plugins) = self.hooks.get_mut(&hook) {
            plugins.retain(|p| p.metadata().name != plugin_name);
        }
    }

    /// Unregister a plugin from all hook points
    pub fn unregister_plugin(&mut self, plugin_name: &str) {
        for plugins in self.hooks.values_mut() {
            plugins.retain(|p| p.metadata().name != plugin_name);
        }
    }

    /// Get all plugins registered for a hook point
    pub fn get_plugins(&self, hook: HookPoint) -> Vec<Arc<dyn Plugin>> {
        self.hooks.get(&hook).cloned().unwrap_or_default()
    }

    /// Get the number of plugins registered for a hook point
    pub fn count_plugins(&self, hook: HookPoint) -> usize {
        self.hooks.get(&hook).map(Vec::len).unwrap_or(0)
    }

    /// Clear all hook registrations
    pub fn clear(&mut self) {
        self.hooks.clear();
    }

    /// Get statistics about hook registrations
    pub fn stats(&self) -> HashMap<HookPoint, usize> {
        self.hooks
            .iter()
            .map(|(hook, plugins)| (*hook, plugins.len()))
            .collect()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook executor
///
/// Responsible for executing plugin hooks at specific points in the system.
/// Handles error propagation, logging, and execution order.
pub struct HookExecutor {
    /// Whether to stop execution on first error (before hooks only)
    fail_fast: bool,
    /// Whether to collect timing metrics
    collect_metrics: bool,
}

impl HookExecutor {
    /// Create a new hook executor
    pub fn new() -> Self {
        Self {
            fail_fast: true,
            collect_metrics: false,
        }
    }

    /// Create a hook executor with fail-fast disabled
    ///
    /// When fail-fast is disabled, all plugins will be executed even if
    /// some fail. Useful for after-hooks where you want to ensure all
    /// plugins get a chance to run.
    pub fn without_fail_fast() -> Self {
        Self {
            fail_fast: false,
            collect_metrics: false,
        }
    }

    /// Enable metrics collection
    pub fn with_metrics(mut self) -> Self {
        self.collect_metrics = true;
        self
    }

    /// Execute before hooks
    ///
    /// Executes all plugins at the specified hook point. If fail_fast is enabled,
    /// stops at the first error. Otherwise, collects all errors and returns them.
    pub async fn execute_before(
        &self,
        hook: HookPoint,
        plugins: &[Arc<dyn Plugin>],
        context: &PluginContext,
    ) -> Result<(), PluginError> {
        debug!("Executing {} with {} plugins", hook, plugins.len());

        let mut errors = Vec::new();

        for plugin in plugins {
            let plugin_name = &plugin.metadata().name;
            debug!("Executing hook {} for plugin {}", hook, plugin_name);

            let start = std::time::Instant::now();

            match plugin.before_hook(hook.as_str(), context).await {
                Ok(()) => {
                    if self.collect_metrics {
                        let duration = start.elapsed();
                        debug!(
                            "Plugin {} completed {} in {:?}",
                            plugin_name, hook, duration
                        );
                    }
                }
                Err(e) => {
                    warn!("Plugin {} failed on {}: {}", plugin_name, hook, e);

                    if self.fail_fast {
                        return Err(e);
                    }
                    errors.push((plugin_name.clone(), e));
                }
            }
        }

        if !errors.is_empty() {
            let error_msg = errors
                .iter()
                .map(|(name, e)| format!("{}: {}", name, e))
                .collect::<Vec<_>>()
                .join("; ");

            return Err(PluginError::HookFailed(format!(
                "Multiple plugins failed: {}",
                error_msg
            )));
        }

        Ok(())
    }

    /// Execute after hooks
    ///
    /// Executes all plugins at the specified hook point. After hooks never
    /// fail the operation - errors are logged but execution continues.
    pub async fn execute_after(
        &self,
        hook: HookPoint,
        plugins: &[Arc<dyn Plugin>],
        context: &PluginContext,
    ) -> Result<(), PluginError> {
        debug!("Executing {} with {} plugins", hook, plugins.len());

        for plugin in plugins {
            let plugin_name = &plugin.metadata().name;
            debug!("Executing hook {} for plugin {}", hook, plugin_name);

            let start = std::time::Instant::now();

            match plugin.after_hook(hook.as_str(), context).await {
                Ok(()) => {
                    if self.collect_metrics {
                        let duration = start.elapsed();
                        debug!(
                            "Plugin {} completed {} in {:?}",
                            plugin_name, hook, duration
                        );
                    }
                }
                Err(e) => {
                    // After hooks should not fail the operation
                    warn!(
                        "Plugin {} failed on after hook {}: {}",
                        plugin_name, hook, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Execute a hook point
    ///
    /// Automatically determines whether to use before or after hook semantics
    /// based on the hook point.
    pub async fn execute(
        &self,
        hook: HookPoint,
        plugins: &[Arc<dyn Plugin>],
        context: &PluginContext,
    ) -> Result<(), PluginError> {
        if hook.is_before() {
            self.execute_before(hook, plugins, context).await
        } else {
            self.execute_after(hook, plugins, context).await
        }
    }
}

impl Default for HookExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook execution result with timing information
#[derive(Debug)]
pub struct HookExecutionResult {
    /// Hook point that was executed
    pub hook: HookPoint,
    /// Number of plugins executed
    pub plugins_executed: usize,
    /// Total execution time
    pub total_duration: std::time::Duration,
    /// Individual plugin execution times
    pub plugin_durations: HashMap<String, std::time::Duration>,
    /// Any errors that occurred
    pub errors: Vec<(String, String)>,
}

impl HookExecutionResult {
    /// Check if the execution was successful
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the average execution time per plugin
    pub fn average_duration(&self) -> std::time::Duration {
        if self.plugins_executed == 0 {
            return std::time::Duration::ZERO;
        }
        self.total_duration / self.plugins_executed as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{PluginBuilder, PluginMetadata};
    use async_trait::async_trait;

    struct MockPlugin {
        metadata: PluginMetadata,
        should_fail: bool,
    }

    impl MockPlugin {
        fn new(name: &str, should_fail: bool) -> Self {
            let metadata = PluginBuilder::new(name, "1.0.0")
                .author("Test")
                .description("Test plugin")
                .build();
            Self {
                metadata,
                should_fail,
            }
        }
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn before_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
            if self.should_fail {
                Err(PluginError::HookFailed("Test failure".to_string()))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_hook_point_as_str() {
        assert_eq!(HookPoint::BeforeCreateNode.as_str(), "before_create_node");
        assert_eq!(HookPoint::AfterCreateNode.as_str(), "after_create_node");
    }

    #[test]
    fn test_hook_point_is_before() {
        assert!(HookPoint::BeforeCreateNode.is_before());
        assert!(!HookPoint::AfterCreateNode.is_before());
    }

    #[test]
    fn test_hook_registry() {
        let mut registry = HookRegistry::new();
        let plugin: Arc<dyn Plugin> = Arc::new(MockPlugin::new("test", false));

        registry.register_hook(HookPoint::BeforeCreateNode, Arc::clone(&plugin));
        assert_eq!(registry.count_plugins(HookPoint::BeforeCreateNode), 1);

        let plugins = registry.get_plugins(HookPoint::BeforeCreateNode);
        assert_eq!(plugins.len(), 1);

        registry.unregister_hook(HookPoint::BeforeCreateNode, "test");
        assert_eq!(registry.count_plugins(HookPoint::BeforeCreateNode), 0);
    }

    #[tokio::test]
    async fn test_hook_executor_success() {
        let executor = HookExecutor::new();
        let plugins: Vec<Arc<dyn Plugin>> = vec![
            Arc::new(MockPlugin::new("plugin1", false)),
            Arc::new(MockPlugin::new("plugin2", false)),
        ];

        let context = PluginContext::new("test", serde_json::json!({}));

        let result = executor
            .execute_before(HookPoint::BeforeCreateNode, &plugins, &context)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hook_executor_fail_fast() {
        let executor = HookExecutor::new();
        let plugins: Vec<Arc<dyn Plugin>> = vec![
            Arc::new(MockPlugin::new("plugin1", false)),
            Arc::new(MockPlugin::new("plugin2", true)),
            Arc::new(MockPlugin::new("plugin3", false)),
        ];

        let context = PluginContext::new("test", serde_json::json!({}));

        let result = executor
            .execute_before(HookPoint::BeforeCreateNode, &plugins, &context)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_hook_executor_without_fail_fast() {
        let executor = HookExecutor::without_fail_fast();
        let plugins: Vec<Arc<dyn Plugin>> = vec![
            Arc::new(MockPlugin::new("plugin1", false)),
            Arc::new(MockPlugin::new("plugin2", true)),
            Arc::new(MockPlugin::new("plugin3", false)),
        ];

        let context = PluginContext::new("test", serde_json::json!({}));

        let result = executor
            .execute_before(HookPoint::BeforeCreateNode, &plugins, &context)
            .await;

        // Should fail but only after executing all plugins
        assert!(result.is_err());
    }
}
