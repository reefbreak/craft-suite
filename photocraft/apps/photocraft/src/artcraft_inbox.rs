//! File › Export › Send to ArtCraft: delivers a finished PNG into ArtCraft's inbox folder,
//! `<home>/Artcraft/inbox`.
//!
//! ArtCraft polls that folder and imports every complete file in it, so a file must never be seen
//! half-written: the bytes go to a hidden `.part` file in the same folder (ArtCraft skips hidden
//! names and `.part`/`.tmp` files), are flushed, and only then appear under their final name in
//! one step. Files are named `PhotoCraft - <document>.png` so ArtCraft can say where they came
//! from, and an existing file is never replaced: the name gains ` 2`, ` 3`, … instead.

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Every file name starts with this, so ArtCraft can tell where the file came from.
pub const NAME_PREFIX: &str = "PhotoCraft - ";
/// How many numbered names (` 2` … ` 9999`) are tried before giving up.
const MAX_SUFFIX: u32 = 9999;
/// Longest document-name part kept, in characters (file systems allow 255 bytes per name).
const MAX_NAME_CHARS: usize = 120;

/// ArtCraft's inbox under the user's home folder (`HOME`, or `USERPROFILE` on Windows), read
/// from `env` so tests can fake it. `None` without a home folder.
pub fn inbox_dir(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let home = env("HOME").filter(|h| !h.is_empty()).or_else(|| if cfg!(windows) { env("USERPROFILE").filter(|h| !h.is_empty()) } else { None })?;
    Some(PathBuf::from(home).join("Artcraft").join("inbox"))
}

/// Write `png` into `dir` (created if missing) as `PhotoCraft - <document name>.png`, numbered when
/// that name is taken, and return the final path. The file appears atomically; on failure no
/// partial file is left behind.
pub fn send(dir: &Path, doc_name: &str, png: &[u8]) -> io::Result<PathBuf> {
    fs::create_dir_all(dir).map_err(|e| context(e, format!("cannot create {}", dir.display())))?;
    let base = base_name(doc_name);
    let part = write_part(dir, &base, png)?;
    let result = publish_unique(&part, dir, &base);
    // After a hard link (or on failure) the hidden part file is still there; after a rename it isn't.
    let _ = fs::remove_file(&part);
    result
}

/// `PhotoCraft - <document name>` without the document's extension, safe as a file name on every
/// platform.
pub fn base_name(doc_name: &str) -> String {
    let stem = match doc_name.rsplit_once('.') {
        Some((stem, ext)) if !stem.trim().is_empty() && !ext.is_empty() && !ext.contains(' ') => stem,
        _ => doc_name,
    };
    let clean: String = stem
        .chars()
        .map(|c| if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .take(MAX_NAME_CHARS)
        .collect();
    // Windows drops trailing dots and spaces from names.
    let clean = clean.trim().trim_end_matches('.').trim_end();
    format!("{NAME_PREFIX}{}", if clean.is_empty() { "Untitled" } else { clean })
}

/// The `n`th candidate name: `base.png`, then `base 2.png`, `base 3.png`, …
pub fn numbered(base: &str, n: u32) -> String {
    if n <= 1 { format!("{base}.png") } else { format!("{base} {n}.png") }
}

/// Write the bytes to a new hidden `.part` file in `dir` and flush them to disk.
fn write_part(dir: &Path, base: &str, png: &[u8]) -> io::Result<PathBuf> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let stem: String = base.chars().take(64).collect();
    for _ in 0..8 {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let part = dir.join(format!(".{stem}.{}-{n}.png.part", std::process::id()));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&part) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(context(e, format!("cannot write to {}", dir.display()))),
        };
        if let Err(e) = file.write_all(png).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = fs::remove_file(&part);
            return Err(context(e, format!("cannot write to {}", dir.display())));
        }
        return Ok(part);
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{}: could not create a temporary file", dir.display())))
}

/// Give `part` the first free numbered name in `dir`, never replacing an existing file.
fn publish_unique(part: &Path, dir: &Path, base: &str) -> io::Result<PathBuf> {
    for n in 1..=MAX_SUFFIX {
        let target = dir.join(numbered(base, n));
        match publish(part, &target) {
            Ok(()) => return Ok(target),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(context(e, format!("cannot create {}", target.display()))),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{}: every name for {base}.png is taken", dir.display())))
}

/// Make the finished `part` file appear as `target`, failing with `AlreadyExists` if `target`
/// exists. A hard link is atomic and never replaces a file; where the file system has no hard links
/// (FAT, some network shares) the part file is renamed instead, after checking the name is free.
fn publish(part: &Path, target: &Path) -> io::Result<()> {
    match fs::hard_link(part, target) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(e),
        Err(_) if fs::symlink_metadata(target).is_ok() => Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} exists", target.display()))),
        Err(_) => fs::rename(part, target),
    }
}

