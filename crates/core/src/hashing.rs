//! Content hashing for duplicate detection, ScreenScraper lookups and BIOS checks.

use md5::{Digest, Md5};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const CHUNK: usize = 1 << 20;

/// CRC32 (hex) and size of the *game data*.
///
/// For a zip holding a single file this is the inner file's CRC, read from
/// the zip directory without decompressing, so "Tetris.zip" and "Tetris.gb"
/// are recognised as the same game.
pub fn content_crc(path: &Path) -> io::Result<(String, u64)> {
    if path.is_dir() {
        return dir_crc(path);
    }
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        if let Ok(f) = File::open(path) {
            if let Ok(mut z) = zip::ZipArchive::new(f) {
                let files: Vec<usize> = (0..z.len())
                    .filter(|&i| z.by_index_raw(i).map(|f| !f.is_dir()).unwrap_or(false))
                    .collect();
                if files.len() == 1 {
                    if let Ok(f) = z.by_index_raw(files[0]) {
                        return Ok((format!("{:08x}", f.crc32()), f.size()));
                    }
                }
            }
        }
    }
    let mut f = File::open(path)?;
    let mut h = crc32fast::Hasher::new();
    let mut buf = vec![0u8; CHUNK];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        size += n as u64;
    }
    Ok((format!("{:08x}", h.finalize()), size))
}

pub fn md5_file(path: &Path) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut h = Md5::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

pub fn md5_bytes(data: &[u8]) -> String {
    format!("{:x}", Md5::digest(data))
}

/// CRC of a game folder: every file's relative path and contents, in order.
fn dir_crc(dir: &Path) -> io::Result<(String, u64)> {
    let mut files: Vec<_> = walkdir::WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    files.sort();
    let mut h = crc32fast::Hasher::new();
    let mut size = 0;
    let mut buf = vec![0u8; CHUNK];
    for p in files {
        let rel = p
            .strip_prefix(dir)
            .unwrap_or(&p)
            .to_string_lossy()
            .replace('\\', "/");
        h.update(rel.to_lowercase().as_bytes());
        let mut f = File::open(&p)?;
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            size += n as u64;
        }
    }
    Ok((format!("{:08x}", h.finalize()), size))
}
