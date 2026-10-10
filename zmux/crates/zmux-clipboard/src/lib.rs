use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Cursor, Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
pub struct Receipt {
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: usize,
    pub width: u32,
    pub height: u32,
}
pub fn stage_png(input: &Path, dir: &Path) -> Result<Receipt, String> {
    let mut source = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(input)
        .map_err(|e| e.to_string())?;
    let m = source.metadata().map_err(|e| e.to_string())?;
    if !m.is_file() || m.len() > 8 * 1024 * 1024 {
        return Err("PNG must be a regular file <= 8 MiB".into());
    }
    let mut b = Vec::new();
    std::io::Read::by_ref(&mut source)
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut b)
        .map_err(|e| e.to_string())?;
    if b.len() > 8 * 1024 * 1024 || !b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("not a bounded PNG".into());
    }
    let mut decoder = png::Decoder::new(Cursor::new(&b));
    decoder.set_limits(png::Limits {
        bytes: 64 * 1024 * 1024,
    });
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let (width, height) = (reader.info().width, reader.info().height);
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("PNG dimensions exceed 4096".into());
    }
    let n = reader
        .output_buffer_size()
        .ok_or("PNG allocation overflow")?;
    if n > 64 * 1024 * 1024 {
        return Err("PNG allocation limit".into());
    }
    reader
        .next_frame(&mut vec![0; n])
        .map_err(|e| e.to_string())?;
    let m = fs::symlink_metadata(dir).map_err(|e| e.to_string())?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err("staging directory must be owned and 0700".into());
    }
    let path = dir.join(format!("{}.png", uuid::Uuid::new_v4().simple()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(|e| e.to_string())?;
    if let Err(e) = file.write_all(&b).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&path);
        return Err(e.to_string());
    }
    Ok(Receipt {
        path,
        sha256: format!("{:x}", Sha256::digest(&b)),
        bytes: b.len(),
        width,
        height,
    })
}
