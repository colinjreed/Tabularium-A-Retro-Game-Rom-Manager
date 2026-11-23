use crate::app::{Page, RomApp};
use crate::jobs::{Job, JobOutput};
use crate::widgets::{self, card, page_title, section, systems_picker};
use egui::{RichText, Ui};
use tabularium_core::http::default_fetcher;
use tabularium_core::library::Library;
use tabularium_core::scrape::{scrape_library, ScrapeReport};
use tabularium_core::scrapers::libretro::LibretroScraper;
use tabularium_core::scrapers::screenscraper::ScreenScraper;
use tabularium_core::scrapers::{art_kind_label, Scraper, ART_KINDS};

pub struct ArtworkPage {
    pub libretro: bool,
    pub screenscraper: bool,
    pub screenscraper_first: bool,
    pub kinds: Vec<String>,
    pub systems: Vec<String>,
    pub refresh: bool,
    pub result: Option<ScrapeReport>,
}

impl Default for ArtworkPage {
    fn default() -> Self {
        ArtworkPage {
            libretro: true,
            screenscraper: false,
            screenscraper_first: true,
            kinds: vec!["boxart".into()],
            systems: vec![],
            refresh: false,
            result: None,
        }
    }
}

impl RomApp {
    pub fn artwork_page(&mut self, ui: &mut Ui) {
        page_title(
            ui,
            "Artwork",
            "Download box art and screenshots once into the library; every SD card you build reuses them.",
        );
        let total = self.data.roms.len();
        let have = self.data.with_art.len();
        ui.label(format!("{have} of {total} ROMs have box art."));
        ui.add_space(6.0);

        card(ui, |ui| {
            section(ui, "Sources");
            ui.checkbox(&mut self.artwork.libretro, "libretro thumbnails")
                .on_hover_text(
                    "Free, no account. Finds art by filename: best for No-Intro/Redump-named ROMs.",
                );
            ui.label(RichText::new("    Free, no account. Matches by file name, so works best with No-Intro / Redump names.").weak());
            let has_dev = self.settings.screenscraper.has_dev();
            ui.add_enabled_ui(has_dev, |ui| {
                ui.checkbox(&mut self.artwork.screenscraper, "ScreenScraper");
            });
            ui.label(RichText::new(
                "    Matches by file contents, so it finds badly named ROMs and arcade games, and has mix images \
                 and logos. Needs your account and developer credentials.",
            ).weak());
            if !has_dev {
                ui.horizontal(|ui| {
                    ui.label("    ");
                    if ui.link("Add ScreenScraper details in Settings").clicked() {
                        self.page = Page::Settings;
                    }
                });
            }
            if self.artwork.libretro && self.artwork.screenscraper {
                ui.checkbox(
                    &mut self.artwork.screenscraper_first,
                    "Try ScreenScraper first, then libretro",
                );
            }

            section(ui, "Image types");
            ui.horizontal_wrapped(|ui| {
                for k in ART_KINDS {
                    let mut on = self.artwork.kinds.iter().any(|x| x == k);
                    let only_ss = matches!(*k, "mix" | "wheel");
                    let label = if only_ss {
                        format!("{} (ScreenScraper)", art_kind_label(k))
                    } else {
                        art_kind_label(k).to_string()
                    };
                    if ui.checkbox(&mut on, label).changed() {
                        if on {
                            self.artwork.kinds.push(k.to_string())
                        } else {
                            self.artwork.kinds.retain(|x| x != k)
                        }
                    }
                }
            });

            section(ui, "Systems");
            systems_picker(
                ui,
                "art-systems",
                &mut self.artwork.systems,
                &self.data.stats,
            );
            ui.add_space(4.0);
            ui.checkbox(
                &mut self.artwork.refresh,
                "Retry ROMs that were already looked up",
            );

            ui.add_space(8.0);
            let ready = (self.artwork.libretro || self.artwork.screenscraper)
                && !self.artwork.kinds.is_empty()
                && !self.busy();
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new(RichText::new("Download artwork").strong()),
                )
                .clicked()
            {
                self.start_scrape(ui.ctx());
            }
        });

