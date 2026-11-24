//! Filtering and 1G1R ("one game, one ROM") selection.
//!
//! The same rules weed the library (Clean) and choose what goes on a card
//! (Collections) without touching the library.

use crate::library::Rom;
use crate::naming;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const DEFAULT_REGION_PRIORITY: &[&str] = &[
    "USA",
    "World",
    "Europe",
    "UK",
    "Australia",
    "Canada",
    "Japan",
];

pub const COMMON_REGIONS: &[&str] = &[
    "USA",
    "World",
    "Europe",
    "Japan",
    "UK",
    "Australia",
    "Canada",
    "Germany",
    "France",
    "Spain",
    "Italy",
    "Korea",
    "China",
    "Brazil",
    "Asia",
];

fn default_priority() -> Vec<String> {
    DEFAULT_REGION_PRIORITY
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct FilterSpec {
    pub exclude_flags: Vec<String>,
    /// Keep only these regions (empty = any).
    pub regions: Vec<String>,
    /// Keep only these languages (empty = any).
    pub languages: Vec<String>,
    /// Glob patterns; when non-empty only matching titles are kept.
    pub include_titles: Vec<String>,
    pub exclude_titles: Vec<String>,
    pub one_game_one_rom: bool,
    pub region_priority: Vec<String>,
    /// Keep only these genres (empty = any).
    pub genres: Vec<String>,
    /// Keep only games with at least one of these tags (empty = any).
    pub tags: Vec<String>,
    /// Leave out games with any of these tags.
    pub exclude_tags: Vec<String>,
    /// Leave out games marked played or finished.
    pub only_unplayed: bool,
}

impl Default for FilterSpec {
    fn default() -> Self {
        FilterSpec {
            exclude_flags: vec![],
            regions: vec![],
            languages: vec![],
            include_titles: vec![],
            exclude_titles: vec![],
            one_game_one_rom: false,
            region_priority: default_priority(),
            genres: vec![],
            tags: vec![],
            exclude_tags: vec![],
            only_unplayed: false,
        }
    }
}

impl FilterSpec {
    pub fn is_empty(&self) -> bool {
        self.exclude_flags.is_empty()
            && self.regions.is_empty()
            && self.languages.is_empty()
            && self.include_titles.is_empty()
            && self.exclude_titles.is_empty()
            && !self.one_game_one_rom
            && self.genres.is_empty()
            && self.tags.is_empty()
            && self.exclude_tags.is_empty()
            && !self.only_unplayed
    }
}

/// Case-insensitive glob with `*` and `?`.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn non_empty(v: &[String]) -> impl Iterator<Item = &str> {
    v.iter().map(|s| s.trim()).filter(|s| !s.is_empty())
}

pub fn matches_any(rom: &Rom, patterns: &[String]) -> bool {
    non_empty(patterns).any(|p| {
        let p = if p.contains('*') || p.contains('?') {
            p.to_string()
        } else {
            format!("*{p}*")
        };
        glob_match(&p, &rom.title) || glob_match(&p, &rom.stem)
    })
}

/// Why a single ROM fails the filters (ignoring 1G1R), or None if it passes.
pub fn rejection_reason(rom: &Rom, spec: &FilterSpec) -> Option<String> {
    let hits: Vec<&str> = spec
        .exclude_flags
        .iter()
        .filter(|f| rom.flags.contains(*f))
        .map(|s| s.as_str())
        .collect();
    if !hits.is_empty() {
        return Some(format!("flag: {}", hits.join(", ")));
    }
    let wanted: HashSet<String> = non_empty(&spec.regions).map(|r| r.to_lowercase()).collect();
    if !wanted.is_empty()
        && !rom.regions.is_empty()
        && !rom
            .regions
            .iter()
            .any(|r| wanted.contains(&r.to_lowercase()))
    {
        return Some(format!("region: {}", rom.regions.join(", ")));
    }
    let wanted: HashSet<String> = non_empty(&spec.languages)
        .map(|l| l.to_lowercase())
        .collect();
    if !wanted.is_empty()
        && !rom.languages.is_empty()
        && !rom.languages.iter().any(|l| wanted.contains(l))
    {
        return Some(format!("language: {}", rom.languages.join(", ")));
    }
    if spec.only_unplayed && rom.played > 0 {
        return Some(crate::library::played_label(rom.played).to_lowercase());
    }
    if !spec.genres.is_empty() && !rom.genre.as_ref().is_some_and(|g| spec.genres.contains(g)) {
        return Some(format!(
            "genre: {}",
            rom.genre.as_deref().unwrap_or("unknown")
        ));
    }
    if !spec.tags.is_empty() && !spec.tags.iter().any(|t| rom.tags.contains(t)) {
        return Some("doesn't have the chosen tags".into());
    }
    if let Some(t) = spec.exclude_tags.iter().find(|t| rom.tags.contains(*t)) {
        return Some(format!("tagged '{t}'"));
    }
    if non_empty(&spec.include_titles).next().is_some() && !matches_any(rom, &spec.include_titles) {
        return Some("not in include list".into());
    }
    if matches_any(rom, &spec.exclude_titles) {
        return Some("title excluded".into());
    }
    None
}

