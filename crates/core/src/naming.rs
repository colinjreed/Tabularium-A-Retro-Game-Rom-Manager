//! Parse No-Intro / Redump / GoodTools style ROM filenames.
//!
//! ```text
//! Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 2).gb
//! Legend of Zelda, The - Link's Awakening DX (USA, Europe) (Rev 2).gbc
//! Final Fantasy VII (USA) (Disc 2).chd
//! Pokemon - Red Version (USA, Europe) [T+Fre].gb
//! ```

use regex::Regex;
use std::collections::BTreeSet;
use std::sync::OnceLock;

/// Flags users can filter on.
pub const ALL_FLAGS: &[&str] = &[
    "aftermarket",
    "alternate",
    "bad",
    "beta",
    "bios",
    "demo",
    "fixed",
    "hack",
    "kiosk",
    "overdump",
    "pirate",
    "program",
    "promo",
    "proto",
    "sample",
    "trainer",
    "translation",
    "unlicensed",
    "virtualconsole",
];

/// The "remove the junk" preset.
pub const DEFAULT_JUNK: &[&str] = &[
    "beta", "proto", "demo", "sample", "kiosk", "program", "bios", "bad", "overdump", "trainer",
    "pirate",
];

pub fn flag_description(flag: &str) -> &'static str {
    match flag {
        "aftermarket" => "Aftermarket / homebrew releases",
        "alternate" => "Alternate dumps [a]",
        "bad" => "Bad dumps [b]",
        "beta" => "Betas",
        "bios" => "BIOS files",
        "demo" => "Demos and trials",
        "fixed" => "Fixed dumps [f]",
        "hack" => "Hacks",
        "kiosk" => "Kiosk / store demo units",
        "overdump" => "Overdumps [o]",
        "pirate" => "Pirate releases",
        "program" => "Test programs",
        "promo" => "Promotional releases",
        "proto" => "Prototypes",
        "sample" => "Samples",
        "trainer" => "Trainers [t]",
        "translation" => "Fan translations [T+]",
        "unlicensed" => "Unlicensed",
        "virtualconsole" => "Virtual Console / re-release extracts",
        _ => "",
    }
}

fn region(token: &str) -> Option<&'static str> {
    Some(match token {
        "usa" | "us" | "u" | "america" => "USA",
        "world" | "w" => "World",
        "europe" | "eu" | "e" => "Europe",
        "japan" | "jp" | "j" => "Japan",
        "uk" | "united kingdom" => "UK",
        "germany" | "g" => "Germany",
        "france" | "f" => "France",
        "spain" | "s" => "Spain",
        "italy" | "i" => "Italy",
        "netherlands" => "Netherlands",
        "sweden" => "Sweden",
        "denmark" => "Denmark",
        "norway" => "Norway",
        "finland" => "Finland",
        "portugal" => "Portugal",
        "korea" | "k" => "Korea",
        "china" | "c" => "China",
        "taiwan" => "Taiwan",
        "hong kong" => "Hong Kong",
        "asia" => "Asia",
        "australia" | "a" => "Australia",
        "brazil" | "b" => "Brazil",
        "canada" => "Canada",
        "russia" => "Russia",
        "scandinavia" => "Scandinavia",
        "latin america" => "Latin America",
        "unknown" => "Unknown",
        _ => return None,
    })
}

const LANG_CODES: &[&str] = &[
    "en", "ja", "fr", "de", "es", "it", "nl", "pt", "sv", "no", "da", "fi", "zh", "ko", "pl", "ru",
    "el", "tr", "cs", "hu", "ca", "hr",
];