        ui.add_space(10.0);
        card(ui, |ui| {
            section(ui, "Genres");
            let known = self.data.roms.iter().filter(|r| r.genre.is_some()).count();
            ui.label(format!(
                "{known} of {} games have a genre. Genres come from libretro's lists for cartridge \
                 systems and are filled in automatically on import; use this for games imported earlier.",
                self.data.roms.len()
            ));
            if ui
                .add_enabled(!self.busy(), egui::Button::new("Look up genres"))
                .clicked()
            {
                let Some(root) = self.library_root() else {
                    return;
                };
                self.job =
                    Some(Job::spawn(
                        "Looking up genres",
                        ui.ctx(),
                        move |rep| match Library::open(&root).and_then(|lib| {
                            tabularium_core::metadata::enrich_genres(&lib, &default_fetcher(), rep)
                        }) {
                            Ok(n) => JobOutput::Genres(n),
                            Err(e) => JobOutput::Error(e.to_string()),
                        },
                    ));
            }
        });

        if let Some(r) = &self.artwork.result {
            ui.add_space(10.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} image(s) downloaded · {} not found · {} already done",
                        r.found, r.missing, r.skipped
                    ))
                    .strong()
                    .size(15.0),
                );
                if let Some(s) = &r.stopped {
                    ui.colored_label(widgets::err_color(ui), format!("Stopped: {s}"));
                }
                if r.cancelled {
                    ui.colored_label(
                        widgets::warn_color(ui),
                        "Cancelled. Run again to continue where you left off.",
                    );
                }
                if !r.missing_list.is_empty() {
                    egui::CollapsingHeader::new(format!("Not found ({})", r.missing_list.len())).show(ui, |ui| {
                        ui.label(RichText::new(
                            "Tip: libretro matching needs standard names. Try ScreenScraper for these, or lower \
                             the match strictness in Settings.",
                        ).weak());
                        for m in r.missing_list.iter().take(2000) {
                            ui.label(m);
                        }
                    });
                }
            });
        }
    }

    fn start_scrape(&mut self, ctx: &egui::Context) {
        let Some(root) = self.library_root() else {
            return;
        };
        let page = &self.artwork;
        let (use_lr, use_ss, ss_first) =
            (page.libretro, page.screenscraper, page.screenscraper_first);
        let kinds = page.kinds.clone();
        let systems = page.systems.clone();
        let refresh = page.refresh;
        let creds = self.settings.screenscraper.clone();
        let cutoff = self.settings.match_cutoff;
        self.artwork.result = None;
        self.job = Some(Job::spawn("Downloading artwork", ctx, move |rep| {
            let lib = match Library::open(&root) {
                Ok(l) => l,
                Err(e) => return JobOutput::Error(e.to_string()),
            };
            let mut scrapers: Vec<Box<dyn Scraper>> = vec![];
            if use_ss {
                match ScreenScraper::new(creds, default_fetcher()) {
                    Ok(s) => scrapers.push(Box::new(s)),
                    Err(e) => return JobOutput::Error(e),
                }
            }
            if use_lr {
                let lr = LibretroScraper::new(root.join("media").join(".cache"), default_fetcher())
                    .with_cutoff(cutoff);
                if use_ss && !ss_first {
                    scrapers.insert(0, Box::new(lr));
                } else {
                    scrapers.push(Box::new(lr));
                }
            }
            // Arcade art is matched by full name, so make sure names are known.
            if let Err(e) = tabularium_core::metadata::enrich(&lib, &default_fetcher(), rep) {
                rep.log(e.to_string());
            }
            match scrape_library(&lib, &mut scrapers, &kinds, &systems, refresh, rep) {
                Ok(r) => JobOutput::Scrape(r),
                Err(e) => JobOutput::Error(e.to_string()),
            }
        }));
    }
}
