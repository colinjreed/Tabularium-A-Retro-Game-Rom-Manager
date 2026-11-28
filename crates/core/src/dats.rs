//! Official No-Intro names, looked up by checksum.
//!
//! libretro-database carries the No-Intro DAT for every cartridge system:
//! for each known dump, its CRC32 and official name. Matching a file's CRC
//! gives its proper name ("Sonic The Hedgehog (USA, Europe)") whatever it is
//! called now, and can reveal that a file belongs to a related system (a
//! Game Gear game dumped with a .sms extension).

use crate::http::Fetcher;
use crate::naming;
use crate::systems;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

const BASE: &str =
    "https://raw.githubusercontent.com/libretro/libretro-database/master/metadat/no-intro";
const MAX_AGE: Duration = Duration::from_secs(30 * 86400);

/// URL of a system's No-Intro DAT (cartridge systems only).
pub fn dat_url(system: &str) -> Option<String> {
    let s = systems::get(system)?;
    if s.disc_based || systems::is_arcade(system) || systems::folder_games(system) {
        return None;
    }
    let playlist = s.libretro?;
    let enc = percent_encoding::utf8_percent_encode(playlist, percent_encoding::NON_ALPHANUMERIC);
    Some(format!("{BASE}/{enc}.dat"))
}

/// Systems whose dumps are easily mixed up with this one's.
pub fn related(system: &str) -> &'static [&'static str] {
    match system {
        "mastersystem" | "gamegear" | "sg1000" => &["mastersystem", "gamegear", "sg1000"],
        "genesis" | "sega32x" => &["genesis", "sega32x"],
        "gb" | "gbc" => &["gb", "gbc"],
        "wonderswan" | "wonderswancolor" => &["wonderswan", "wonderswancolor"],
        "ngp" | "ngpc" => &["ngp", "ngpc"],
        "nes" | "fds" => &["nes", "fds"],
        _ => &[],
    }
}

/// CRC (lowercase hex) -> official game name.
pub fn parse_dat(text: &str) -> HashMap<String, String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?s)game \(\s*name "((?:[^"\\]|\\.)*)".*?rom \( name (?:"[^"]*"|\S+) size \d+ crc ([0-9A-Fa-f]{8})"#).unwrap()
    });
    let mut out = HashMap::new();
    for c in re.captures_iter(text) {
        out.entry(c[2].to_lowercase())
            .or_insert_with(|| c[1].replace("\\\"", "\""));
    }
    out
}

/// Loads (and caches on disk) No-Intro DATs on demand.
pub struct Dats {
    cache: PathBuf,
    fetch: Fetcher,
    loaded: HashMap<String, Option<HashMap<String, String>>>,
}

impl Dats {
    pub fn new(cache: PathBuf, fetch: Fetcher) -> Dats {
        Dats {
            cache: cache.join("no-intro"),
            fetch,
            loaded: HashMap::new(),
        }
    }

    /// Whether this system's list has been loaded (or found missing) already.
    pub fn is_loaded(&self, system: &str) -> bool {
        self.loaded.contains_key(system)
    }

    pub fn get(&mut self, system: &str) -> Option<&HashMap<String, String>> {
        if !self.loaded.contains_key(system) {
            let m = self.load(system);
            self.loaded.insert(system.to_string(), m);
        }
        self.loaded.get(system).and_then(|m| m.as_ref())
    }

    fn load(&self, system: &str) -> Option<HashMap<String, String>> {
        let url = dat_url(system)?;
        let path = self.cache.join(format!("{system}.json"));
        let fresh = fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|a| a < MAX_AGE);
        let cached = || {
            fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
        };
        if fresh {
            if let Some(m) = cached() {
                return Some(m);
            }
        }
        match (self.fetch)(&url) {
            Ok(Some(body)) => {
                let map = parse_dat(&String::from_utf8_lossy(&body));
                let _ = fs::create_dir_all(&self.cache);
                let _ = fs::write(&path, serde_json::to_vec(&map).unwrap());
                Some(map)
            }
            _ => cached(), // offline: use a stale copy if there is one
        }
    }

    /// Find a file's system and official name from its CRCs, checking the
    /// detected system first, then the folder's system, then related ones.
    pub fn identify(
        &mut self,
        crcs: &[String],
        detected: &str,
        hint: Option<&str>,
    ) -> Option<(String, String)> {
        let mut order: Vec<&str> = vec![detected];
        order.extend(hint);
        order.extend(related(detected));
        if let Some(h) = hint {
            order.extend(related(h));
        }
        let mut seen = vec![];
        for sys in order {
            if seen.contains(&sys) {
                continue;
            }
            seen.push(sys);
            if let Some(map) = self.get(sys) {
                if let Some(name) = crcs.iter().find_map(|c| map.get(c)) {
                    return Some((sys.to_string(), name.clone()));
                }
            }
        }
        None
    }
}

