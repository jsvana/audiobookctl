//! Trusted source handling for auto-accept lookups

use clap::ValueEnum;

/// Valid sources that can be trusted for auto-accept
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TrustedSource {
    Audible,
    Audnexus,
    Openlibrary,
}

impl TrustedSource {
    /// Get the source name as it appears in LookupResult.source
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustedSource::Audible => "audible",
            TrustedSource::Audnexus => "audnexus",
            TrustedSource::Openlibrary => "openlibrary",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trusted_source_as_str() {
        assert_eq!(TrustedSource::Audible.as_str(), "audible");
        assert_eq!(TrustedSource::Audnexus.as_str(), "audnexus");
        assert_eq!(TrustedSource::Openlibrary.as_str(), "openlibrary");
    }
}
