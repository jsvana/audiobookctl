// Allow dead code during phase 1 development - these will be used by commands
#![allow(dead_code, unused_imports)]

mod fields;
mod reader;
mod writer;

pub use fields::AudiobookMetadata;
pub use reader::read_metadata;
pub use writer::write_metadata;