/// Ranking used to choose the best ROM of a game. Greater is better.
fn compare(a: &Rom, b: &Rom, priority: &[String]) -> Ordering {
    let prio: HashMap<String, usize> = priority
        .iter()
        .enumerate()
        .map(|(i, r)| (r.to_lowercase(), i))
        .collect();
    let region_rank = |r: &Rom| {
        r.regions
            .iter()
            .map(|x| *prio.get(&x.to_lowercase()).unwrap_or(&prio.len()))
            .min()
            .unwrap_or(prio.len())
    };
    let bad = |r: &Rom| {
        r.flags
            .iter()
            .filter(|f| *f != "alternate" && *f != "fixed")
            .count()
    };
    bad(b)
        .cmp(&bad(a))
        .then(region_rank(b).cmp(&region_rank(a)))
        .then((!a.flags.contains("translation")).cmp(&!b.flags.contains("translation")))
        .then(a.verified.cmp(&b.verified))
        .then(a.revision.cmp(&b.revision))
        .then((!a.flags.contains("alternate")).cmp(&!b.flags.contains("alternate")))
        .then(b.stem.len().cmp(&a.stem.len()))
        .then(b.id.cmp(&a.id))
}

/// Discs of the same release share a variant key.
pub fn variant_key(rom: &Rom) -> String {
    naming::strip_disc(&rom.stem).to_lowercase()
}

/// Returns (kept, dropped id -> stem of the ROM that replaced it).
pub fn pick_1g1r(roms: Vec<Rom>, priority: &[String]) -> (Vec<Rom>, HashMap<i64, String>) {
    let mut groups: BTreeMap<(String, String), Vec<Rom>> = BTreeMap::new();
    for r in roms {
        groups
            .entry((r.system.clone(), r.key.clone()))
            .or_default()
            .push(r);
    }
    let mut kept = vec![];
    let mut dropped = HashMap::new();
    for (_, members) in groups {
        let mut variants: BTreeMap<String, Vec<Rom>> = BTreeMap::new();
        for r in members {
            variants.entry(variant_key(&r)).or_default().push(r);
        }
        let first_disc = |v: &Vec<Rom>| {
            v.iter()
                .min_by_key(|r| r.disc.unwrap_or(0))
                .unwrap()
                .clone()
        };
        let best_key = variants
            .iter()
            .max_by(|a, b| compare(&first_disc(a.1), &first_disc(b.1), priority))
            .map(|(k, _)| k.clone())
            .unwrap();
        let winner = first_disc(&variants[&best_key]).stem;
        for (k, rs) in variants {
            if k == best_key {
                kept.extend(rs);
            } else {
                for r in rs {
                    dropped.insert(r.id, winner.clone());
                }
            }
        }
    }
    kept.sort_by(|a, b| (&a.system, &a.title, a.disc).cmp(&(&b.system, &b.title, b.disc)));
    (kept, dropped)
}

/// ROMs byte-identical to another; maps dropped id -> stem of the kept ROM.
pub fn exact_duplicates(roms: &[Rom]) -> HashMap<i64, String> {
    let mut groups: HashMap<(String, String, Option<u64>), Vec<&Rom>> = HashMap::new();
    for r in roms {
        if let Some(c) = &r.crc {
            groups
                .entry((r.system.clone(), c.clone(), r.content_size))
                .or_default()
                .push(r);
        }
    }
    let prio = default_priority();
    let mut out = HashMap::new();
    for members in groups.values().filter(|m| m.len() > 1) {
        let best = members.iter().max_by(|a, b| compare(a, b, &prio)).unwrap();
        for r in members {
            if r.id != best.id {
                out.insert(r.id, best.stem.clone());
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct Rejected {
    pub rom: Rom,
    pub reason: String,
}

/// Run the whole pipeline. Returns (kept, rejected-with-reasons).
pub fn apply(roms: Vec<Rom>, spec: &FilterSpec) -> (Vec<Rom>, Vec<Rejected>) {
    let mut rejected = vec![];
    let mut survivors = vec![];
    for r in roms {
        match rejection_reason(&r, spec) {
            Some(reason) => rejected.push(Rejected { rom: r, reason }),
            None => survivors.push(r),
        }
    }
    let dupes = exact_duplicates(&survivors);
    if !dupes.is_empty() {
        let (d, s): (Vec<Rom>, Vec<Rom>) = survivors
            .into_iter()
            .partition(|r| dupes.contains_key(&r.id));
        rejected.extend(d.into_iter().map(|r| {
            let reason = format!("exact duplicate of {}", dupes[&r.id]);
            Rejected { rom: r, reason }
        }));
        survivors = s;
    }
    if spec.one_game_one_rom {
        let all = survivors.clone();
        let (kept, dropped) = pick_1g1r(survivors, &spec.region_priority);
        rejected.extend(
            all.into_iter()
                .filter(|r| dropped.contains_key(&r.id))
                .map(|r| {
                    let reason = format!("1G1R: kept {}", dropped[&r.id]);
                    Rejected { rom: r, reason }
                }),
        );
        survivors = kept;
    }
    (survivors, rejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globbing() {
        assert!(glob_match("*zelda*", "The Legend of Zelda"));
        assert!(glob_match("pok?mon*", "Pokemon Red"));
        assert!(!glob_match("mario", "Super Mario"));
    }
}
