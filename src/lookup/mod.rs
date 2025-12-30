#![allow(dead_code, unused_imports)]

pub mod api;
pub mod merge;
mod trusted;

pub use api::{fetch_audible, fetch_audnexus, fetch_openlibrary, LookupResult};
pub use merge::{merge_results, resolve_with_trusted_source, has_trusted_source_data, FieldValue, MergedMetadata};
pub use trusted::TrustedSource;
