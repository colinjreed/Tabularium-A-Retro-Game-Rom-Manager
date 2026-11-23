//! Long-running work (import, scraping, export…) runs on a background thread
//! and reports progress to the UI through shared state.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tabularium_core::bios::ImportOutcome;
use tabularium_core::collection::ExportReport;
use tabularium_core::import::ImportReport;
use tabularium_core::progress::Reporter;
use tabularium_core::scrape::ScrapeReport;

pub enum JobOutput {
    Import {
        report: ImportReport,
        dry_run: bool,
    },
    Scrape(ScrapeReport),
    Trashed(usize),
    Export {
        report: ExportReport,
        dest: PathBuf,
        dry_run: bool,
    },
    BiosImport(Vec<(PathBuf, ImportOutcome)>),
    History(tabularium_core::playhistory::PlayReport),
    Genres(usize),
    Rename {
        report: tabularium_core::rename::FixReport,
        applied: bool,
    },
    Error(String),
}

#[derive(Default)]
pub struct JobState {
    pub done: usize,
    pub total: usize,
    pub current: String,
    pub log: Vec<String>,
    pub output: Option<JobOutput>,
}

pub struct Job {
    pub title: String,
    pub state: Arc<Mutex<JobState>>,
    pub cancel: Arc<AtomicBool>,
}

struct JobReporter {
    state: Arc<Mutex<JobState>>,
    cancel: Arc<AtomicBool>,
    ctx: egui::Context,
}

impl Reporter for JobReporter {
    fn progress(&self, done: usize, total: usize, current: &str) {
        let mut s = self.state.lock().unwrap();
        s.done = done;
        s.total = total;
        s.current = current.to_string();
        drop(s);
        self.ctx.request_repaint();
    }

    fn log(&self, line: String) {
        let mut s = self.state.lock().unwrap();
        s.log.push(line);
        let n = s.log.len();
        if n > 5000 {
            s.log.drain(..n - 5000);
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

impl Job {
    pub fn spawn<F>(title: impl Into<String>, ctx: &egui::Context, work: F) -> Job
    where
        F: FnOnce(&dyn Reporter) -> JobOutput + Send + 'static,
    {
        let state = Arc::new(Mutex::new(JobState::default()));
        let cancel = Arc::new(AtomicBool::new(false));
        let rep = JobReporter {
            state: state.clone(),
            cancel: cancel.clone(),
            ctx: ctx.clone(),
        };
        std::thread::spawn(move || {
            let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&rep)))
                .unwrap_or_else(|_| JobOutput::Error("The operation crashed unexpectedly.".into()));
            rep.state.lock().unwrap().output = Some(out);
            rep.ctx.request_repaint();
        });
        Job {
            title: title.into(),
            state,
            cancel,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.state.lock().unwrap().output.is_some()
    }

    pub fn take_output(&self) -> Option<JobOutput> {
        self.state.lock().unwrap().output.take()
    }

    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn cancel_requested(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}
