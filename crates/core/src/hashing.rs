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
