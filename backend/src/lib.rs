//! Backend for the mus music library, HTTP API, and media workers.
//!
//! HTTP handlers are internal. The exposed modules support startup, library
//! inspection, and media operations; this crate is application code, not a stable SDK.

/// HTTP router construction.
pub mod app;
mod artwork;
/// Artist normalization and safe music filenames.
pub mod audio_names;
/// Category markers inferred from track titles.
pub mod categories;
/// SQLite initialization and library persistence.
pub mod db;
mod derived;
mod downloads;
/// HTTP errors with preserved internal causes.
pub mod error;
mod events;
/// File discovery, fingerprints, and directory permissions.
pub mod files;
mod hls;
/// Metadata, artwork, and in-place audio updates.
pub mod media;
/// Library records and media edit requests.
pub mod models;
mod player;
/// Bounded subprocess execution.
pub mod process;
/// Music discovery and filesystem watching.
pub mod scanner;
/// Shared application resources and mutation locks.
pub mod state;
mod system;
/// Checked Unix timestamp conversion.
pub mod time;
mod tracks;

#[cfg(test)]
extern crate self as mus_backend;
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod test_support;