/// CRCs to try for a file: its content CRC, plus the CRC without a copier
/// header for SNES dumps that carry one (No-Intro lists headerless data).
pub fn candidate_crcs(path: &Path, content_crc: &str) -> Vec<String> {
    let mut out = vec![content_crc.to_lowercase()];
    let ext = crate::detect::ext_of(path);
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    // NES dumps usually start with a 16-byte iNES header (plus a 512-byte
    // trainer when flagged); lists may hold the CRC of the data without it.
    if ext == "nes" {
        if let Ok(data) = fs::read(path) {
            if data.len() > 16 && data.starts_with(b"NES\x1a") {
                let skip = 16 + if data[6] & 4 != 0 { 512 } else { 0 };
                if data.len() > skip {
                    out.push(format!("{:08x}", crc32fast::hash(&data[skip..])));
                }
            }
        }
    }
    if matches!(ext.as_str(), "smc" | "sfc" | "swc" | "fig") && size % 1024 == 512 {
        if let Ok(mut f) = File::open(path) {
            let mut h = crc32fast::Hasher::new();
            let mut buf = vec![0u8; 1 << 20];
            if f.seek(SeekFrom::Start(512)).is_ok() {
                while let Ok(n) = f.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    h.update(&buf[..n]);
                }
                out.push(format!("{:08x}", h.finalize()));
            }
        }
    }
    out
}

/// Remove "003 - ", "012_", "0042. " style numbering from the start of a name.
/// Names like "1942 (Japan)" or "007 Racing" are left alone unless numbered.
pub fn strip_numbering(stem: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"^\s*\d{1,4}\s*(?:[-_.)\]]\s*|\s+)([A-Za-z'\[(].*|007.*)$").unwrap()
    });
    let c = re.captures(stem)?;
    // "007 - The World Is Not Enough", "007 Racing": the Bond games.
    if stem.trim_start().starts_with("007") {
        return None;
    }
    let rest = c[1].trim().to_string();
    // "1942 (Japan)" -> the number is the title itself.
    if rest.starts_with('(') || rest.starts_with('[') || naming::parse(&rest).title.is_empty() {
        return None;
    }
    Some(rest)
}

/// Should numbering be stripped for this group of names? Only when most of
/// them are numbered, so a lone "007 - The World Is Not Enough" is kept.
/// Numbering that's never part of a title: four digits starting with zero,
/// then a separator, like "0042 - " or "0137_". Shorter numbers such as
/// "003 - " are only stripped when most of their folder is numbered.
pub fn obviously_numbered(stem: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*0\d{3}\s*[-_.)\]]").unwrap())
        .is_match(stem)
        && strip_numbering(stem).is_some()
}

pub fn mostly_numbered<'a>(stems: impl Iterator<Item = &'a str>) -> bool {
    let (mut total, mut numbered) = (0, 0);
    for s in stems {
        total += 1;
        if strip_numbering(s).is_some() {
            numbered += 1;
        }
    }
    total >= 3 && numbered * 10 >= total * 6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_intro() {
        let dat = r#"
game (
	name "Sonic The Hedgehog (USA, Europe)"
	region "USA / Europe"
	serial "00001009-00"
	rom ( name "Sonic The Hedgehog (USA, Europe).md" size 524288 crc F9394E97 md5 X )
)
game (
	name "Addams Family, The (World)"
	rom ( name "Addams Family, The (World).gg" size 262144 crc 1D01F999 md5 X )
)
"#;
        let m = parse_dat(dat);
        assert_eq!(m["f9394e97"], "Sonic The Hedgehog (USA, Europe)");
        assert_eq!(m["1d01f999"], "Addams Family, The (World)");
    }

    #[test]
    fn numbering() {
        assert_eq!(
            strip_numbering("003 - Sonic The Hedgehog (USA)").as_deref(),
            Some("Sonic The Hedgehog (USA)")
        );
        assert_eq!(
            strip_numbering("123 Streets of Rage").as_deref(),
            Some("Streets of Rage")
        );
        assert_eq!(
            strip_numbering("0042_Golden Axe").as_deref(),
            Some("Golden Axe")
        );
        assert_eq!(strip_numbering("1942 (Japan)"), None);
        assert_eq!(strip_numbering("007 - The World Is Not Enough (USA)"), None);
        assert_eq!(strip_numbering("007 Racing (USA)"), None);
        assert_eq!(
            strip_numbering("0123 - 007 - Nightfire (USA)").as_deref(),
            Some("007 - Nightfire (USA)")
        );
        assert!(obviously_numbered("0042 - Golden Axe"));
        assert!(obviously_numbered("0137_Contra (USA)"));
        assert!(!obviously_numbered("007 - Nightfire (USA)"));
        assert!(!obviously_numbered("003 - Sonic"));
        assert!(!obviously_numbered("1942 (Japan)"));
        assert_eq!(strip_numbering("Sonic"), None);
        assert!(mostly_numbered(
            ["001 A", "002 B", "003 C", "D"].into_iter()
        ));
        assert!(!mostly_numbered(
            ["007 - The World Is Not Enough", "Goldeneye", "Perfect Dark"].into_iter()
        ));
    }
}
