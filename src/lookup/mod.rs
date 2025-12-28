#![allow(dead_code, unused_imports)]

pub mod api;
pub mod merge;

pub use api::{fetch_audible, fetch_audnexus, fetch_openlibrary, LookupResult};
pub use merge::{merge_results, FieldValue, MergedMetadata};
