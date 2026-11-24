//! Time an import: `cargo run --release --example import_bench -- SRC LIB [offline]`
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tabularium_core::import::{import, ImportOptions};
use tabularium_core::library::Library;
use tabularium_core::progress::Reporter;

struct Timer {
    start: Instant,
    last: Mutex<String>,
}
impl Reporter for Timer {
    fn progress(&self, done: usize, total: usize, current: &str) {
        let word = current.split_whitespace().next().unwrap_or("");
        let phase = match word {
            "Scanning" | "Identifying" | "Checking" | "Looking" | "Fetching" => word.to_string(),
            _ if total == 0 => current.to_string(),
            _ => "Adding".into(),
        };
        let mut last = self.last.lock().unwrap();
        if *last != phase {
            eprintln!(
                "{:7.2}s  {phase} ({done}/{total})",
                self.start.elapsed().as_secs_f64()
            );
            *last = phase;
        }
    }
    fn log(&self, _: String) {}
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let lib_dir = PathBuf::from(&a[2]);
    let _ = std::fs::remove_dir_all(&lib_dir);
    let lib = Library::create(&lib_dir).unwrap();
    let opts = ImportOptions {
        fetch: (a.get(3).map(|s| s.as_str()) != Some("offline"))
            .then(tabularium_core::http::default_fetcher),
        official_names: true,
        strip_numbers: true,
        ..Default::default()
    };
    let t = Timer {
        start: Instant::now(),
        last: Mutex::new(String::new()),
    };
    let rep = import(&lib, &[PathBuf::from(&a[1])], &opts, &t).unwrap();
    eprintln!(
        "{:7.2}s  finished: {} added, {} unknown, {} dupes",
        t.start.elapsed().as_secs_f64(),
        rep.added_total(),
        rep.unknown.len(),
        rep.duplicates.len()
    );
}
