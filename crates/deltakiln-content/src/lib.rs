//! Source discovery, front matter, routing and markdown rendering.
//!
//! Everything here is a pure function of its inputs, which is what makes the
//! naive full build usable as a correctness oracle.

pub mod discover;
pub mod markdown;
pub mod page;
pub mod route;

pub use discover::{SourceFile, is_markdown, list_files};
pub use page::{FrontMatter, Page};
pub use route::{Route, ensure_unique_outputs, route};
