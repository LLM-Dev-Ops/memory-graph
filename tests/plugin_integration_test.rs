//! Integration tests for the plugin system
//!
//! Tests plugin registration, lifecycle management, hook execution,
//! and interaction with the core memory graph functionality.

use llm_memory_graph::plugin::{
    Plugin, PluginBuilder, PluginContext, PluginError, PluginManager, PluginMetadata, PluginRegistry,
    HookExecutor, HookPoint,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

// Test plugin that tracks hook calls
struct CountingPlugin {
    metadata: PluginMetadata,
    before_create_node_count: Arc<AtomicUsize>,
    after_create_node_count: Arc<AtomicUsize>,
}

impl CountingPlugin {
    fn new(name: &str) -> Self {
        let metadata = PluginBuilder::new(name, "1.0.0")
            .author("Test")
            .description("Counting plugin for tests")
            .capability("testing")
            .build();

        Self {
            metadata,
            before_create_node_count: Arc::new(AtomicUsize::new(0)),
            after_create_node_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn get_before_count(&self) -> usize {
        self.before_create_node_count.load(Ordering::Relaxed)
    }

    fn get_after_count(&self) -> usize {
        self.after_create_node_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl Plugin for CountingPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn before_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
        self.before_create_node_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn after_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
        self.after_create_node_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

// Test plugin that fails on demand
struct FailingPlugin {
    metadata: PluginMetadata,
    should_fail: Arc<AtomicUsize>,
}

impl FailingPlugin {
    fn new(name: &str) -> Self {
        let metadata = PluginBuilder::new(name, "1.0.0")
            .author("Test")
            .description("Failing plugin for tests")
            .capability("testing")
            .build();

        Self {
            metadata,
            should_fail: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn set_fail(&self, should_fail: bool) {
        self.should_fail.store(should_fail as usize, Ordering::Relaxed);
    }
}

#[async_trait]
impl Plugin for FailingPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn before_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
        if self.should_fail.load(Ordering::Relaxed) != 0 {
            Err(PluginError::HookFailed("Intentional failure".to_string()))
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// Plugin Manager Tests
// ============================================================================

#[tokio::test]
async fn test_plugin_registration() {
    let mut manager = PluginManager::new();
    let plugin: Arc<dyn Plugin> = Arc::new(CountingPlugin::new("test_plugin"));

    assert!(manager.register(Arc::clone(&plugin)).await.is_ok());
    assert!(manager.get_state("test_plugin").is_some());
}

#[tokio::test]
async fn test_plugin_duplicate_registration() {
    let mut manager = PluginManager::new();
    let plugin1: Arc<dyn Plugin> = Arc::new(CountingPlugin::new("test_plugin"));
    let plugin2: Arc<dyn Plugin> = Arc::new(CountingPlugin::new("test_plugin"));

    assert!(manager.register(plugin1).await.is_ok());
    assert!(manager.register(plugin2).await.is_err());
}

#[tokio::test]
async fn test_plugin_lifecycle() {
    let mut manager = PluginManager::new();
    let plugin: Arc<dyn Plugin> = Arc::new(CountingPlugin::new("test_plugin"));

    manager.register(Arc::clone(&plugin)).await.unwrap();
    assert!(manager.get_state("test_plugin").is_some());

    manager.initialize("test_plugin").await.unwrap();
    manager.enable("test_plugin").unwrap();
    assert!(manager.is_enabled("test_plugin"));

    manager.disable("test_plugin").unwrap();
    assert!(!manager.is_enabled("test_plugin"));
}

#[tokio::test]
async fn test_multiple_plugins() {
    let mut manager = PluginManager::new();
    let plugin1: Arc<dyn Plugin> = Arc::new(CountingPlugin::new("plugin1"));
    let plugin2: Arc<dyn Plugin> = Arc::new(CountingPlugin::new("plugin2"));

    manager.register(Arc::clone(&plugin1)).await.unwrap();
    manager.register(Arc::clone(&plugin2)).await.unwrap();

    manager.init_all().await.unwrap();
    manager.enable_all().unwrap();

    let active = manager.active_plugins();
    assert_eq!(active.len(), 2);
}

#[tokio::test]
async fn test_hook_execution() {
    let mut manager = PluginManager::new();
    let counter_plugin = Arc::new(CountingPlugin::new("test_plugin"));
    let plugin: Arc<dyn Plugin> = Arc::clone(&counter_plugin) as Arc<dyn Plugin>;

    manager.register(Arc::clone(&plugin)).await.unwrap();
    manager.initialize("test_plugin").await.unwrap();
    manager.enable("test_plugin").unwrap();

    let context = PluginContext::new("test", json!({"content": "test"}));

    manager
        .execute_before_hooks("before_create_node", &context)
        .await
        .unwrap();

    assert_eq!(counter_plugin.get_before_count(), 1);
}

#[tokio::test]
async fn test_hook_execution_failure() {
    let mut manager = PluginManager::new();
    let failing_plugin = Arc::new(FailingPlugin::new("failing_plugin"));
    let plugin: Arc<dyn Plugin> = Arc::clone(&failing_plugin) as Arc<dyn Plugin>;

    manager.register(Arc::clone(&plugin)).await.unwrap();
    manager.initialize("failing_plugin").await.unwrap();
    manager.enable("failing_plugin").unwrap();

    failing_plugin.set_fail(true);

    let context = PluginContext::new("test", json!({"content": "test"}));

    let result = manager
        .execute_before_hooks("before_create_node", &context)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_shutdown_all() {
    let mut manager = PluginManager::new();
    let plugin1: Arc<dyn Plugin> = Arc::new(CountingPlugin::new("plugin1"));
    let plugin2: Arc<dyn Plugin> = Arc::new(CountingPlugin::new("plugin2"));

    manager.register(plugin1).await.unwrap();
    manager.register(plugin2).await.unwrap();

    manager.shutdown_all().await.unwrap();

    // After shutdown, no plugins should be available
    assert_eq!(manager.active_plugins().len(), 0);
}

// ============================================================================
// Plugin Registry Tests
// ============================================================================

#[test]
fn test_registry_registration() {
    let mut registry = PluginRegistry::new();
    let metadata = PluginBuilder::new("test_plugin", "1.0.0")
        .author("Test")
        .description("Test plugin")
        .capability("testing")
        .build();

    assert!(registry.register(metadata, None).is_ok());
    assert!(registry.contains("test_plugin"));
}

#[test]
fn test_registry_find_by_capability() {
    let mut registry = PluginRegistry::new();

    let metadata1 = PluginBuilder::new("plugin1", "1.0.0")
        .capability("validation")
        .build();

    let metadata2 = PluginBuilder::new("plugin2", "1.0.0")
        .capability("validation")
        .capability("enrichment")
        .build();

    let metadata3 = PluginBuilder::new("plugin3", "1.0.0")
        .capability("enrichment")
        .build();

    registry.register(metadata1, None).unwrap();
    registry.register(metadata2, None).unwrap();
    registry.register(metadata3, None).unwrap();

    let validation_plugins = registry.find_by_capability("validation");
    assert_eq!(validation_plugins.len(), 2);

    let enrichment_plugins = registry.find_by_capability("enrichment");
    assert_eq!(enrichment_plugins.len(), 2);
}

#[test]
fn test_registry_tags() {
    let mut registry = PluginRegistry::new();
    let metadata = PluginBuilder::new("test_plugin", "1.0.0").build();

    registry.register(metadata, None).unwrap();
    registry.add_tag("test_plugin", "production").unwrap();
    registry.add_tag("test_plugin", "critical").unwrap();

    let tagged = registry.find_by_tag("production");
    assert_eq!(tagged.len(), 1);

    registry.remove_tag("test_plugin", "production").unwrap();
    let tagged = registry.find_by_tag("production");
    assert_eq!(tagged.len(), 0);
}

#[test]
fn test_registry_stats() {
    let mut registry = PluginRegistry::new();

    let metadata1 = PluginBuilder::new("plugin1", "1.0.0")
        .capability("validation")
        .build();

    let metadata2 = PluginBuilder::new("plugin2", "1.0.0")
        .capability("validation")
        .capability("enrichment")
        .build();

    registry.register(metadata1, None).unwrap();
    registry.register(metadata2, None).unwrap();

    registry.mark_loaded("plugin1").unwrap();

    let stats = registry.stats();
    assert_eq!(stats.total_plugins, 2);
    assert_eq!(stats.loaded_plugins, 1);
    assert_eq!(stats.unloaded_plugins, 1);
    assert_eq!(*stats.capabilities.get("validation").unwrap(), 2);
    assert_eq!(*stats.capabilities.get("enrichment").unwrap(), 1);
}

// ============================================================================
// Hook Executor Tests
// ============================================================================

#[tokio::test]
async fn test_hook_executor() {
    let executor = HookExecutor::new();
    let counter_plugin = Arc::new(CountingPlugin::new("test_plugin"));
    let plugins: Vec<Arc<dyn Plugin>> = vec![Arc::clone(&counter_plugin) as Arc<dyn Plugin>];

    let context = PluginContext::new("test", json!({}));

    executor
        .execute_before(HookPoint::BeforeCreateNode, &plugins, &context)
        .await
        .unwrap();

    assert_eq!(counter_plugin.get_before_count(), 1);
}

#[tokio::test]
async fn test_hook_executor_fail_fast() {
    let executor = HookExecutor::new();
    let plugin1 = Arc::new(CountingPlugin::new("plugin1"));
    let failing_plugin = Arc::new(FailingPlugin::new("plugin2"));
    let plugin3 = Arc::new(CountingPlugin::new("plugin3"));

    failing_plugin.set_fail(true);

    let plugins: Vec<Arc<dyn Plugin>> = vec![
        Arc::clone(&plugin1) as Arc<dyn Plugin>,
        Arc::clone(&failing_plugin) as Arc<dyn Plugin>,
        Arc::clone(&plugin3) as Arc<dyn Plugin>,
    ];

    let context = PluginContext::new("test", json!({}));

    let result = executor
        .execute_before(HookPoint::BeforeCreateNode, &plugins, &context)
        .await;

    assert!(result.is_err());

    // First plugin should have executed
    assert_eq!(plugin1.get_before_count(), 1);

    // Third plugin should NOT have executed (fail-fast)
    assert_eq!(plugin3.get_before_count(), 0);
}

#[tokio::test]
async fn test_hook_executor_after_hooks() {
    let executor = HookExecutor::new();
    let plugin1 = Arc::new(CountingPlugin::new("plugin1"));
    let failing_plugin = Arc::new(FailingPlugin::new("plugin2"));
    let plugin3 = Arc::new(CountingPlugin::new("plugin3"));

    failing_plugin.set_fail(true);

    let plugins: Vec<Arc<dyn Plugin>> = vec![
        Arc::clone(&plugin1) as Arc<dyn Plugin>,
        Arc::clone(&failing_plugin) as Arc<dyn Plugin>,
        Arc::clone(&plugin3) as Arc<dyn Plugin>,
    ];

    let context = PluginContext::new("test", json!({}));

    // After hooks should not fail even if a plugin fails
    let result = executor
        .execute_after(HookPoint::AfterCreateNode, &plugins, &context)
        .await;

    assert!(result.is_ok());

    // All plugins should have been called
    assert_eq!(plugin1.get_after_count(), 1);
    assert_eq!(plugin3.get_after_count(), 1);
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_plugin_manager() {
    let manager = Arc::new(RwLock::new(PluginManager::new()));
    let counter_plugin = Arc::new(CountingPlugin::new("concurrent_plugin"));
    let plugin: Arc<dyn Plugin> = Arc::clone(&counter_plugin) as Arc<dyn Plugin>;

    {
        let mut mgr = manager.write().await;
        mgr.register(Arc::clone(&plugin)).await.unwrap();
        mgr.initialize("concurrent_plugin").await.unwrap();
        mgr.enable("concurrent_plugin").unwrap();
    }

    // Spawn multiple tasks that execute hooks concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let mgr = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let manager = mgr.read().await;
            let context = PluginContext::new(
                format!("test_{}", i),
                json!({"content": format!("test_{}", i)}),
            );

            manager
                .execute_before_hooks("before_create_node", &context)
                .await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    assert_eq!(counter_plugin.get_before_count(), 10);
}

// ============================================================================
// Plugin Context Tests
// ============================================================================

#[test]
fn test_plugin_context_metadata() {
    let context = PluginContext::new("test", json!({"key": "value"}))
        .with_metadata("user", "test_user")
        .with_metadata("environment", "test");

    assert_eq!(context.operation(), "test");
    assert_eq!(context.get_metadata("user"), Some(&"test_user".to_string()));
    assert_eq!(
        context.get_metadata("environment"),
        Some(&"test".to_string())
    );
    assert_eq!(context.get_metadata("nonexistent"), None);
}

#[test]
fn test_plugin_context_data() {
    let data = json!({
        "content": "test content",
        "metadata": {
            "key": "value"
        }
    });

    let context = PluginContext::new("test", data);

    assert_eq!(
        context.data().get("content").and_then(|v| v.as_str()),
        Some("test content")
    );
    assert!(context.data().get("metadata").is_some());
}
