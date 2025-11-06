//! Configuration for the memory graph

use std::path::PathBuf;

/// Configuration for `MemoryGraph`
#[derive(Debug, Clone)]
pub struct Config {
    /// Path to the database directory
    pub path: PathBuf,
    /// Cache size in megabytes
    pub cache_size_mb: usize,
    /// Enable write-ahead logging for durability
    pub enable_wal: bool,
    /// Compression level (0-9, 0 = no compression)
    pub compression_level: u8,
    /// Flush interval in milliseconds (0 = sync every write)
    pub flush_interval_ms: u64,
}

impl Config {
    /// Create a new configuration with custom path
    #[must_use]
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            path: path.into(),
            cache_size_mb: 100,
            enable_wal: true,
            compression_level: 3,
            flush_interval_ms: 1000,
        }
    }

    /// Set the cache size
    #[must_use]
    pub fn with_cache_size(mut self, size_mb: usize) -> Self {
        self.cache_size_mb = size_mb;
        self
    }

    /// Enable or disable write-ahead logging
    #[must_use]
    pub const fn with_wal(mut self, enable: bool) -> Self {
        self.enable_wal = enable;
        self
    }

    /// Set compression level (0-9)
    #[must_use]
    pub fn with_compression(mut self, level: u8) -> Self {
        self.compression_level = level.min(9);
        self
    }

    /// Set flush interval in milliseconds
    #[must_use]
    pub const fn with_flush_interval(mut self, interval_ms: u64) -> Self {
        self.flush_interval_ms = interval_ms;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./data/graph.db"),
            cache_size_mb: 100,
            enable_wal: true,
            compression_level: 3,
            flush_interval_ms: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.cache_size_mb, 100);
        assert!(config.enable_wal);
    }

    #[test]
    fn test_builder_pattern() {
        let config = Config::new("./test.db")
            .with_cache_size(200)
            .with_wal(false)
            .with_compression(5)
            .with_flush_interval(2000);

        assert_eq!(config.cache_size_mb, 200);
        assert!(!config.enable_wal);
        assert_eq!(config.compression_level, 5);
        assert_eq!(config.flush_interval_ms, 2000);
    }

    #[test]
    fn test_compression_clamping() {
        let config = Config::default().with_compression(15);
        assert_eq!(config.compression_level, 9);
    }
}
