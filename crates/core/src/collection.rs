//! Collections: saved selections of the library exported to an SD card.
//!
//! A collection is a JSON file in LIBRARY/collections/ naming a firmware
//! profile, the systems to include, filters, an optional size budget and the
//! art type. Exporting never modifies the library.

use crate::filters::{self, FilterSpec};
use crate::library::{Library, Rom};
use crate::profiles::{self, GamelistEntry, Profile};
use crate::progress::Reporter;
use crate::{bios, naming, systems, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const MANIFEST_DIR: &str = ".tabularium";
/// Where earlier versions (as "ROM Manager") kept card manifests.
const OLD_MANIFEST_DIR: &str = ".rom-manager";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Collection {
    pub name: String,
    pub profile: String,
    pub systems: Vec<String>,
    pub filters: FilterSpec,
    pub max_size: Option<u64>,
    pub priority_titles: Vec<String>,
    pub art_kind: Option<String>,
    pub multidisc_m3u: bool,
    pub folder_overrides: BTreeMap<String, String>,
    /// Hand-picked games: always on the card, whatever the rules say.
    #[serde(alias = "include_ids")]
    pub picked: Vec<i64>,
    pub exclude_ids: Vec<i64>,
    /// Also add games chosen by the rules (systems + filters).
    pub use_rules: bool,
    /// Pick this many games at random from those the rules match.
    pub random_count: Option<usize>,
    /// Seed for the random pick, so the choice stays put until reshuffled.
    pub random_seed: u64,
}

impl Default for Collection {
    fn default() -> Self {
        Collection {
            name: String::new(),
            profile: "onion".into(),
            systems: vec![],
            filters: FilterSpec::default(),
            max_size: None,
            priority_titles: vec![],
            art_kind: None,
            multidisc_m3u: true,
            folder_overrides: BTreeMap::new(),
            picked: vec![],
            exclude_ids: vec![],
            use_rules: true,
            random_count: None,
            random_seed: 1,
        }
    }
}

pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || " ._-".contains(c))
        && !name.starts_with('.')
}

fn path_for(lib: &Library, name: &str) -> Result<PathBuf> {
    if !valid_name(name) {
        return Err(Error::Other(
            "Collection names may contain letters, numbers, spaces, . _ -".into(),
        ));
    }
    Ok(lib.root.join("collections").join(format!("{name}.json")))
}

impl Collection {
    pub fn load(lib: &Library, name: &str) -> Result<Collection> {
        let p = path_for(lib, name)?;
        let data =
            fs::read(&p).map_err(|_| Error::Other(format!("No collection named '{name}'")))?;
        let mut c: Collection =
            serde_json::from_slice(&data).map_err(|e| Error::Other(e.to_string()))?;
        c.name = name.to_string();
        Ok(c)
    }

    pub fn save(&self, lib: &Library) -> Result<()> {
        let p = path_for(lib, &self.name)?;
        fs::create_dir_all(p.parent().unwrap())?;
        fs::write(p, serde_json::to_vec_pretty(self).unwrap())?;
        Ok(())
    }

    pub fn delete(lib: &Library, name: &str) -> Result<()> {
        fs::remove_file(path_for(lib, name)?)?;
        Ok(())
    }

    pub fn profile(&self) -> &'static Profile {
        profiles::get(&self.profile).unwrap_or(&profiles::PROFILES[0])
    }
}

pub fn list(lib: &Library) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(lib.root.join("collections"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension()? != "json" {
                return None;
            }
            Some(p.file_stem()?.to_string_lossy().to_string())
        })
        .collect();
    v.sort_by_key(|s| s.to_lowercase());
    v
}

// ------------------------------------------------------------------ resolving

#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub roms: Vec<Rom>,
    pub rejected: Vec<filters::Rejected>,
    pub over_budget: Vec<Rom>,
}

impl Selection {
    pub fn size(&self) -> u64 {
        self.roms.iter().map(|r| r.total_size).sum()
    }
}

