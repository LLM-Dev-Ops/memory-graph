//! Plugin lifecycle management
//!
//! This module provides the `PluginManager` which handles:
//! - Plugin registration and deregistration
//! - Plugin initialization and shutdown
//! - Plugin enable/disable state
//! - Plugin execution coordination
//! - Version compatibility checking

use super::{Plugin, PluginContext, PluginError, PluginMetadata};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginState {
    /// Plugin is registered but not initialized
    Registered,
    /// Plugin is initialized and ready
    Initialized,
    /// Plugin is enabled and active
    Enabled,
    /// Plugin is disabled
    Disabled,
    /// Plugin encountered an error
    Error,
}

/// Plugin wrapper with state tracking
struct PluginWrapper {
    plugin: Arc<dyn Plugin>,
    state: PluginState,
}

/// Plugin manager
///
/// Manages the lifecycle of all plugins in the system. This includes:
/// - Registering new plugins
/// - Initializing plugins on startup
/// - Enabling/disabling plugins at runtime
/// - Executing plugin hooks
/// - Shutting down plugins gracefully
///
/// # Thread Safety
///
/// The `PluginManager` is not thread-safe by itself. It should be wrapped
/// in `Arc<RwLock<PluginManager>>` for concurrent access.
///
/// # Example
///
/// ```rust
/// use llm_memory_graph::plugin::PluginManager;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut manager = PluginManager::new();
/// // Register plugins...
/// manager.init_all().await?;
///
/// // Wrap for concurrent access
/// let manager = Arc::new(RwLock::new(manager));
/// # Ok(())
/// # }
/// ```
pub struct PluginManager {
    plugins: HashMap<String, PluginWrapper>,
    api_version: String,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            api_version: "1.0.0".to_string(),
        }
    }

    /// Create a plugin manager with a specific API version
    pub fn with_api_version(api_version: impl Into<String>) -> Self {
        Self {
            plugins: HashMap::new(),
            api_version: api_version.into(),
        }
    }

    /// Register a plugin
    ///
    /// Registers a new plugin with the manager. The plugin must not already
    /// be registered. After registration, the plugin is in the `Registered` state
    /// and must be initialized before it can be used.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The plugin is already registered
    /// - The plugin's API version is incompatible
    pub fn register(&mut self, plugin: Arc<dyn Plugin>) -> Result<(), PluginError> {
        let metadata = plugin.metadata();
        let name = metadata.name.clone();

        // Check if already registered
        if self.plugins.contains_key(&name) {
            return Err(PluginError::AlreadyRegistered(name));
        }

        info!(
            "Registering plugin: {} v{} by {}",
            name, metadata.version, metadata.author
        );

        // Check API version compatibility
        if !self.is_compatible_version(&metadata.api_version) {
            return Err(PluginError::VersionMismatch(format!(
                "Plugin {} requires API version {}, but {} is supported",
                name, metadata.api_version, self.api_version
            )));
        }

        // Register the plugin
        self.plugins.insert(
            name.clone(),
            PluginWrapper {
                plugin,
                state: PluginState::Registered,
            },
        );

        debug!("Plugin {} registered successfully", name);
        Ok(())
    }

    /// Unregister a plugin
    ///
    /// Removes a plugin from the manager. The plugin must be disabled before
    /// it can be unregistered.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The plugin is not found
    /// - The plugin is still enabled
    pub fn unregister(&mut self, name: &str) -> Result<(), PluginError> {
        let wrapper = self
            .plugins
            .get(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        if wrapper.state == PluginState::Enabled {
            return Err(PluginError::General(format!(
                "Plugin {} must be disabled before unregistering",
                name
            )));
        }

        self.plugins.remove(name);
        info!("Unregistered plugin: {}", name);
        Ok(())
    }

    /// Initialize a specific plugin
    ///
    /// Initializes a registered plugin, calling its `init()` method.
    /// After successful initialization, the plugin is in the `Initialized` state.
    pub async fn initialize(&mut self, name: &str) -> Result<(), PluginError> {
        let wrapper = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        if wrapper.state != PluginState::Registered {
            return Ok(()); // Already initialized
        }

        info!("Initializing plugin: {}", name);

        // Call init on the plugin (plugins use interior mutability for state changes)
        match wrapper.plugin.init().await {
            Ok(()) => {
                wrapper.state = PluginState::Initialized;
                info!("Plugin {} initialized successfully", name);
                Ok(())
            }
            Err(e) => {
                wrapper.state = PluginState::Error;
                error!("Failed to initialize plugin {}: {}", name, e);
                Err(e)
            }
        }
    }

    /// Enable a plugin
    ///
    /// Enables an initialized plugin, making it active and ready to execute hooks.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The plugin is not found
    /// - The plugin is not initialized
    pub fn enable(&mut self, name: &str) -> Result<(), PluginError> {
        let wrapper = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        match wrapper.state {
            PluginState::Initialized | PluginState::Disabled => {
                wrapper.state = PluginState::Enabled;
                info!("Enabled plugin: {}", name);
                Ok(())
            }
            PluginState::Enabled => {
                debug!("Plugin {} is already enabled", name);
                Ok(())
            }
            PluginState::Registered => Err(PluginError::General(format!(
                "Plugin {} must be initialized before enabling",
                name
            ))),
            PluginState::Error => Err(PluginError::General(format!(
                "Plugin {} is in error state",
                name
            ))),
        }
    }

    /// Disable a plugin
    ///
    /// Disables an enabled plugin, preventing it from executing hooks.
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin is not found
    pub fn disable(&mut self, name: &str) -> Result<(), PluginError> {
        let wrapper = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        if wrapper.state == PluginState::Enabled {
            wrapper.state = PluginState::Disabled;
            info!("Disabled plugin: {}", name);
        } else {
            debug!("Plugin {} is already disabled", name);
        }

        Ok(())
    }

    /// Get active plugins
    ///
    /// Returns a list of all enabled plugins that are ready to execute.
    pub fn active_plugins(&self) -> Vec<Arc<dyn Plugin>> {
        self.plugins
            .iter()
            .filter(|(_, wrapper)| wrapper.state == PluginState::Enabled)
            .map(|(_, wrapper)| Arc::clone(&wrapper.plugin))
            .collect()
    }

    /// Get all plugins regardless of state
    pub fn all_plugins(&self) -> Vec<(PluginMetadata, PluginState)> {
        self.plugins
            .values()
            .map(|wrapper| (wrapper.plugin.metadata().clone(), wrapper.state))
            .collect()
    }

    /// Get plugin state
    pub fn get_state(&self, name: &str) -> Option<PluginState> {
        self.plugins.get(name).map(|wrapper| wrapper.state)
    }

    /// Check if a plugin is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.plugins
            .get(name)
            .map(|wrapper| wrapper.state == PluginState::Enabled)
            .unwrap_or(false)
    }

    /// Initialize all registered plugins
    ///
    /// Initializes all plugins that are in the `Registered` state.
    /// Continues even if some plugins fail to initialize.
    ///
    /// # Errors
    ///
    /// Returns an error if any plugin fails to initialize.
    /// The error contains information about the first failure encountered.
    pub async fn init_all(&mut self) -> Result<(), PluginError> {
        let plugin_names: Vec<String> = self
            .plugins
            .iter()
            .filter(|(_, wrapper)| wrapper.state == PluginState::Registered)
            .map(|(name, _)| name.clone())
            .collect();

        let mut errors = Vec::new();

        for name in plugin_names {
            if let Err(e) = self.initialize(&name).await {
                errors.push((name, e));
            }
        }

        if !errors.is_empty() {
            let (name, error) = &errors[0];
            return Err(PluginError::InitFailed(format!(
                "Failed to initialize plugin {}: {}",
                name, error
            )));
        }

        info!("All plugins initialized successfully");
        Ok(())
    }

    /// Enable all initialized plugins
    pub fn enable_all(&mut self) -> Result<(), PluginError> {
        let plugin_names: Vec<String> = self
            .plugins
            .iter()
            .filter(|(_, wrapper)| wrapper.state == PluginState::Initialized)
            .map(|(name, _)| name.clone())
            .collect();

        for name in plugin_names {
            self.enable(&name)?;
        }

        info!("All plugins enabled");
        Ok(())
    }

    /// Disable all plugins
    pub fn disable_all(&mut self) -> Result<(), PluginError> {
        let plugin_names: Vec<String> = self.plugins.keys().cloned().collect();

        for name in plugin_names {
            self.disable(&name)?;
        }

        info!("All plugins disabled");
        Ok(())
    }

    /// Shutdown all plugins
    ///
    /// Calls `shutdown()` on all plugins and removes them from the manager.
    pub async fn shutdown_all(&mut self) -> Result<(), PluginError> {
        let plugin_names: Vec<String> = self.plugins.keys().cloned().collect();

        for name in plugin_names {
            info!("Shutting down plugin: {}", name);

            if let Some(wrapper) = self.plugins.get(&name) {
                // Call shutdown on the plugin
                if let Err(e) = wrapper.plugin.shutdown().await {
                    error!("Error shutting down plugin {}: {}", name, e);
                }
            }
        }

        self.plugins.clear();
        info!("All plugins shut down");
        Ok(())
    }

    /// Execute before hooks for all active plugins
    ///
    /// Executes the specified hook on all enabled plugins in registration order.
    /// If any plugin returns an error, execution stops and the error is returned.
    pub async fn execute_before_hooks(
        &self,
        hook_name: &str,
        context: &PluginContext,
    ) -> Result<(), PluginError> {
        for plugin in self.active_plugins() {
            if let Err(e) = plugin.before_hook(hook_name, context).await {
                error!(
                    "Plugin {} failed on hook {}: {}",
                    plugin.metadata().name,
                    hook_name,
                    e
                );
                return Err(e);
            }
        }
        Ok(())
    }

    /// Execute after hooks for all active plugins
    ///
    /// Executes the specified hook on all enabled plugins in registration order.
    /// Unlike before hooks, errors in after hooks are logged but don't stop execution.
    pub async fn execute_after_hooks(
        &self,
        hook_name: &str,
        context: &PluginContext,
    ) -> Result<(), PluginError> {
        for plugin in self.active_plugins() {
            if let Err(e) = plugin.after_hook(hook_name, context).await {
                // After hooks should not fail the operation
                warn!(
                    "Plugin {} failed on after hook {}: {}",
                    plugin.metadata().name,
                    hook_name,
                    e
                );
            }
        }
        Ok(())
    }

    /// Load plugins from directory (dynamic loading - future)
    ///
    /// This is a placeholder for future dynamic plugin loading functionality.
    /// Currently, plugins must be compiled into the application.
    pub fn load_from_directory(&mut self, _path: impl AsRef<Path>) -> Result<(), PluginError> {
        // TODO: Implement dynamic plugin loading using libloading or similar
        warn!("Dynamic plugin loading not yet implemented");
        Ok(())
    }

    /// List all plugins with their metadata and state
    pub fn list_plugins(&self) -> Vec<(PluginMetadata, PluginState)> {
        self.all_plugins()
    }

    /// Get plugin count by state
    pub fn count_by_state(&self) -> HashMap<PluginState, usize> {
        let mut counts = HashMap::new();
        for wrapper in self.plugins.values() {
            *counts.entry(wrapper.state).or_insert(0) += 1;
        }
        counts
    }

    /// Check if API version is compatible
    fn is_compatible_version(&self, plugin_version: &str) -> bool {
        // For now, we only support exact version match
        // TODO: Implement semantic versioning comparison
        plugin_version == self.api_version
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{PluginBuilder, PluginMetadata};
    use async_trait::async_trait;

    struct MockPlugin {
        metadata: PluginMetadata,
    }

    impl MockPlugin {
        fn new(name: &str) -> Self {
            let metadata = PluginBuilder::new(name, "1.0.0")
                .author("Test")
                .description("Test plugin")
                .build();
            Self { metadata }
        }
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let mut manager = PluginManager::new();
        let plugin = Arc::new(MockPlugin::new("test_plugin"));

        assert!(manager.register(plugin).is_ok());
        assert!(manager.plugins.contains_key("test_plugin"));
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut manager = PluginManager::new();
        let plugin = Arc::new(MockPlugin::new("test_plugin"));

        manager.register(plugin).unwrap();
        assert_eq!(
            manager.get_state("test_plugin"),
            Some(PluginState::Registered)
        );

        manager.initialize("test_plugin").await.unwrap();
        assert_eq!(
            manager.get_state("test_plugin"),
            Some(PluginState::Initialized)
        );

        manager.enable("test_plugin").unwrap();
        assert_eq!(manager.get_state("test_plugin"), Some(PluginState::Enabled));
        assert!(manager.is_enabled("test_plugin"));

        manager.disable("test_plugin").unwrap();
        assert_eq!(
            manager.get_state("test_plugin"),
            Some(PluginState::Disabled)
        );
        assert!(!manager.is_enabled("test_plugin"));
    }

    #[tokio::test]
    async fn test_active_plugins() {
        let mut manager = PluginManager::new();
        let plugin1 = Arc::new(MockPlugin::new("plugin1"));
        let plugin2 = Arc::new(MockPlugin::new("plugin2"));

        manager.register(plugin1).unwrap();
        manager.register(plugin2).unwrap();

        manager.initialize("plugin1").await.unwrap();
        manager.initialize("plugin2").await.unwrap();

        manager.enable("plugin1").unwrap();

        let active = manager.active_plugins();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].metadata().name, "plugin1");
    }
}
