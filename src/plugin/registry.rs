//! Plugin registry and discovery system
//!
//! This module provides functionality for discovering, cataloging, and
//! managing plugin metadata in a centralized registry.

use super::{Plugin, PluginError, PluginMetadata};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Plugin registry entry
#[derive(Debug, Clone)]
pub struct PluginRegistryEntry {
    /// Plugin metadata
    pub metadata: PluginMetadata,
    /// Plugin source location (if known)
    pub source: Option<PathBuf>,
    /// Whether the plugin is currently loaded
    pub loaded: bool,
    /// When the plugin was registered
    pub registered_at: chrono::DateTime<chrono::Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Plugin registry
///
/// Maintains a catalog of available plugins, their metadata, and discovery information.
/// The registry is separate from the manager - it tracks what plugins are available,
/// while the manager tracks what plugins are actively loaded and running.
pub struct PluginRegistry {
    entries: HashMap<String, PluginRegistryEntry>,
    search_paths: Vec<PathBuf>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            search_paths: Vec::new(),
        }
    }

    /// Add a search path for plugin discovery
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Register a plugin
    ///
    /// Adds a plugin to the registry with its metadata and optional source location.
    pub fn register(
        &mut self,
        metadata: PluginMetadata,
        source: Option<PathBuf>,
    ) -> Result<(), PluginError> {
        let name = metadata.name.clone();

        if self.entries.contains_key(&name) {
            return Err(PluginError::AlreadyRegistered(name));
        }

        info!("Registering plugin in registry: {}", name);

        self.entries.insert(
            name.clone(),
            PluginRegistryEntry {
                metadata,
                source,
                loaded: false,
                registered_at: chrono::Utc::now(),
                tags: Vec::new(),
            },
        );

        Ok(())
    }

    /// Register a loaded plugin
    ///
    /// Convenience method to register a plugin that's already loaded.
    pub fn register_plugin(&mut self, plugin: &Arc<dyn Plugin>) -> Result<(), PluginError> {
        let metadata = plugin.metadata().clone();
        let name = metadata.name.clone();

        if self.entries.contains_key(&name) {
            return Err(PluginError::AlreadyRegistered(name));
        }

        self.entries.insert(
            name.clone(),
            PluginRegistryEntry {
                metadata,
                source: None,
                loaded: true,
                registered_at: chrono::Utc::now(),
                tags: Vec::new(),
            },
        );

        Ok(())
    }

    /// Mark a plugin as loaded
    pub fn mark_loaded(&mut self, name: &str) -> Result<(), PluginError> {
        let entry = self
            .entries
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        entry.loaded = true;
        Ok(())
    }

    /// Mark a plugin as unloaded
    pub fn mark_unloaded(&mut self, name: &str) -> Result<(), PluginError> {
        let entry = self
            .entries
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        entry.loaded = false;
        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&mut self, name: &str) -> Result<(), PluginError> {
        self.entries
            .remove(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        info!("Unregistered plugin from registry: {}", name);
        Ok(())
    }

    /// Get a plugin entry
    pub fn get(&self, name: &str) -> Option<&PluginRegistryEntry> {
        self.entries.get(name)
    }

    /// Check if a plugin is registered
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// List all registered plugins
    pub fn list_all(&self) -> Vec<&PluginRegistryEntry> {
        self.entries.values().collect()
    }

    /// List loaded plugins
    pub fn list_loaded(&self) -> Vec<&PluginRegistryEntry> {
        self.entries
            .values()
            .filter(|entry| entry.loaded)
            .collect()
    }

    /// List unloaded plugins
    pub fn list_unloaded(&self) -> Vec<&PluginRegistryEntry> {
        self.entries
            .values()
            .filter(|entry| !entry.loaded)
            .collect()
    }

    /// Find plugins by capability
    pub fn find_by_capability(&self, capability: &str) -> Vec<&PluginRegistryEntry> {
        self.entries
            .values()
            .filter(|entry| entry.metadata.capabilities.contains(&capability.to_string()))
            .collect()
    }

    /// Find plugins by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<&PluginRegistryEntry> {
        self.entries
            .values()
            .filter(|entry| entry.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Add a tag to a plugin
    pub fn add_tag(&mut self, name: &str, tag: impl Into<String>) -> Result<(), PluginError> {
        let entry = self
            .entries
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        let tag = tag.into();
        if !entry.tags.contains(&tag) {
            entry.tags.push(tag);
        }

        Ok(())
    }

    /// Remove a tag from a plugin
    pub fn remove_tag(&mut self, name: &str, tag: &str) -> Result<(), PluginError> {
        let entry = self
            .entries
            .get_mut(name)
            .ok_or_else(|| PluginError::NotFound(name.to_string()))?;

        entry.tags.retain(|t| t != tag);
        Ok(())
    }

    /// Get registry statistics
    pub fn stats(&self) -> PluginRegistryStats {
        let total = self.entries.len();
        let loaded = self.list_loaded().len();
        let unloaded = self.list_unloaded().len();

        let mut capabilities = HashMap::new();
        for entry in self.entries.values() {
            for capability in &entry.metadata.capabilities {
                *capabilities.entry(capability.clone()).or_insert(0) += 1;
            }
        }

        PluginRegistryStats {
            total_plugins: total,
            loaded_plugins: loaded,
            unloaded_plugins: unloaded,
            capabilities,
        }
    }

    /// Clear the registry
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin registry statistics
#[derive(Debug, Clone)]
pub struct PluginRegistryStats {
    /// Total number of registered plugins
    pub total_plugins: usize,
    /// Number of loaded plugins
    pub loaded_plugins: usize,
    /// Number of unloaded plugins
    pub unloaded_plugins: usize,
    /// Capabilities and their counts
    pub capabilities: HashMap<String, usize>,
}

/// Plugin discovery
///
/// Provides functionality for discovering plugins from various sources.
pub struct PluginDiscovery {
    search_paths: Vec<PathBuf>,
}

impl PluginDiscovery {
    /// Create a new plugin discovery instance
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
        }
    }

    /// Add a search path
    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Discover plugins in all search paths
    ///
    /// This is a placeholder for future dynamic plugin loading.
    /// Currently returns an empty list as plugins must be compiled in.
    pub fn discover(&self) -> Vec<PluginMetadata> {
        let mut discovered = Vec::new();

        for path in &self.search_paths {
            if let Ok(entries) = self.discover_in_path(path) {
                discovered.extend(entries);
            }
        }

        discovered
    }

    /// Discover plugins in a specific path
    fn discover_in_path(&self, path: &Path) -> Result<Vec<PluginMetadata>, PluginError> {
        debug!("Searching for plugins in: {}", path.display());

        if !path.exists() {
            warn!("Plugin search path does not exist: {}", path.display());
            return Ok(Vec::new());
        }

        // TODO: Implement actual plugin discovery
        // This would involve:
        // 1. Scanning directories for plugin libraries
        // 2. Reading plugin metadata files
        // 3. Validating plugin signatures
        // 4. Loading plugin manifests
        warn!("Plugin discovery not yet implemented");

        Ok(Vec::new())
    }

    /// Scan for plugin metadata files
    ///
    /// Looks for plugin.json or plugin.toml files in the search paths.
    pub fn scan_metadata_files(&self) -> Vec<PathBuf> {
        let mut metadata_files = Vec::new();

        for path in &self.search_paths {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name() {
                            let filename = filename.to_string_lossy();
                            if filename == "plugin.json" || filename == "plugin.toml" {
                                metadata_files.push(path);
                            }
                        }
                    }
                }
            }
        }

        metadata_files
    }

    /// Load plugin metadata from a file
    ///
    /// Placeholder for loading plugin metadata from JSON or TOML files.
    pub fn load_metadata_from_file(
        &self,
        _path: &Path,
    ) -> Result<PluginMetadata, PluginError> {
        // TODO: Implement metadata file parsing
        Err(PluginError::General(
            "Metadata file loading not yet implemented".to_string(),
        ))
    }
}

