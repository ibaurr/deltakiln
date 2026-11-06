//! Templating and the naive full build.
//!
//! [`naive::naive_build`] is the correctness oracle: a plain, from-scratch
//! build with no cache and no graph. Later milestones (`verify`, the property
//! tests) compare incremental output against it, so it must stay simple,
//! deterministic and permanently available.

pub mod naive;
pub mod templates;

pub use naive::{BuildOptions, BuildReport, PageOutput, naive_build};
pub use templates::DEFAULT_PAGE_TEMPLATE;
