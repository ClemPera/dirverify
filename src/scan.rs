use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Map of path relative to the scanned root -> file size in bytes.
pub type Files = BTreeMap<PathBuf, u64>;

pub struct Scan {
    pub files: Files,
    /// Entries that are neither regular files nor symlinks to one.
    pub skipped: usize,
}

/// Walk `root` recursively, calling `on_file` with each relative path and size.
pub fn scan(root: &Path, mut on_file: impl FnMut(&Path, u64)) -> io::Result<Scan> {
    let root = root.canonicalize()?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!("not a directory: {}", root.display()),
        ));
    }

    let mut scan = Scan {
        files: Files::new(),
        skipped: 0,
    };
    walk(&root, &root, &mut scan, &mut on_file)?;
    Ok(scan)
}

fn walk(
    root: &Path,
    dir: &Path,
    scan: &mut Scan,
    on_file: &mut impl FnMut(&Path, u64),
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;

        if kind.is_dir() {
            walk(root, &path, scan, on_file)?;
        } else if kind.is_file() {
            record(scan, on_file, root, &path, entry.metadata()?.len());
        } else if kind.is_symlink() {
            // Follow file symlinks, ignore directory ones to avoid cycles.
            match fs::metadata(&path) {
                Ok(meta) if meta.is_file() => record(scan, on_file, root, &path, meta.len()),
                _ => scan.skipped += 1,
            }
        } else {
            scan.skipped += 1;
        }
    }
    Ok(())
}

fn record(
    scan: &mut Scan,
    on_file: &mut impl FnMut(&Path, u64),
    root: &Path,
    path: &Path,
    size: u64,
) {
    let rel = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    on_file(&rel, size);
    scan.files.insert(rel, size);
}