/// Group discs of one release so a size budget never splits a game.
fn units(roms: Vec<Rom>) -> Vec<Vec<Rom>> {
    let mut groups: BTreeMap<(String, String), Vec<Rom>> = BTreeMap::new();
    for r in roms {
        groups
            .entry((r.system.clone(), filters::variant_key(&r)))
            .or_default()
            .push(r);
    }
    groups
        .into_values()
        .map(|mut g| {
            g.sort_by_key(|r| r.disc.unwrap_or(0));
            g
        })
        .collect()
}

/// Small deterministic shuffle (splitmix64), so a seed always gives the same pick.
fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut next = || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    for i in (1..items.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

pub fn resolve(lib: &Library, col: &Collection) -> Result<Selection> {
    // Hand-picked games come first and ignore the rules.
    let mut picked: Vec<Rom> = vec![];
    for id in &col.picked {
        if let Ok(r) = lib.get(*id) {
            if r.status == "active" && !picked.iter().any(|p: &Rom| p.id == r.id) {
                picked.push(r);
            }
        }
    }
    let picked_ids: HashSet<i64> = picked.iter().map(|r| r.id).collect();
    let (mut kept, mut rejected) = if col.use_rules {
        let pool: Vec<Rom> = lib
            .roms(&col.systems, "active")?
            .into_iter()
            .filter(|r| !picked_ids.contains(&r.id))
            .collect();
        filters::apply(pool, &col.filters)
    } else {
        (vec![], vec![])
    };
    if let (true, Some(n)) = (col.use_rules, col.random_count) {
        let mut groups = units(kept);
        shuffle(&mut groups, col.random_seed);
        let not_chosen: Vec<Rom> = groups.iter().skip(n).flatten().cloned().collect();
        rejected.extend(not_chosen.into_iter().map(|rom| filters::Rejected {
            rom,
            reason: "not in this random pick".into(),
        }));
        kept = groups.into_iter().take(n).flatten().collect();
    }
    let mut all = picked;
    all.extend(kept);
    let mut kept = all;
    if !col.exclude_ids.is_empty() {
        let (ex, keep): (Vec<Rom>, Vec<Rom>) = kept
            .into_iter()
            .partition(|r| col.exclude_ids.contains(&r.id));
        rejected.extend(ex.into_iter().map(|rom| filters::Rejected {
            rom,
            reason: "excluded by hand".into(),
        }));
        kept = keep;
    }
    let mut over = vec![];
    if let Some(budget) = col.max_size {
        let all = units(kept);
        let is_prio = |u: &Vec<Rom>| {
            u.iter().any(|r| picked_ids.contains(&r.id))
                || (!col.priority_titles.is_empty()
                    && u.iter()
                        .any(|r| filters::matches_any(r, &col.priority_titles)))
        };
        let (prio, rest): (Vec<_>, Vec<_>) = all.into_iter().partition(|u| is_prio(u));
        // Round-robin across systems so one big system can't starve the others.
        let mut by_sys: BTreeMap<String, std::collections::VecDeque<Vec<Rom>>> = BTreeMap::new();
        for u in rest {
            by_sys.entry(u[0].system.clone()).or_default().push_back(u);
        }
        let mut order = prio;
        while by_sys.values().any(|q| !q.is_empty()) {
            for q in by_sys.values_mut() {
                if let Some(u) = q.pop_front() {
                    order.push(u);
                }
            }
        }
        let mut used = 0u64;
        kept = vec![];
        for u in order {
            let size: u64 = u.iter().map(|r| r.total_size).sum();
            if used + size <= budget {
                used += size;
                kept.extend(u);
            } else {
                over.extend(u);
            }
        }
    }
    kept.sort_by(|a, b| (&a.system, &a.title, a.disc).cmp(&(&b.system, &b.title, b.disc)));
    Ok(Selection {
        roms: kept,
        rejected,
        over_budget: over,
    })
}

// ------------------------------------------------------------------ export

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub dry_run: bool,
    pub sync: bool,
    pub art: bool,
    pub bios: bool,
    pub ignore_free_space: bool,
    /// Read play history from the card before copying.
    pub read_history: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            dry_run: false,
            sync: false,
            art: true,
            bios: true,
            ignore_free_space: false,
            read_history: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExportReport {
    pub roms: usize,
    pub copied: usize,
    pub unchanged: usize,
    pub art: usize,
    pub art_missing: usize,
    pub removed: usize,
    pub bytes_copied: u64,
    pub bios: bios::InstallReport,
    pub history: Option<crate::playhistory::PlayReport>,
    pub warnings: Vec<String>,
    pub planned: Vec<String>,
    pub cancelled: bool,
}

/// PNG bytes, resized to `width` if wider.
pub fn prepare_art(src: &Path, width: Option<u32>) -> std::io::Result<Vec<u8>> {
    let data = fs::read(src)?;
    let is_png = data.starts_with(b"\x89PNG");
    let Ok(img) = image::load_from_memory(&data) else {
        return Ok(data);
    };
    let needs_resize = width.is_some_and(|w| img.width() > w);
    if is_png && !needs_resize {
        return Ok(data);
    }
    let img = match width {
        Some(w) if needs_resize => {
            let h = ((img.height() as f64) * (w as f64) / (img.width() as f64))
                .round()
                .max(1.0) as u32;
            img.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
        }
        _ => img,
    };
    let mut out = std::io::Cursor::new(vec![]);
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(out.into_inner())
}

const FAT_BAD: &[char] = &['<', '>', ':', '"', '|', '?', '*'];
const FAT32_MAX: u64 = 4 * 1024 * 1024 * 1024 - 1;

fn free_space(path: &Path) -> Option<u64> {
    fs4::available_space(path).ok()
}

struct Writer<'a> {
    dest: &'a Path,
    dry_run: bool,
    written: BTreeSet<String>,
    report: ExportReport,
}

