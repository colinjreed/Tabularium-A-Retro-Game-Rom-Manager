//! Download artwork for library ROMs into the library's media cache.

use crate::library::Library;
use crate::progress::Reporter;
use crate::scrapers::{ScrapeError, Scraper};
use crate::{systems, Result};

#[derive(Debug, Clone, Default)]
pub struct ScrapeReport {
    pub found: usize,
    pub missing: usize,
    pub skipped: usize,
    pub missing_list: Vec<String>,
    pub stopped: Option<String>,
    pub cancelled: bool,
}

fn ext_for(data: &[u8]) -> &'static str {
    if data.starts_with(b"\xff\xd8\xff") {
        ".jpg"
    } else if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        ".webp"
    } else {
        ".png"
    }
}

pub fn scrape_library(
    lib: &Library,
    scrapers: &mut [Box<dyn Scraper>],
    kinds: &[String],
    systems_filter: &[String],
    refresh: bool,
    rep: &dyn Reporter,
) -> Result<ScrapeReport> {
    let mut out = ScrapeReport::default();
    let roms = lib.roms(systems_filter, "active")?;
    let total = roms.len();
    for (i, rom) in roms.iter().enumerate() {
        if rep.cancelled() {
            out.cancelled = true;
            break;
        }
        rep.progress(i, total, &rom.stem);
        let Some(system) = systems::get(&rom.system) else {
            continue;
        };
        for kind in kinds {
            if !refresh && lib.media_attempted(rom.id, kind)? {
                out.skipped += 1;
                continue;
            }
            let mut got: Option<(Vec<u8>, &'static str)> = None;
            let mut transient = false;
            for sc in scrapers.iter_mut() {
                if !sc.kinds().contains(&kind.as_str()) {
                    continue;
                }
                match sc.fetch(rom, system, kind) {
                    Ok(Some(d)) => {
                        got = Some((d, sc.name()));
                        break;
                    }
                    Ok(None) => {}
                    Err(ScrapeError::Fatal(msg)) => {
                        rep.log(format!("Stopping: {msg}"));
                        out.stopped = Some(msg);
                        return Ok(out);
                    }
                    Err(ScrapeError::Skip(msg)) => {
                        transient = true;
                        rep.log(format!("{} error on {}: {msg}", sc.name(), rom.stem));
                    }
                }
            }
            match got {
                Some((data, source)) => {
                    let dest = lib.media_dir(&rom.system, kind).join(format!(
                        "{}{}",
                        rom.stem,
                        ext_for(&data)
                    ));
                    std::fs::create_dir_all(dest.parent().unwrap())?;
                    std::fs::write(&dest, &data)?;
                    lib.set_media(rom.id, kind, Some(&dest), Some(source))?;
                    out.found += 1;
                }
                None => {
                    // Network trouble: don't mark as tried, so a later run retries it.
                    if !transient {
                        lib.set_media(rom.id, kind, None, None)?;
                    }
                    out.missing += 1;
                    out.missing_list
                        .push(format!("{} / {} [{kind}]", rom.system, rom.stem));
                }
            }
        }
    }
    rep.progress(total, total, "Done");
    Ok(out)
}
