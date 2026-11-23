//! ScreenScraper.fr: matched by file hash, needs an account.
//!
//! ScreenScraper identifies games by CRC/size/filename, so it finds art for
//! badly named ROMs and arcade sets that libretro's name matching misses, and
//! offers "mix" images and wheels/logos. Its API needs developer credentials
//! (requested from ScreenScraper for this program) plus your user account,
//! which sets the daily quota.

use super::{ScrapeError, Scraper};
use crate::http::{Fetcher, HttpError};
use crate::library::Rom;
use crate::systems::System;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const API: &str = "https://api.screenscraper.fr/api2/jeuInfos.php";
const REGION_ORDER: &[&str] = &["us", "wor", "eu", "uk", "ss", "jp", "cus"];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Credentials {
    pub dev_id: String,
    pub dev_password: String,
    pub user: String,
    pub password: String,
}

impl Credentials {
    /// Fill blanks from SCREENSCRAPER_* environment variables.
    pub fn with_env_fallback(mut self) -> Self {
        let env = |k: &str| std::env::var(k).unwrap_or_default();
        if self.dev_id.is_empty() {
            self.dev_id = env("SCREENSCRAPER_DEVID")
        }
        if self.dev_password.is_empty() {
            self.dev_password = env("SCREENSCRAPER_DEVPASSWORD")
        }
        if self.user.is_empty() {
            self.user = env("SCREENSCRAPER_USER")
        }
        if self.password.is_empty() {
            self.password = env("SCREENSCRAPER_PASSWORD")
        }
        self
    }

    pub fn has_dev(&self) -> bool {
        !self.dev_id.is_empty() && !self.dev_password.is_empty()
    }
}

pub fn media_types(kind: &str) -> &'static [&'static str] {
    match kind {
        "boxart" => &["box-2D"],
        "snap" => &["ss"],
        "title" => &["sstitle"],
        "mix" => &["mixrbv2", "mixrbv1"],
        "wheel" => &["wheel", "wheel-hd"],
        _ => &[],
    }
}

pub fn pick_media(medias: &[serde_json::Value], kind: &str) -> Option<String> {
    for t in media_types(kind) {
        let best = medias
            .iter()
            .filter(|m| m["type"].as_str() == Some(t) && m["url"].as_str().is_some())
            .min_by_key(|m| {
                let r = m["region"].as_str().unwrap_or("");
                REGION_ORDER
                    .iter()
                    .position(|x| *x == r)
                    .unwrap_or(REGION_ORDER.len())
            });
        if let Some(m) = best {
            return m["url"].as_str().map(String::from);
        }
    }
    None
}

fn enc(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

pub struct ScreenScraper {
    creds: Credentials,
    fetch: Fetcher,
    min_interval: Duration,
    last: Option<Instant>,
    games: HashMap<i64, Option<serde_json::Value>>,
}

impl ScreenScraper {
    pub fn new(creds: Credentials, fetch: Fetcher) -> Result<Self, String> {
        if !creds.has_dev() {
            return Err(
                "ScreenScraper needs developer credentials (Dev ID and Dev password) in \
                        Settings, or SCREENSCRAPER_DEVID / SCREENSCRAPER_DEVPASSWORD."
                    .into(),
            );
        }
        Ok(ScreenScraper {
            creds,
            fetch,
            min_interval: Duration::from_millis(1100),
            last: None,
            games: HashMap::new(),
        })
    }

    pub fn with_interval(mut self, d: Duration) -> Self {
        self.min_interval = d;
        self
    }

    pub fn query_url(&self, rom: &Rom, system: &System) -> String {
        let mut q = vec![
            ("devid", self.creds.dev_id.clone()),
            ("devpassword", self.creds.dev_password.clone()),
            ("softname", "rom-manager".into()),
            ("output", "json".into()),
            ("ssid", self.creds.user.clone()),
            ("sspassword", self.creds.password.clone()),
            ("romtype", "rom".into()),
            ("romnom", rom.file_name().to_string()),
        ];
        if let Some(id) = system.screenscraper_id {
            q.push(("systemeid", id.to_string()));
        }
        if let Some(c) = &rom.crc {
            q.push(("crc", c.to_uppercase()));
        }
        if let Some(s) = rom.content_size {
            q.push(("romtaille", s.to_string()));
        }
        let qs: Vec<String> = q.iter().map(|(k, v)| format!("{k}={}", enc(v))).collect();
        format!("{API}?{}", qs.join("&"))
    }

    fn game(
        &mut self,
        rom: &Rom,
        system: &System,
    ) -> Result<Option<serde_json::Value>, ScrapeError> {
        if let Some(g) = self.games.get(&rom.id) {
            return Ok(g.clone());
        }
        if let Some(last) = self.last {
            let wait = self.min_interval.saturating_sub(last.elapsed());
            std::thread::sleep(wait);
        }
        self.last = Some(Instant::now());
        let body = match (self.fetch)(&self.query_url(rom, system)) {
            Ok(b) => b,
            Err(HttpError::Status(c @ (401 | 403))) => {
                return Err(ScrapeError::Fatal(format!(
                    "ScreenScraper refused the credentials (HTTP {c})."
                )))
            }
            Err(HttpError::Status(c @ (423 | 426 | 429 | 430 | 431))) => {
                return Err(ScrapeError::Fatal(format!(
                    "ScreenScraper quota or availability problem (HTTP {c}); try again later."
                )))
            }
            Err(e) => return Err(ScrapeError::Skip(e.to_string())),
        };
        // "Not found" is answered with plain text, which isn't JSON.
        let game = body
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .map(|v| v["response"]["jeu"].clone())
            .filter(|g| !g.is_null());
        self.games.insert(rom.id, game.clone());
        Ok(game)
    }
}

impl Scraper for ScreenScraper {
    fn name(&self) -> &'static str {
        "screenscraper"
    }

    fn kinds(&self) -> &'static [&'static str] {
        &["boxart", "snap", "title", "mix", "wheel"]
    }

    fn fetch(
        &mut self,
        rom: &Rom,
        system: &System,
        kind: &str,
    ) -> Result<Option<Vec<u8>>, ScrapeError> {
        let Some(game) = self.game(rom, system)? else {
            return Ok(None);
        };
        let medias = game["medias"].as_array().cloned().unwrap_or_default();
        let Some(url) = pick_media(&medias, kind) else {
            return Ok(None);
        };
        (self.fetch)(&url).map_err(|e| ScrapeError::Skip(e.to_string()))
    }
}
