//! libretro-thumbnails: free, no account, matched by No-Intro / Redump filename.
//!
//! A per-system list of available images is downloaded once so that slightly
//! different names (other region, revision, missing tags) still match.

use super::{ScrapeError, Scraper};
use crate::filters::DEFAULT_REGION_PRIORITY;
use crate::http::{Fetcher, HttpError};
use crate::library::Rom;
use crate::naming;
use crate::systems::System;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use regex::Regex;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const BASE: &str = "https://thumbnails.libretro.com";
const INDEX_MAX_AGE: Duration = Duration::from_secs(14 * 86400);

pub fn kind_dir(kind: &str) -> Option<&'static str> {
    match kind {
        "boxart" => Some("Named_Boxarts"),
        "snap" => Some("Named_Snaps"),
        "title" => Some("Named_Titles"),
        _ => None,
    }
}

fn enc(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

pub fn repo_name(playlist: &str) -> String {
    playlist.replace(' ', "_")
}

fn region_rank(regions: &[String], wanted: &[String], priority: &[String]) -> usize {
    if regions.iter().any(|r| wanted.contains(r)) {
        return 0;
    }
    regions
        .iter()
        .map(|r| {
            priority
                .iter()
                .position(|p| p == r)
                .unwrap_or(priority.len())
                + 1
        })
        .min()
        .unwrap_or(priority.len() + 1)
}

/// Choose the thumbnail name that best fits a ROM filename stem.
pub fn best_match(stem: &str, names: &[String], cutoff: f64) -> Option<String> {
    let priority: Vec<String> = DEFAULT_REGION_PRIORITY
        .iter()
        .map(|s| s.to_string())
        .collect();
    let set: BTreeSet<&str> = names.iter().map(|s| s.as_str()).collect();
    let safe = naming::libretro_safe(stem);
    if set.contains(safe.as_str()) {
        return Some(safe);
    }
    let no_disc = naming::libretro_safe(&naming::strip_disc(stem));
    if set.contains(no_disc.as_str()) {
        return Some(no_disc);
    }
    let target = naming::parse(stem);
    let mut by_key: HashMap<String, Vec<&String>> = HashMap::new();
    for n in names {
        by_key.entry(naming::parse(n).key).or_default().push(n);
    }
    let candidates = match by_key.get(&target.key) {
        Some(c) => c.clone(),
        None => {
            let (best, score) = by_key
                .keys()
                .map(|k| (k, strsim::normalized_levenshtein(k, &target.key)))
                .max_by(|a, b| a.1.total_cmp(&b.1))?;
            if score < cutoff {
                return None;
            }
            by_key[best].clone()
        }
    };
    candidates
        .into_iter()
        .min_by_key(|n| {
            let p = naming::parse(n);
            (
                p.flags.iter().filter(|f| *f != "alternate").count(),
                region_rank(&p.regions, &target.regions, &priority),
                p.disc != target.disc,
                usize::MAX - p.tags.iter().filter(|t| target.tags.contains(t)).count(),
                n.len(),
            )
        })
        .cloned()
}

pub struct LibretroScraper {
    cache_dir: PathBuf,
    cutoff: f64,
    fetch: Fetcher,
    indexes: HashMap<(String, String), Option<Vec<String>>>,
    pub log: Box<dyn FnMut(String) + Send>,
}

impl LibretroScraper {
    pub fn new(cache_dir: PathBuf, fetch: Fetcher) -> Self {
        LibretroScraper {
            cache_dir,
            cutoff: 0.92,
            fetch,
            indexes: HashMap::new(),
            log: Box::new(|_| {}),
        }
    }

    pub fn with_cutoff(mut self, cutoff: f64) -> Self {
        self.cutoff = cutoff;
        self
    }

    fn fetch_index(&self, playlist: &str, dir: &str) -> Option<Vec<String>> {
        let url = format!("{BASE}/{}/{dir}/", enc(playlist));
        if let Ok(Some(html)) = (self.fetch)(&url) {
            let re = Regex::new(r#"href="([^"?/]+)\.png""#).unwrap();
            let text = String::from_utf8_lossy(&html);
            let names: BTreeSet<String> = re
                .captures_iter(&text)
                .map(|c| percent_decode_str(&c[1]).decode_utf8_lossy().into_owned())
                .collect();
            if !names.is_empty() {
                return Some(names.into_iter().collect());
            }
        }
        let url = format!(
            "https://api.github.com/repos/libretro-thumbnails/{}/git/trees/master?recursive=1",
            repo_name(playlist)
        );
        if let Ok(Some(body)) = (self.fetch)(&url) {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&body) {
                let prefix = format!("{dir}/");
                let names: Vec<String> = v["tree"]
                    .as_array()?
                    .iter()
                    .filter_map(|t| t["path"].as_str())
                    .filter_map(|p| {
                        p.strip_prefix(&prefix)?
                            .strip_suffix(".png")
                            .map(String::from)
                    })
                    .collect();
                if !names.is_empty() {
                    return Some(names);
                }
            }
        }
        None
    }

    fn index(&mut self, playlist: &str, dir: &str) -> Option<Vec<String>> {
        let key = (playlist.to_string(), dir.to_string());
        if let Some(v) = self.indexes.get(&key) {
            return v.clone();
        }
        let cache = self
            .cache_dir
            .join("libretro")
            .join(format!("{}__{dir}.json", repo_name(playlist)));
        let fresh = fs::metadata(&cache)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|age| age < INDEX_MAX_AGE);
        let names = if fresh {
            fs::read(&cache)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
        } else {
            let names = self.fetch_index(playlist, dir);
            match &names {
                Some(n) => {
                    let _ = fs::create_dir_all(cache.parent().unwrap());
                    let _ = fs::write(&cache, serde_json::to_vec(n).unwrap());
                }
                None => (self.log)(format!(
                    "No image list for {playlist}; using exact-name lookups only"
                )),
            }
            names
        };
        self.indexes.insert(key, names.clone());
        names
    }

    fn download(
        &self,
        playlist: &str,
        dir: &str,
        name: &str,
    ) -> Result<Option<Vec<u8>>, HttpError> {
        let urls = [
            format!("{BASE}/{}/{dir}/{}.png", enc(playlist), enc(name)),
            format!(
                "https://raw.githubusercontent.com/libretro-thumbnails/{}/master/{dir}/{}.png",
                repo_name(playlist),
                enc(name)
            ),
        ];
        // Only report a network error if no source could answer at all.
        let mut last_err = None;
        let mut answered = false;
        for url in urls {
            match (self.fetch)(&url) {
                Ok(Some(data)) if data.starts_with(b"\x89PNG\r\n\x1a\n") => return Ok(Some(data)),
                Ok(_) => answered = true,
                Err(HttpError::Status(_)) => answered = true,
                Err(e) => last_err = Some(e),
            }
        }
        match last_err {
            Some(e) if !answered => Err(e),
            _ => Ok(None),
        }
    }
}

