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

pub fn scan(root: &Path) -> io::Result<Scan> {
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
    walk(&root, &root, &mut scan)?;
    Ok(scan)
}

fn walk(root: &Path, dir: &Path, scan: &mut Scan) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;

        if kind.is_dir() {
            walk(root, &path, scan)?;
        } else if kind.is_file() {
            scan.files
                .insert(relative(root, &path), entry.metadata()?.len());
        } else if kind.is_symlink() {
            // Follow file symlinks, ignore directory ones to avoid cycles.
            match fs::metadata(&path) {
                Ok(meta) if meta.is_file() => {
                    scan.files.insert(relative(root, &path), meta.len());
                }
                _ => scan.skipped += 1,
            }
        } else {
            scan.skipped += 1;
        }
    }
    Ok(())
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
