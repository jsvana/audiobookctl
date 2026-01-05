//! Terminal UI for browsing audiobooks

// Allow dead code during development - these will be used by the browse command
#![allow(dead_code)]
#![allow(unused_imports)]

mod events;
mod state;
mod ui;

pub use events::{handle_key, poll_event};
pub use state::{App, FilterField, Focus, Mode};
pub use ui::render;
