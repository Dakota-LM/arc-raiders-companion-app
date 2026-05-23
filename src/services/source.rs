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

/// State of the Moka (L1) in-memory cache for a given key, at probe time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L1State {
    Hit,
    Miss,
}

impl L1State {
    /// Lowercase label for the diagnostic pill text.
    pub fn label(&self) -> &'static str {
        match self {
            L1State::Hit => "hit",
            L1State::Miss => "miss",
        }
    }

    /// CSS modifier suffix for the `cache-pill--{}` class (same string as the label).
    pub fn css_class(&self) -> &'static str {
        self.label()
    }
}

/// State of the redb (L2) on-disk cache for a given key, at probe time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L2State {
    Fresh,
    Stale,
    Miss,
}

impl L2State {
    /// Lowercase label for the diagnostic pill text.
    pub fn label(&self) -> &'static str {
        match self {
            L2State::Fresh => "fresh",
            L2State::Stale => "stale",
            L2State::Miss => "miss",
        }
    }

    /// CSS modifier suffix for the `cache-pill--{}` class (same string as the label).
    pub fn css_class(&self) -> &'static str {
        self.label()
    }
}

/// The pre-load state of both cache tiers for a key, shown by the dev diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheState {
    pub l1: L1State,
    pub l2: L2State,
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

    #[test]
    fn l1_state_labels_and_css() {
        assert_eq!(L1State::Hit.label(), "hit");
        assert_eq!(L1State::Miss.label(), "miss");
        assert_eq!(L1State::Hit.css_class(), "hit");
    }

    #[test]
    fn l2_state_labels_and_css() {
        assert_eq!(L2State::Fresh.label(), "fresh");
        assert_eq!(L2State::Stale.label(), "stale");
        assert_eq!(L2State::Miss.label(), "miss");
        assert_eq!(L2State::Stale.css_class(), "stale");
    }
}