impl Writer<'_> {
    fn copy(&mut self, src: &Path, rel: &str) -> std::io::Result<()> {
        self.written.insert(rel.to_string());
        let dst = self.dest.join(rel);
        let size = fs::metadata(src)?.len();
        if fs::metadata(&dst).map(|m| m.len() == size).unwrap_or(false) {
            self.report.unchanged += 1;
            return Ok(());
        }
        self.report.planned.push(rel.to_string());
        self.report.copied += 1;
        self.report.bytes_copied += size;
        if self.dry_run {
            return Ok(());
        }
        fs::create_dir_all(dst.parent().unwrap())?;
        let tmp = dst.with_file_name(format!(
            "{}.part",
            dst.file_name().unwrap().to_string_lossy()
        ));
        fs::copy(src, &tmp)?;
        fs::rename(&tmp, &dst)
    }

    fn copy_dir(&mut self, src: &Path, rel: &str) -> std::io::Result<()> {
        for e in walkdir::WalkDir::new(src).into_iter().flatten() {
            if e.file_type().is_file() {
                let sub = e.path().strip_prefix(src).unwrap_or(e.path());
                let sub = sub.to_string_lossy().replace('\\', "/");
                self.copy(e.path(), &format!("{rel}/{sub}"))?;
            }
        }
        Ok(())
    }

    fn write(&mut self, rel: &str, data: &[u8]) -> std::io::Result<()> {
        self.written.insert(rel.to_string());
        let dst = self.dest.join(rel);
        if fs::read(&dst).map(|d| d == data).unwrap_or(false) {
            return Ok(());
        }
        self.report.planned.push(rel.to_string());
        if self.dry_run {
            return Ok(());
        }
        fs::create_dir_all(dst.parent().unwrap())?;
        fs::write(dst, data)
    }
}

