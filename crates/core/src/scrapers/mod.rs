//! Artwork scrapers.

pub mod libretro;
pub mod screenscraper;

use crate::library::Rom;
use crate::systems::System;

pub const ART_KINDS: &[&str] = &["boxart", "snap", "title", "mix", "wheel"];

pub fn art_kind_label(kind: &str) -> &'static str {
    match kind {
        "boxart" => "Box art",
        "snap" => "Screenshot",
        "title" => "Title screen",
        "mix" => "Mix image",
        "wheel" => "Logo / wheel",
        _ => "Image",
    }
}

#[derive(Debug)]
pub enum ScrapeError {
    /// Stop scraping altogether (bad credentials, quota exhausted).
    Fatal(String),
    /// Problem with this one ROM; carry on.
    Skip(String),
}

pub trait Scraper: Send {
    fn name(&self) -> &'static str;
    fn kinds(&self) -> &'static [&'static str];
    fn fetch(
        &mut self,
        rom: &Rom,
        system: &System,
        kind: &str,
    ) -> Result<Option<Vec<u8>>, ScrapeError>;
}