impl Scraper for LibretroScraper {
    fn name(&self) -> &'static str {
        "libretro"
    }

    fn kinds(&self) -> &'static [&'static str] {
        &["boxart", "snap", "title"]
    }

    fn fetch(
        &mut self,
        rom: &Rom,
        system: &System,
        kind: &str,
    ) -> Result<Option<Vec<u8>>, ScrapeError> {
        let (Some(dir), Some(playlist)) = (kind_dir(kind), system.libretro) else {
            return Ok(None);
        };
        let guesses: Vec<String> = match self.index(playlist, dir) {
            Some(names) => best_match(&rom.stem, &names, self.cutoff)
                .into_iter()
                .collect(),
            None => {
                let mut g = vec![naming::libretro_safe(&rom.stem)];
                let nd = naming::libretro_safe(&naming::strip_disc(&rom.stem));
                if nd != g[0] {
                    g.push(nd);
                }
                g
            }
        };
        for g in guesses {
            match self.download(playlist, dir, &g) {
                Ok(Some(d)) => return Ok(Some(d)),
                Ok(None) => {}
                Err(e) => return Err(ScrapeError::Skip(e.to_string())),
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching() {
        let names: Vec<String> = [
            "Tetris (World) (Rev 1)",
            "Tetris (Japan)",
            "Tetris 2 (USA)",
            "Legend of Zelda, The - Link's Awakening (USA, Europe) (Rev 2)",
            "Final Fantasy VII (USA) (Disc 1)",
            "Q_bert (USA)",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let m = |s| best_match(s, &names, 0.92);
        assert_eq!(
            m("Tetris (World) (Rev 1)").unwrap(),
            "Tetris (World) (Rev 1)"
        );
        assert_eq!(m("Tetris (USA)").unwrap(), "Tetris (World) (Rev 1)");
        assert_eq!(m("Tetris (Japan) (Rev A)").unwrap(), "Tetris (Japan)");
        assert!(m("The Legend of Zelda - Link's Awakening (Europe)")
            .unwrap()
            .starts_with("Legend of Zelda"));
        assert_eq!(m("Q*bert (USA)").unwrap(), "Q_bert (USA)");
        assert!(m("Completely Different Game (USA)").is_none());
    }
}
