//! BIOS / firmware files: what each system needs, checking and installing.
//!
//! Hashes come from libretro's System.dat (libretro-database). "Required"
//! means games won't boot without it in the usual RetroArch core on
//! handhelds; "optional" files have a built-in (HLE) substitute.

use crate::hashing::md5_file;
use crate::profiles::Profile;
use crate::systems;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq)]
pub struct BiosFile {
    /// Path relative to the BIOS folder, e.g. "dc/dc_boot.bin".
    pub name: &'static str,
    /// Accepted hashes; empty = any (name check only).
    pub md5: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Requirement {
    pub system: &'static str,
    pub files: &'static [BiosFile],
    pub required: bool,
    /// One of the files is enough (e.g. PS1 regions).
    pub any_of: bool,
    pub note: &'static str,
}

macro_rules! f {
    ($n:expr) => { BiosFile { name: $n, md5: &[] } };
    ($n:expr, $($m:expr),+) => { BiosFile { name: $n, md5: &[$($m),+] } };
}

pub static REQUIREMENTS: &[Requirement] = &[
    Requirement {
        system: "psx",
        required: true,
        any_of: true,
        files: &[
            f!("scph5501.bin", "490f666e1afb15b7362b406ed1cea246"),
            f!("scph5500.bin", "8dd7d5296a650fac7319bce665a6a53c"),
            f!("scph5502.bin", "32736f17079d0b2b7024407c39bd3050"),
            f!("scph1001.bin", "924e392ed05558ffdb115408c263dccf"),
        ],
        note: "PCSX ReARMed can run without one, but many games need a real BIOS. \
               scph5501 = USA, scph5500 = Japan, scph5502 = Europe.",
    },
    Requirement {
        system: "segacd",
        required: true,
        any_of: true,
        files: &[
            f!("bios_CD_U.bin", "2efd74e3232ff260e371b99f84024f7f"),
            f!("bios_CD_E.bin", "e66fa1dc5820d254611fdcdba0662372"),
            f!("bios_CD_J.bin", "278a9397d192149e84e820ac621a8edd"),
        ],
        note: "Genesis Plus GX / PicoDrive use the BIOS matching each game's region \
               (U = USA, E = Europe, J = Japan).",
    },
    Requirement {
        system: "saturn",
        required: true,
        any_of: true,
        files: &[
            f!("mpr-17933.bin", "3240872c70984b6cbfda1586cab68dbe"),
            f!("sega_101.bin", "85ec9ca47d8f6807718151cbcca8b964"),
            f!("saturn_bios.bin", "af5828fdff51384f99b3c4926be27762"),
        ],
        note: "Beetle Saturn: mpr-17933 (US/EU) or sega_101 (JP). YabaSanshiro: saturn_bios.bin.",
    },
    Requirement {
        system: "dreamcast",
        required: true,
        any_of: false,
        files: &[f!("dc/dc_boot.bin", "e10c53c2f8b90bab96ead2d368858623")],
        note: "Flycast. Some builds include an HLE BIOS, with lower compatibility.",
    },
    Requirement {
        system: "pcenginecd",
        required: true,
        any_of: false,
        files: &[f!(
            "syscard3.pce",
            "38179df8f4ac870017db21ebcbf53114",
            "0754f903b52e3b3342202bdafb13efa5"
        )],
        note: "System Card 3.0 for Beetle PCE / PCE Fast.",
    },
    Requirement {
        system: "fds",
        required: true,
        any_of: false,
        files: &[f!("disksys.rom", "ca30b50f880eb660a320674ed365ef7a")],
        note: "",
    },
    Requirement {
        system: "atarilynx",
        required: true,
        any_of: false,
        files: &[f!("lynxboot.img", "fcd403db69f54290b51035d82f835e7b")],
        note: "Handy core.",
    },
    Requirement {
        system: "colecovision",
        required: true,
        any_of: false,
        files: &[f!("colecovision.rom", "2c66f5911e5b42b8ebe113403548eee7")],
        note: "",
    },
    Requirement {
        system: "arcade",
        required: false,
        any_of: false,
        files: &[f!("neogeo.zip")],
        note: "Needed for Neo Geo games only. Must match your MAME/FBNeo romset version, \
               so only the name is checked.",
    },
    Requirement {
        system: "nds",
        required: false,
        any_of: false,
        files: &[
            f!("bios7.bin", "df692a80a5b1bc90728bc3dfc76cd948"),
            f!("bios9.bin", "a392174eb3e572fed6447e956bde4b25"),
            f!("firmware.bin", "e45033d9b0fa6b0de071292bba7c9d13"),
        ],
        note: "DeSmuME runs without them; melonDS / DraStic benefit.",
    },
    Requirement {
        system: "gba",
        required: false,
        any_of: false,
        files: &[f!("gba_bios.bin", "a860e8c0b6d573d191e4ec7db1b1e4f6")],
        note: "mGBA / gpSP have an HLE BIOS; a few games need the real one.",
    },
    Requirement {
        system: "gb",
        required: false,
        any_of: false,
        files: &[f!("gb_bios.bin", "32fbbd84168d3482956eb3c5051637f5")],
        note: "Boot logo only.",
    },
    Requirement {
        system: "gbc",
        required: false,
        any_of: false,
        files: &[f!("gbc_bios.bin", "dbfce9db9deaa2567f6a84fde55f9680")],
        note: "Boot logo only.",
    },
    Requirement {
        system: "pokemini",
        required: false,
        any_of: false,
        files: &[f!("bios.min", "1e4fb124a3a886865acb574f388c803d")],
        note: "",
    },
    Requirement {
        system: "atari7800",
        required: false,
        any_of: false,
        files: &[f!("7800 BIOS (U).rom", "0763f1ffb006ddbe32e52d497ee848ae")],
        note: "",
    },
];

/// Requirements for these systems (None = all).
pub fn requirements_for(systems: Option<&[String]>) -> Vec<&'static Requirement> {
    REQUIREMENTS
        .iter()
        .filter(|r| systems.is_none_or(|s| s.iter().any(|x| x == r.system)))
        .collect()
}

