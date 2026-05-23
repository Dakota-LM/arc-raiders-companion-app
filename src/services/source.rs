//! Shared description of which cache tier served a value. Produced by every data
//! service and rendered by the `CacheBadge` UI component.

use std::fmt;

/// Which tier of the cache cascade served a piece of data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    /// Fetched live from the MetaForge API.
    Api,
    /// Served from the Moka in-memory L1 cache.
    Memory,
    /// Served from the redb on-disk L2 cache (fresh or stale).
    Disk,
    /// Traders' hardcoded fallback list (API and both caches unavailable).
    Fallback,
}

impl CacheSource {
    /// Human-readable label for the UI badge.
    pub fn label(&self) -> &'static str {
        match self {
            CacheSource::Api => "API",
            CacheSource::Memory => "Memory",
            CacheSource::Disk => "Disk",
            CacheSource::Fallback => "Fallback",
        }
    }

    /// CSS modifier suffix for the `cache-badge--{}` class.
    pub fn css_class(&self) -> &'static str {
        match self {
            CacheSource::Api => "api",
            CacheSource::Memory => "memory",
            CacheSource::Disk => "disk",
            CacheSource::Fallback => "fallback",
        }
    }
}

impl fmt::Display for CacheSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_matches_each_variant() {
        assert_eq!(CacheSource::Api.label(), "API");
        assert_eq!(CacheSource::Memory.label(), "Memory");
        assert_eq!(CacheSource::Disk.label(), "Disk");
        assert_eq!(CacheSource::Fallback.label(), "Fallback");
    }

    #[test]
    fn css_class_matches_each_variant() {
        assert_eq!(CacheSource::Api.css_class(), "api");
        assert_eq!(CacheSource::Memory.css_class(), "memory");
        assert_eq!(CacheSource::Disk.css_class(), "disk");
        assert_eq!(CacheSource::Fallback.css_class(), "fallback");
    }

    #[test]
    fn display_uses_label() {
        assert_eq!(format!("{}", CacheSource::Disk), "Disk");
    }
}