/// Folder name for a DOS / ScummVM game on the card. ScummVM folders end in
/// `.scummvm` (Batocera, ES-DE, ROCKNIX, muOS); Batocera's DOS folders end in `.pc`.
fn folder_game_name(profile: &Profile, system: &str, stem: &str, dir: &Path) -> String {
    match (system, profile.id) {
        ("scummvm", _) => format!("{stem}.scummvm"),
        ("dos", "batocera") => format!("{stem}.pc"),
        _ => file_name(dir),
    }
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn rel_from(dir: &str, target: &str) -> String {
    match target.strip_prefix(&format!("{dir}/")) {
        Some(r) => format!("./{r}"),
        None => target.to_string(),
    }
}

pub fn export(
    lib: &Library,
    col: &Collection,
    dest: &Path,
    opts: &ExportOptions,
    rep: &dyn Reporter,
) -> Result<ExportReport> {
    let profile = col.profile();
    let art_kind = col.art_kind.clone().unwrap_or_else(|| "boxart".into());
    // Pick up what was played on the card before touching it.
    let history = if opts.read_history && dest.exists() {
        match crate::playhistory::read_card(lib, dest, profile, &col.folder_overrides, rep) {
            Ok(h) => Some(h),
            Err(e) => {
                rep.log(format!("Couldn't read play history: {e}"));
                None
            }
        }
    } else {
        None
    };
    let sel = resolve(lib, col)?;
    if !opts.dry_run {
        fs::create_dir_all(dest)?;
    }
    if !opts.dry_run && !opts.ignore_free_space {
        if let Some(free) = free_space(dest) {
            let need = sel.size();
            if need > free {
                return Err(Error::Other(format!(
                    "This collection needs {} but the destination has {} free. Set a size limit on the collection.",
                    crate::human_size(need),
                    crate::human_size(free)
                )));
            }
        }
    }
    let overrides = &col.folder_overrides;
    let mut w = Writer {
        dest,
        dry_run: opts.dry_run,
        written: BTreeSet::new(),
        report: ExportReport::default(),
    };
    w.report.roms = sel.roms.len();
    w.report.history = history;
    let mut gamelists: BTreeMap<String, Vec<GamelistEntry>> = BTreeMap::new();
    let mut warnings: BTreeSet<String> = BTreeSet::new();
    let all_units = units(sel.roms.clone());
    let total = all_units.len();

    let place_art = |w: &mut Writer,
                     rom: &Rom,
                     sys: &systems::System,
                     entry_rel: &str|
     -> Result<Option<String>> {
        if !opts.art {
            return Ok(None);
        }
        let media = lib.media(rom.id)?;
        let src = media
            .get(&art_kind)
            .or_else(|| media.get("boxart"))
            .or_else(|| media.values().next());
        let Some(src) = src else {
            w.report.art_missing += 1;
            return Ok(None);
        };
        let rel = profile.art_path(sys, entry_rel, overrides, &art_kind);
        let data = prepare_art(src, profile.art_width)?;
        w.write(&rel, &data)?;
        w.report.art += 1;
        Ok(Some(rel))
    };

    for (i, unit) in all_units.iter().enumerate() {
        if rep.cancelled() {
            w.report.cancelled = true;
            break;
        }
        let first = &unit[0];
        rep.progress(i, total, &first.stem);
        let Some(sys) = systems::get(&first.system) else {
            continue;
        };
        let sys_dir = profile.rom_dir(sys, overrides);
        let multidisc = col.multidisc_m3u
            && sys.disc_based
            && unit.len() > 1
            && unit.iter().all(|r| r.disc.is_some());
        if multidisc {
            let base = naming::strip_disc(&first.stem);
            let hidden = format!("{sys_dir}/.{base}");
            let mut lines = vec![];
            for r in unit {
                for f in lib.abspaths(r) {
                    w.copy(&f, &format!("{hidden}/{}", file_name(&f)))?;
                }
                lines.push(format!(".{base}/{}", r.file_name()));
            }
            let m3u = format!("{sys_dir}/{base}.m3u");
            w.write(&m3u, (lines.join("\n") + "\n").as_bytes())?;
            let image = place_art(&mut w, first, sys, &m3u)?;
            gamelists
                .entry(sys_dir.clone())
                .or_default()
                .push(GamelistEntry {
                    path: format!("./{base}.m3u"),
                    name: naming::strip_disc(&first.title),
                    image: image.map(|i| rel_from(&sys_dir, &i)),
                });
            continue;
        }
        for r in unit {
            let mut main_name = r.file_name().to_string();
            for f in lib.abspaths(r) {
                if f.is_dir() {
                    main_name = folder_game_name(profile, sys.id, &r.stem, &f);
                    if sys.id == "scummvm" && crate::folder_games::scummvm_id(&f).is_none() {
                        warnings.insert(format!(
                            "ScummVM game '{}' has no game ID file, so it may not launch",
                            r.stem
                        ));
                    }
                    w.copy_dir(&f, &format!("{sys_dir}/{main_name}"))?;
                    continue;
                }
                let name = file_name(&f);
                if name.contains(FAT_BAD) {
                    warnings.insert(format!("'{name}' has characters FAT/exFAT cards reject"));
                }
                if fs::metadata(&f)
                    .map(|m| m.len() > FAT32_MAX)
                    .unwrap_or(false)
                {
                    warnings.insert(format!(
                        "'{name}' is over 4 GB: the card must be exFAT, not FAT32"
                    ));
                }
                w.copy(&f, &format!("{sys_dir}/{name}"))?;
            }
            let main_rel = format!("{sys_dir}/{main_name}");
            let image = place_art(&mut w, r, sys, &main_rel)?;
            gamelists
                .entry(sys_dir.clone())
                .or_default()
                .push(GamelistEntry {
                    path: format!("./{main_name}"),
                    name: r.title.clone(),
                    image: image.map(|i| rel_from(&sys_dir, &i)),
                });
        }
    }

    if opts.bios && !w.report.cancelled {
        let exported: Vec<String> = sel
            .roms
            .iter()
            .map(|r| r.system.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        w.report.bios = bios::install(
            &lib.root.join("bios"),
            dest,
            profile,
            &exported,
            opts.dry_run,
        );
    }

    let manifest_path = dest.join(MANIFEST_DIR).join(format!("{}.json", col.name));
    let old_manifest = dest
        .join(OLD_MANIFEST_DIR)
        .join(format!("{}.json", col.name));
    let previous: BTreeSet<String> = fs::read(&manifest_path)
        .or_else(|_| fs::read(&old_manifest))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| serde_json::from_value(v["files"].clone()).ok())
        .unwrap_or_default();

    if opts.sync && !w.report.cancelled && !previous.is_empty() {
        let stale: Vec<String> = previous.difference(&w.written).cloned().collect();
        let mut by_dir: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for relp in &stale {
            let target = dest.join(relp);
            if target.exists() {
                w.report.removed += 1;
                w.report.planned.push(format!("remove {relp}"));
                if !opts.dry_run {
                    let _ = fs::remove_file(&target);
                }
            }
            if let Some((dir, name)) = relp.rsplit_once('/') {
                by_dir
                    .entry(dir.to_string())
                    .or_default()
                    .push(format!("./{name}"));
            }
        }
        if profile.gamelist && !opts.dry_run {
            for (dir, paths) in by_dir {
                profiles::remove_from_gamelist(&dest.join(&dir).join("gamelist.xml"), &paths)?;
            }
        }
    }

    if profile.gamelist && !opts.dry_run {
        for (dir, entries) in &gamelists {
            profiles::write_gamelist(&dest.join(dir).join("gamelist.xml"), entries)?;
        }
    }

    if !opts.dry_run {
        let files: BTreeSet<String> = if opts.sync && !w.report.cancelled {
            w.written.clone()
        } else {
            w.written.union(&previous).cloned().collect()
        };
        fs::create_dir_all(manifest_path.parent().unwrap())?;
        let manifest =
            serde_json::json!({"collection": col.name, "profile": col.profile, "files": files});
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )?;
    }
    if !sel.over_budget.is_empty() {
        warnings.insert(format!(
            "{} ROM(s) left out to fit the size limit",
            sel.over_budget.len()
        ));
    }
    w.report.warnings = warnings.into_iter().collect();
    rep.progress(total, total, "Done");
    Ok(w.report)
}