fn base_name(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Ok,
    Missing,
    BadHash(String),
    /// Right file, wrong place (card checks only).
    Misplaced(PathBuf),
}

#[derive(Debug, Clone)]
pub struct FileStatus {
    pub file: &'static BiosFile,
    pub state: FileState,
    pub found: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ReqStatus {
    pub req: &'static Requirement,
    pub files: Vec<FileStatus>,
}

impl ReqStatus {
    pub fn satisfied(&self) -> bool {
        let mut ok = self.files.iter().map(|f| f.state == FileState::Ok);
        if self.req.any_of {
            ok.any(|x| x)
        } else {
            ok.all(|x| x)
        }
    }
}

const MAX_BIOS: u64 = 64 * 1024 * 1024;

#[derive(Default)]
struct FolderIndex {
    by_md5: HashMap<String, PathBuf>,
    by_name: HashMap<String, PathBuf>,
}

fn index_folder(folder: &Path) -> FolderIndex {
    let mut idx = FolderIndex::default();
    for e in WalkDir::new(folder)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
    {
        let p = e.path().to_path_buf();
        idx.by_name
            .entry(e.file_name().to_string_lossy().to_lowercase())
            .or_insert(p.clone());
        if e.metadata().map(|m| m.len() <= MAX_BIOS).unwrap_or(false) {
            if let Ok(h) = md5_file(&p) {
                idx.by_md5.entry(h).or_insert(p);
            }
        }
    }
    idx
}

/// Check BIOS files in `folder`.
///
/// With a profile, `folder` is an SD card root and each system's BIOS
/// folder for that firmware is checked; otherwise files are looked for
/// anywhere under `folder`.
pub fn check(
    folder: &Path,
    systems: Option<&[String]>,
    layout: Option<&Profile>,
) -> Vec<ReqStatus> {
    let mut cache: HashMap<PathBuf, FolderIndex> = HashMap::new();
    let card_index = layout.map(|_| index_folder(folder));
    let mut out = vec![];
    for req in requirements_for(systems) {
        let base = match (layout, systems::get(req.system)) {
            (Some(p), Some(s)) => folder.join(p.bios_dir(s)),
            _ => folder.to_path_buf(),
        };
        let idx = cache
            .entry(base.clone())
            .or_insert_with(|| index_folder(&base));
        let mut files = vec![];
        for bf in req.files {
            let expected = base.join(bf.name);
            let (state, found) = if expected.is_file() {
                if bf.md5.is_empty() {
                    (FileState::Ok, Some(expected))
                } else {
                    match md5_file(&expected) {
                        Ok(h) if bf.md5.contains(&h.as_str()) => (FileState::Ok, Some(expected)),
                        Ok(h) => (FileState::BadHash(h), Some(expected)),
                        Err(_) => (FileState::Missing, None),
                    }
                }
            } else if layout.is_none() {
                let hit = bf
                    .md5
                    .iter()
                    .find_map(|m| idx.by_md5.get(*m))
                    .cloned()
                    .or_else(|| {
                        let named = idx.by_name.get(&base_name(bf.name).to_lowercase())?;
                        (bf.md5.is_empty()
                            || md5_file(named).is_ok_and(|h| bf.md5.contains(&h.as_str())))
                        .then(|| named.clone())
                    });
                match hit {
                    Some(p) => (FileState::Ok, Some(p)),
                    None => (FileState::Missing, None),
                }
            } else {
                // On a card: is the file somewhere else on it?
                let ci = card_index.as_ref().unwrap();
                match bf.md5.iter().find_map(|m| ci.by_md5.get(*m)) {
                    Some(p) => (FileState::Misplaced(p.clone()), Some(p.clone())),
                    None => (FileState::Missing, None),
                }
            };
            files.push(FileStatus {
                file: bf,
                state,
                found,
            });
        }
        out.push(ReqStatus { req, files });
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    Imported(String),
    AlreadyPresent(String),
    BadHash(String),
}

fn known_by_md5(h: &str) -> Option<&'static BiosFile> {
    REQUIREMENTS
        .iter()
        .flat_map(|r| r.files.iter())
        .find(|f| f.md5.contains(&h))
}

fn known_by_name(name: &str) -> Option<&'static BiosFile> {
    let n = name.to_lowercase();
    REQUIREMENTS
        .iter()
        .flat_map(|r| r.files.iter())
        .find(|f| base_name(f.name).to_lowercase() == n)
}

