//! Minimal blocking HTTP client used by the scrapers.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::sleep;
use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    Status(u16),
    Transport(String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Status(c) => write!(f, "HTTP {c}"),
            HttpError::Transport(e) => write!(f, "network error: {e}"),
        }
    }
}

/// GET a URL: Ok(None) on 404, Ok(Some(body)) on success.
pub type Fetcher = Arc<dyn Fn(&str) -> Result<Option<Vec<u8>>, HttpError> + Send + Sync>;

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .user_agent(concat!("tabularium/", env!("CARGO_PKG_VERSION")))
            .timeout_global(Some(Duration::from_secs(45)))
            .http_status_as_error(false)
            // Use the operating system's certificate store.
            .tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            .build()
            .into()
    })
}

fn host_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Hosts that failed at the network level this session; skipped afterwards
/// so one unreachable mirror doesn't slow down every request.
fn dead_hosts() -> &'static Mutex<HashSet<String>> {
    static DEAD: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    DEAD.get_or_init(|| Mutex::new(HashSet::new()))
}

pub fn get(url: &str) -> Result<Option<Vec<u8>>, HttpError> {
    let host = host_of(url);
    if dead_hosts().lock().unwrap().contains(&host) {
        return Err(HttpError::Transport(format!("{host} is unreachable")));
    }
    let mut last = HttpError::Transport("no attempt".into());
    let mut transport_failures = 0;
    for attempt in 0..3u32 {
        if transport_failures >= 2 {
            break;
        }
        if attempt > 0 {
            sleep(Duration::from_secs(1 << attempt));
        }
        match agent().get(url).call() {
            Ok(mut resp) => {
                let code = resp.status().as_u16();
                if code == 404 {
                    return Ok(None);
                }
                if (200..300).contains(&code) {
                    return resp
                        .body_mut()
                        .with_config()
                        .limit(64 * 1024 * 1024)
                        .read_to_vec()
                        .map(Some)
                        .map_err(|e| HttpError::Transport(e.to_string()));
                }
                last = HttpError::Status(code);
                if !(code == 429 || code >= 500) {
                    return Err(last);
                }
            }
            Err(e) => {
                transport_failures += 1;
                last = HttpError::Transport(e.to_string());
            }
        }
    }
    if transport_failures >= 2 {
        dead_hosts().lock().unwrap().insert(host);
    }
    Err(last)
}

pub fn default_fetcher() -> Fetcher {
    Arc::new(get)
}
