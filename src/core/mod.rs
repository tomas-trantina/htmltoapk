//! Core layer: everything that actually does work.
//!
//! The core layer is completely UI-agnostic. It never reads stdin and never
//! prints; progress is emitted through [`report::Reporter`], which both the CLI
//! and the TUI implement. This is what allows `htmltoapk make` and the TUI build
//! screen to share a single build pipeline.

pub mod assets;
pub mod build;
pub mod clean;
pub mod config;
pub mod doctor;
pub mod fsx;
pub mod input;
pub mod naming;
pub mod paths;
pub mod process;
pub mod report;
pub mod workspace;
pub mod zipper;