struct Res {
    tag: Regex,
    rev: Regex,
    disc: Regex,
    article: Regex,
    lang: Regex,
    flags: Vec<(&'static str, Regex)>,
    good: Regex,
    strip_disc: Regex,
    bios: Regex,
    libretro_bad: Regex,
    ws: Regex,
}

fn res() -> &'static Res {
    static R: OnceLock<Res> = OnceLock::new();
    R.get_or_init(|| {
        let r = |s: &str| Regex::new(s).unwrap();
        Res {
            tag: r(r"\(([^()]*)\)|\[([^\[\]]*)\]"),
            rev: r(r"(?i)^(?:rev\s*([0-9a-z.]+)|v\s?([0-9][0-9a-z.]*))$"),
            disc: r(r"(?i)^(?:disc|disk|cd)\s*([0-9]+|[a-z])(?:\s+of\s+\d+)?$"),
            article: r(r"(?i)^(.*?), (The|A|An|Les|Le|La|Der|Die|Das|El|Il)( - .*)?$"),
            lang: r(r"^[a-z]{2}(\+[a-z]{2})*$"),
            flags: vec![
                ("beta", r(r"(?i)^beta( \S+)?$")),
                ("proto", r(r"(?i)^proto(type)?( \d+)?$")),
                ("demo", r(r"(?i)^(demo|trial|taikenban|playable demo|tech demo)( \d+)?$")),
                ("sample", r(r"(?i)^sample( \d+)?$")),
                ("kiosk", r(r"(?i)^kiosk( \S+)?$")),
                ("promo", r(r"(?i)^promo$")),
                ("unlicensed", r(r"(?i)^(unl|unlicensed)$")),
                ("pirate", r(r"(?i)^pirate$")),
                ("hack", r(r"(?i)^hack$")),
                ("aftermarket", r(r"(?i)^(aftermarket|homebrew)$")),
                ("program", r(r"(?i)^(test )?program$")),
                ("virtualconsole", r(r"(?i)virtual console|switch online|\bnso\b|classic mini|collection|evercade|gamecube|wii u|3ds")),
            ],
            good: r(r"^([a-z!])\d*$"),
            strip_disc: r(r"(?i)\s*\((?:disc|disk|cd)\s*[0-9a-z]+(?:\s+of\s+\d+)?\)"),
            bios: r(r"(?i)^\s*\[BIOS\]\s*"),
            libretro_bad: r(r#"[&*/:`<>?\\|"]"#),
            ws: r(r"\s+"),
        }
    })
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedName {
    /// Human title, tags removed, leading article restored.
    pub title: String,
    /// Normalised key used for grouping and matching.
    pub key: String,
    pub regions: Vec<String>,
    pub languages: Vec<String>,
    pub revision: Vec<u32>,
    pub disc: Option<u32>,
    pub flags: BTreeSet<String>,
    pub verified: bool,
    pub tags: Vec<String>,
}

fn rev_tuple(text: &str) -> Vec<u32> {
    let v: Vec<u32> = text
        .split('.')
        .map(|p| {
            if let Ok(n) = p.parse() {
                n
            } else if p.len() == 1 && p.chars().all(|c| c.is_ascii_alphabetic()) {
                (p.to_ascii_lowercase().as_bytes()[0] - b'a' + 1) as u32
            } else {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            }
        })
        .collect();
    if v.is_empty() {
        vec![0]
    } else {
        v
    }
}

/// Lowercase alphanumerics, leading articles dropped, '&' == 'and'.
pub fn title_key(title: &str) -> String {
    let mut t = title.to_lowercase().replace('&', " and ");
    for a in ["the ", "a ", "an "] {
        if let Some(rest) = t.strip_prefix(a) {
            t = rest.to_string();
            break;
        }
    }
    for a in [" - the ", " - a ", " - an "] {
        t = t.replace(a, " - ");
    }
    t.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// "Legend of Zelda, The - Link's Awakening" -> "The Legend of Zelda - Link's Awakening".
pub fn restore_article(title: &str) -> String {
    match res().article.captures(title) {
        Some(c) => format!("{} {}{}", &c[2], &c[1], c.get(3).map_or("", |m| m.as_str())),
        None => title.to_string(),
    }
}

/// Parse a filename stem (no extension).
pub fn parse(name: &str) -> ParsedName {
    let r = res();
    let mut p = ParsedName {
        revision: vec![0],
        ..Default::default()
    };
    if r.bios.is_match(name)
        || name
            .split(|c: char| !c.is_alphanumeric())
            .any(|w| w == "BIOS")
    {
        p.flags.insert("bios".into());
    }
    for cap in r.tag.captures_iter(name) {
        if let Some(paren) = cap.get(1) {
            let tag = paren.as_str().trim();
            p.tags.push(tag.to_string());
            let low: Vec<String> = tag.split(',').map(|s| s.trim().to_lowercase()).collect();
            let all_regions = low.iter().all(|t| region(t).is_some());
            let all_langs = low.iter().all(|t| LANG_CODES.contains(&t.as_str()));
            if all_regions && !all_langs {
                p.regions
                    .extend(low.iter().map(|t| region(t).unwrap().to_string()));
                continue;
            }
            if low
                .iter()
                .all(|t| LANG_CODES.contains(&t.as_str()) || r.lang.is_match(t))
            {
                for part in &low {
                    p.languages.extend(part.split('+').map(String::from));
                }
                continue;
            }
            if let Some(c) = r.rev.captures(tag) {
                p.revision = rev_tuple(c.get(1).or(c.get(2)).unwrap().as_str());
                continue;
            }
            if let Some(c) = r.disc.captures(tag) {
                let d = &c[1];
                p.disc =
                    Some(d.parse().unwrap_or_else(|_| {
                        (d.to_ascii_lowercase().as_bytes()[0] - b'a' + 1) as u32
                    }));
                continue;
            }
            for (flag, re) in &r.flags {
                if re.is_match(tag) {
                    p.flags.insert(flag.to_string());
                }
            }
        } else if let Some(br) = cap.get(2) {
            let tag = br.as_str().trim();
            p.tags.push(format!("[{tag}]"));
            if tag.eq_ignore_ascii_case("BIOS") {
                p.flags.insert("bios".into());
            } else if tag == "!" {
                p.verified = true;
            } else if tag.starts_with("T+") || tag.starts_with("T-") {
                p.flags.insert("translation".into());
            } else if tag.len() <= 4 {
                if let Some(c) = r.good.captures(tag) {
                    let f = match &c[1] {
                        "b" => "bad",
                        "o" => "overdump",
                        "h" => "hack",
                        "t" => "trainer",
                        "f" => "fixed",
                        "p" => "pirate",
                        "a" => "alternate",
                        _ => "",
                    };
                    if !f.is_empty() {
                        p.flags.insert(f.into());
                    }
                }
            }
        }
    }
    let title = r.tag.replace_all(name, "");
    let title = r.bios.replace(&title, "");
    let title = r.ws.replace_all(&title, " ");
    let title = title.trim_matches(|c: char| c.is_whitespace() || "-_.".contains(c));
    p.title = restore_article(title);
    p.key = title_key(&p.title);
    p
}

/// Make a name safe for libretro thumbnail lookups.
pub fn libretro_safe(name: &str) -> String {
    res().libretro_bad.replace_all(name, "_").into_owned()
}

/// "Final Fantasy VII (USA) (Disc 1)" -> "Final Fantasy VII (USA)".
pub fn strip_disc(name: &str) -> String {
    res().strip_disc.replace_all(name, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_revision_disc() {
        let p = parse("Final Fantasy VII (USA) (Disc 2)");
        assert_eq!(p.title, "Final Fantasy VII");
        assert_eq!(p.regions, vec!["USA"]);
        assert_eq!(p.disc, Some(2));
        let p = parse("Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 2)");
        assert_eq!(p.regions, vec!["USA", "Europe"]);
        assert_eq!(p.revision, vec![2]);
    }

    #[test]
    fn article_and_key() {
        let p = parse(
            "Legend of Zelda, The - Link's Awakening DX (USA, Europe) (Rev 2) (SGB Enhanced)",
        );
        assert_eq!(p.title, "The Legend of Zelda - Link's Awakening DX");
        assert_eq!(
            p.key,
            parse("The Legend of Zelda - Link's Awakening DX (Japan)").key
        );
    }

    #[test]
    fn flags() {
        assert!(parse("Sonic (Beta 2)").flags.contains("beta"));
        assert!(parse("Sonic (Proto)").flags.contains("proto"));
        assert!(parse("Game (USA) (Unl)").flags.contains("unlicensed"));
        assert!(parse("[BIOS] Game Boy Advance (World)")
            .flags
            .contains("bios"));
        assert!(parse("Mother 3 (Japan) [T+Eng1.3]")
            .flags
            .contains("translation"));
        assert!(parse("Contra (U) [b1]").flags.contains("bad"));
        let p = parse("Contra (U) [!]");
        assert!(p.verified && p.regions == vec!["USA"] && p.flags.is_empty());
        assert!(parse("Metroid (USA) (Virtual Console)")
            .flags
            .contains("virtualconsole"));
        assert_eq!(
            parse("Tintin (Europe) (En,Fr,De)").languages,
            vec!["en", "fr", "de"]
        );
    }

    #[test]
    fn helpers() {
        assert_eq!(
            libretro_safe("Q*bert: Arcade & More"),
            "Q_bert_ Arcade _ More"
        );
        assert_eq!(strip_disc("FF7 (USA) (Disc 1)"), "FF7 (USA)");
    }
}

/// Parse a MAME / FBNeo full name such as
/// "Street Fighter II': Champion Edition (Japan 920513)" or
/// "Final Fight (US, rev 1)". Regions and flags hide inside free-form tags.
pub fn parse_arcade(name: &str) -> ParsedName {
    let mut p = parse(name);
    for tag in p.tags.clone() {
        if tag.starts_with('[') {
            continue;
        }
        for part in tag.split(',') {
            let part = part.trim().to_lowercase();
            let first = part.split_whitespace().next().unwrap_or("");
            let first = match first {
                "us" | "usa" => "usa",
                "euro" | "europe" => "europe",
                other => other,
            };
            if let Some(r) = region(first).filter(|_| first.len() > 1) {
                if !p.regions.iter().any(|x| x == r) {
                    p.regions.push(r.to_string());
                }
            }
            if part.contains("bootleg") {
                p.flags.insert("pirate".into());
            }
            if part.contains("prototype") || part.contains("location test") {
                p.flags.insert("proto".into());
            }
            if part.split_whitespace().any(|w| w == "hack") {
                p.flags.insert("hack".into());
            }
            if part.contains("demo") {
                p.flags.insert("demo".into());
            }
            if let Some(rest) = part.strip_prefix("rev ") {
                p.revision = rev_tuple(rest.trim());
            }
        }
    }
    p
}

#[cfg(test)]
mod arcade_tests {
    use super::*;

    #[test]
    fn arcade_names() {
        let p = parse_arcade("Street Fighter II': Champion Edition (Japan 920513)");
        assert_eq!(p.title, "Street Fighter II': Champion Edition");
        assert_eq!(p.regions, vec!["Japan"]);
        let w = parse_arcade("Street Fighter II': Champion Edition (World 920513)");
        assert_eq!(p.key, w.key);
        assert!(parse_arcade("Final Fight (bootleg)")
            .flags
            .contains("pirate"));
        let f = parse_arcade("Final Fight (US, rev 1)");
        assert_eq!(f.regions, vec!["USA"]);
        assert_eq!(f.revision, vec![1]);
    }
}