impl Default for PluginDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin catalog
///
/// A read-only view of available plugins with filtering and search capabilities.
pub struct PluginCatalog {
    entries: Vec<PluginRegistryEntry>,
}

impl PluginCatalog {
    /// Create a catalog from a registry
    pub fn from_registry(registry: &PluginRegistry) -> Self {
        Self {
            entries: registry.list_all().into_iter().cloned().collect(),
        }
    }

    /// Get all entries
    pub fn entries(&self) -> &[PluginRegistryEntry] {
        &self.entries
    }

    /// Filter by capability
    pub fn with_capability(self, capability: &str) -> Self {
        let entries = self
            .entries
            .into_iter()
            .filter(|entry| entry.metadata.capabilities.contains(&capability.to_string()))
            .collect();

        Self { entries }
    }

    /// Filter by loaded status
    pub fn loaded(self, loaded: bool) -> Self {
        let entries = self
            .entries
            .into_iter()
            .filter(|entry| entry.loaded == loaded)
            .collect();

        Self { entries }
    }

    /// Filter by tag
    pub fn with_tag(self, tag: &str) -> Self {
        let entries = self
            .entries
            .into_iter()
            .filter(|entry| entry.tags.contains(&tag.to_string()))
            .collect();

        Self { entries }
    }

    /// Sort by name
    pub fn sort_by_name(mut self) -> Self {
        self.entries
            .sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        self
    }

