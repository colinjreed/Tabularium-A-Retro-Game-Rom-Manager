//! Core logic for Tabularium: organise a ROM library, weed out duplicates
//! and junk, scrape artwork, check BIOS files and export SD card collections.

pub mod bios;
pub mod collection;
pub mod dats;
pub mod detect;
pub mod filters;
pub mod folder_games;
pub mod hashing;
pub mod http;
pub mod import;
pub mod library;
pub mod metadata;
pub mod naming;
pub mod playhistory;
pub mod profiles;
pub mod progress;
pub mod rename;
pub mod scrape;
pub mod scrapers;
pub mod systems;

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No ROM library at {0}")]
    NoLibrary(PathBuf),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// "1.5 GB"-style sizes.
pub fn human_size(n: u64) -> String {
    let mut v = n as f64;
    for unit in ["B", "KB", "MB", "GB"] {
        if v < 1024.0 {
            return if unit == "B" {
                format!("{n} B")
            } else {
                format!("{v:.1} {unit}")
            };
        }
        v /= 1024.0;
    }
    format!("{v:.1} TB")
}

/// Parse "64G", "500 MB", "1.5T".
pub fn parse_size(text: &str) -> Option<u64> {
    let t = text.trim().to_ascii_lowercase();
    let t = t.trim_end_matches('b').trim_end_matches('i');
    let (num, mult) = match t.chars().last()? {
        'k' => (&t[..t.len() - 1], 1u64 << 10),
        'm' => (&t[..t.len() - 1], 1 << 20),
        'g' => (&t[..t.len() - 1], 1 << 30),
        't' => (&t[..t.len() - 1], 1 << 40),
        _ => (t, 1),
    };
    let v: f64 = num.trim().parse().ok()?;
    (v >= 0.0).then_some((v * mult as f64) as u64)
}