fn context(e: io::Error, msg: String) -> io::Error {
    io::Error::new(e.kind(), format!("{msg}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

    mod naming {
        use super::*;

        #[test]
        fn base_name_drops_the_extension_and_unsafe_characters() {
            assert_eq!(base_name("cat.psd"), "PhotoCraft - cat");
            assert_eq!(base_name("Untitled-1"), "PhotoCraft - Untitled-1");
            assert_eq!(base_name("v1.2 final.pcraft"), "PhotoCraft - v1.2 final");
            assert_eq!(base_name("a/b:c*?.png"), "PhotoCraft - a_b_c__");
            assert_eq!(base_name(".psd"), "PhotoCraft - .psd");
            assert_eq!(base_name(""), "PhotoCraft - Untitled");
            assert_eq!(base_name("  ...  "), "PhotoCraft - Untitled");
            assert_eq!(base_name("line\nbreak"), "PhotoCraft - line_break");
            let long = "é".repeat(400);
            assert_eq!(base_name(&long).chars().count(), NAME_PREFIX.chars().count() + MAX_NAME_CHARS);
        }

        #[test]
        fn numbered_names_add_a_space_and_number_before_the_extension() {
            assert_eq!(numbered("PhotoCraft - cat", 1), "PhotoCraft - cat.png");
            assert_eq!(numbered("PhotoCraft - cat", 2), "PhotoCraft - cat 2.png");
            assert_eq!(numbered("PhotoCraft - cat", 10), "PhotoCraft - cat 10.png");
        }

        #[test]
        fn inbox_is_artcraft_inbox_under_home() {
            let env = |k: &str| (k == "HOME").then(|| OsString::from("/home/ada"));
            assert_eq!(inbox_dir(env), Some(PathBuf::from("/home/ada").join("Artcraft").join("inbox")));
            assert_eq!(inbox_dir(|_| None), None);
            assert_eq!(inbox_dir(|_| Some(OsString::new())), None);
        }
    }

    mod writing {
        use super::*;

        #[test]
        fn send_writes_a_complete_png_and_leaves_no_partial_file() {
            let tmp = tempdir("send");
            // The inbox folder is created when missing.
            let dir = tmp.join("Artcraft").join("inbox");
            let png = sample_png();
            let path = send(&dir, "cat.psd", &png).unwrap();
            assert_eq!(path, dir.join("PhotoCraft - cat.png"));
            let written = fs::read(&path).unwrap();
            assert_eq!(written, png);
            assert!(written.starts_with(PNG_MAGIC));
            let img = photocraft_codecs::decode(&written).unwrap();
            assert_eq!(img.dimensions(), (3, 2));
            assert_eq!(names(&dir), ["PhotoCraft - cat.png"], "no hidden or .part file remains");
            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn send_never_overwrites_and_numbers_the_name() {
            let tmp = tempdir("dedup");
            let png = sample_png();
            fs::write(tmp.join("PhotoCraft - cat.png"), b"older").unwrap();
            fs::write(tmp.join("PhotoCraft - cat 2.png"), b"older 2").unwrap();
            let third = send(&tmp, "cat.psd", &png).unwrap();
            assert_eq!(third, tmp.join("PhotoCraft - cat 3.png"));
            let fourth = send(&tmp, "cat.tif", &png).unwrap();
            assert_eq!(fourth, tmp.join("PhotoCraft - cat 4.png"));
            assert_eq!(fs::read(tmp.join("PhotoCraft - cat.png")).unwrap(), b"older");
            assert_eq!(fs::read(tmp.join("PhotoCraft - cat 2.png")).unwrap(), b"older 2");
            assert_eq!(fs::read(&fourth).unwrap(), png);
            assert_eq!(names(&tmp), ["PhotoCraft - cat 2.png", "PhotoCraft - cat 3.png", "PhotoCraft - cat 4.png", "PhotoCraft - cat.png"]);
            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn a_failed_send_leaves_nothing_behind() {
            let tmp = tempdir("fail");
            // The inbox path is a file, so the folder can't be created.
            let blocked = tmp.join("inbox");
            fs::write(&blocked, b"not a folder").unwrap();
            assert!(send(&blocked, "cat.psd", &sample_png()).is_err());
            assert_eq!(names(&tmp), ["inbox"]);
            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn publish_refuses_an_existing_target() {
            let tmp = tempdir("publish");
            let (part, target) = (tmp.join(".x.part"), tmp.join("x.png"));
            fs::write(&part, b"new").unwrap();
            fs::write(&target, b"old").unwrap();
            assert_eq!(publish(&part, &target).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
            assert_eq!(fs::read(&target).unwrap(), b"old");
            let _ = fs::remove_dir_all(&tmp);
        }
    }

    /// A fresh, empty folder for one test.
    fn tempdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("photocraft-artcraft-inbox-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Sorted file names in `dir`, hidden ones included.
    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    }

    /// A 3×2 RGBA PNG with a transparent pixel.
    fn sample_png() -> Vec<u8> {
        let mut px = vec![255u8; 3 * 2 * 4];
        px[3] = 0;
        let img = photocraft_codecs::Image::from_u8(3, 2, photocraft_codecs::ChannelLayout::Rgba, px).unwrap();
        photocraft_codecs::encode(&img, photocraft_codecs::Format::Png, &photocraft_codecs::EncodeOptions::default()).unwrap()
    }
}