    /// Sort by registration time
    pub fn sort_by_time(mut self) -> Self {
        self.entries
            .sort_by(|a, b| a.registered_at.cmp(&b.registered_at));
        self
    }

    /// Get the count
    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginBuilder;

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        let metadata = PluginBuilder::new("test_plugin", "1.0.0")
            .author("Test")
            .description("Test plugin")
            .capability("validation")
            .build();

        assert!(registry.register(metadata.clone(), None).is_ok());
        assert!(registry.contains("test_plugin"));
        assert_eq!(registry.list_all().len(), 1);
    }

    #[test]
    fn test_plugin_registry_loaded_status() {
        let mut registry = PluginRegistry::new();
        let metadata = PluginBuilder::new("test_plugin", "1.0.0")
            .author("Test")
            .description("Test plugin")
            .build();

        registry.register(metadata, None).unwrap();
        assert_eq!(registry.list_loaded().len(), 0);
        assert_eq!(registry.list_unloaded().len(), 1);

        registry.mark_loaded("test_plugin").unwrap();
        assert_eq!(registry.list_loaded().len(), 1);
        assert_eq!(registry.list_unloaded().len(), 0);
    }

    #[test]
    fn test_plugin_registry_capabilities() {
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

        let validation_plugins = registry.find_by_capability("validation");
        assert_eq!(validation_plugins.len(), 2);

        let enrichment_plugins = registry.find_by_capability("enrichment");
        assert_eq!(enrichment_plugins.len(), 1);
    }

    #[test]
    fn test_plugin_catalog() {
        let mut registry = PluginRegistry::new();
        let metadata1 = PluginBuilder::new("plugin1", "1.0.0")
            .capability("validation")
            .build();
        let metadata2 = PluginBuilder::new("plugin2", "1.0.0")
            .capability("enrichment")
            .build();

        registry.register(metadata1, None).unwrap();
        registry.register(metadata2, None).unwrap();
        registry.mark_loaded("plugin1").unwrap();

        let catalog = PluginCatalog::from_registry(&registry);
        assert_eq!(catalog.count(), 2);

        let loaded_catalog = catalog.loaded(true);
        assert_eq!(loaded_catalog.count(), 1);
    }
}
