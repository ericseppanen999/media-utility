use crate::domain::*;
use std::path::{Path, PathBuf};

pub fn sanitize(title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"/\|?*"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(100)
        .collect();
    let clean = clean.trim().trim_end_matches(['.', ' ']);
    let stem = clean.split('.').next().unwrap_or("").to_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if clean.is_empty() {
        "Untitled".into()
    } else if reserved {
        format!("_{clean}")
    } else {
        clean.into()
    }
}

pub fn available(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.into();
    }
    let parent = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    for n in 2.. {
        let candidate = parent.join(format!("{stem} ({n}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

pub fn finalize(staged: &Path, desired: &Path) -> Result<PathBuf> {
    // Atomic, non-replacing move on Windows; a hard-link fallback on other platforms.
    for _ in 0..1000 {
        let target = available(desired);
        match move_without_replace(staged, &target) {
            Ok(()) => return Ok(target),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(AppError::new(
                    "FINALIZE",
                    format!(
                        "Cannot finalize on this filesystem: {e}. Choose a writable destination."
                    ),
                ))
            }
        }
    }
    Err(AppError::new(
        "COLLISION",
        "Could not allocate an unused filename",
    ))
}

pub fn fingerprint(path: &Path) -> Result<String> {
    let p = path.canonicalize()?;
    let m = p.metadata()?;
    if !m.is_file() {
        return Err(AppError::new("SOURCE", "Choose a media file"));
    }
    let modified = m
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(digest(
        format!("{}:{}:{modified}", p.display(), m.len()).as_bytes(),
    ))
}

#[cfg(windows)]
fn move_without_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn move_without_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::hard_link(from, to)?;
    std::fs::remove_file(from)
}

pub fn content_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(sanitize("CON.txt"), "_CON.txt");
        assert_eq!(sanitize("a/b: c?."), "a_b_ c_");
        assert_eq!(sanitize("  "), "Untitled");
        assert_eq!(sanitize("Björk"), "Björk")
    }

    #[test]
    fn never_replaces() {
        let d = tempfile::tempdir().unwrap();
        let dest = d.path().join("song.mp3");
        let stage = d.path().join("temp.mp3");
        std::fs::write(&dest, b"original").unwrap();
        std::fs::write(&stage, b"new").unwrap();
        let out = finalize(&stage, &dest).unwrap();
        assert_ne!(out, dest);
        assert_eq!(std::fs::read(dest).unwrap(), b"original");
    }
}
