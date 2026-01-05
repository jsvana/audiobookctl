//! Terminal UI for browsing audiobooks

// Allow dead code during development - these will be used by the browse command
#![allow(dead_code)]
#![allow(unused_imports)]

mod state;

pub use state::{App, FilterField, Focus, Mode};