/// Copy recognised BIOS files (by hash, or by name for name-only entries)
/// into `dest`, under their correct names. Unknown files are ignored.
pub fn import_files(
    sources: &[PathBuf],
    dest: &Path,
    dry_run: bool,
) -> Vec<(PathBuf, ImportOutcome)> {
    let mut out = vec![];
    for src in sources {
        for e in WalkDir::new(src)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
        {
            if e.metadata().map(|m| m.len() > MAX_BIOS).unwrap_or(true) {
                continue;
            }
            let p = e.path().to_path_buf();
            let Ok(h) = md5_file(&p) else { continue };
            let file_name = e.file_name().to_string_lossy();
            let bf = match known_by_md5(&h) {
                Some(bf) => bf,
                None => match known_by_name(&file_name) {
                    Some(bf) if bf.md5.is_empty() => bf,
                    Some(bf) => {
                        out.push((
                            p,
                            ImportOutcome::BadHash(format!(
                                "named like {} but its MD5 {h} isn't a known good dump",
                                bf.name
                            )),
                        ));
                        continue;
                    }
                    None => continue,
                },
            };
            let target = dest.join(bf.name);
            if target.is_file() && md5_file(&target).is_ok_and(|t| t == h) {
                out.push((p, ImportOutcome::AlreadyPresent(bf.name.into())));
                continue;
            }
            if !dry_run {
                if let Some(parent) = target.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::copy(&p, &target).is_err() {
                    continue;
                }
            }
            out.push((p, ImportOutcome::Imported(bf.name.into())));
        }
    }
    out
}

#[derive(Debug, Clone, Default)]
pub struct InstallReport {
    pub copied: Vec<String>,
    pub missing_required: Vec<String>,
    pub missing_optional: Vec<String>,
}

/// Copy the BIOS files these systems use from the library to a card.
pub fn install(
    library_bios: &Path,
    card: &Path,
    profile: &Profile,
    systems_list: &[String],
    dry_run: bool,
) -> InstallReport {
    let mut rep = InstallReport::default();
    if systems_list.is_empty() {
        return rep;
    }
    for rs in check(library_bios, Some(systems_list), None) {
        let Some(sys) = systems::get(rs.req.system) else {
            continue;
        };
        let dest_dir = card.join(profile.bios_dir(sys));
        for fs_ in &rs.files {
            if let (FileState::Ok, Some(found)) = (&fs_.state, &fs_.found) {
                let target = dest_dir.join(fs_.file.name);
                let same = fs::metadata(&target).ok().map(|m| m.len())
                    == fs::metadata(found).ok().map(|m| m.len());
                if !same {
                    if !dry_run {
                        if let Some(p) = target.parent() {
                            let _ = fs::create_dir_all(p);
                        }
                        let _ = fs::copy(found, &target);
                    }
                    rep.copied
                        .push(format!("{}: {}", rs.req.system, fs_.file.name));
                }
            }
        }
        if !rs.satisfied() {
            let names: Vec<&str> = rs
                .files
                .iter()
                .filter(|f| f.state != FileState::Ok)
                .map(|f| f.file.name)
                .collect();
            let joined = names.join(if rs.req.any_of { " or " } else { ", " });
            let label = format!("{}: {joined}", systems::name_of(rs.req.system));
            if rs.req.required {
                rep.missing_required.push(label)
            } else {
                rep.missing_optional.push(label)
            }
        }
    }
    rep
}
