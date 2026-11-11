use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

/// Per-file read buffer. Large enough that syscall overhead disappears on 100 GB files.
const CHUNK: usize = 4 << 20;

pub struct Compared {
    pub rel: PathBuf,
    pub result: io::Result<bool>,
}

/// Byte-exact comparison, stopping at the first differing chunk.
pub fn same_file(a: &Path, b: &Path) -> io::Result<bool> {
    let mut file_a = File::open(a)?;
    let mut file_b = File::open(b)?;
    let mut buf_a = vec![0; CHUNK];
    let mut buf_b = vec![0; CHUNK];
    same_stream(&mut file_a, &mut file_b, &mut buf_a, &mut buf_b)
}

fn same_stream(
    file_a: &mut File,
    file_b: &mut File,
    buf_a: &mut [u8],
    buf_b: &mut [u8],
) -> io::Result<bool> {
    loop {
        let len_a = fill(file_a, buf_a)?;
        let len_b = fill(file_b, buf_b)?;
        if len_a != len_b || buf_a[..len_a] != buf_b[..len_b] {
            return Ok(false);
        }
        if len_a == 0 {
            return Ok(true);
        }
    }
}

/// Fill `buf` unless EOF, tolerating short reads on filesystems that return them.
fn fill(file: &mut File, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(len) => filled += len,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(filled)
}

/// Compare relative paths from tree `a` against tree `b` using `jobs` threads.
/// Results are sorted by relative path regardless of completion order.
pub fn compare_all(rels: &[PathBuf], a: &Path, b: &Path, jobs: usize) -> Vec<Compared> {
    let next = AtomicUsize::new(0);
    let done = Mutex::new(Vec::with_capacity(rels.len()));
    let workers = jobs.max(1).min(rels.len().max(1));

    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let mut buf_a = vec![0; CHUNK];
                let mut buf_b = vec![0; CHUNK];
                let mut local = Vec::new();

                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(rel) = rels.get(index) else { break };
                    let result = open_pair(a, b, rel).and_then(|(mut file_a, mut file_b)| {
                        same_stream(&mut file_a, &mut file_b, &mut buf_a, &mut buf_b)
                    });
                    local.push(Compared {
                        rel: rel.clone(),
                        result,
                    });
                }

                done.lock().unwrap().extend(local);
            });
        }
    });

    let mut results = done.into_inner().unwrap();
    results.sort_by(|left, right| left.rel.cmp(&right.rel));
    results
}

fn open_pair(a: &Path, b: &Path, rel: &Path) -> io::Result<(File, File)> {
    Ok((File::open(a.join(rel))?, File::open(b.join(rel))?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn empty_files_match() {
        let dir = tempdir().unwrap();
        let a = write(dir.path(), "a", b"");
        let b = write(dir.path(), "b", b"");
        assert!(same_file(&a, &b).unwrap());
    }

    #[test]
    fn identical_files_match_across_chunk_boundaries() {
        let dir = tempdir().unwrap();
        let data: Vec<u8> = (0..CHUNK * 2 + 123).map(|i| (i % 251) as u8).collect();
        let a = write(dir.path(), "a", &data);
        let b = write(dir.path(), "b", &data);
        assert!(same_file(&a, &b).unwrap());
    }

    #[test]
    fn differing_byte_is_detected() {
        let dir = tempdir().unwrap();
        let mut data = vec![7u8; CHUNK + 1];
        let a = write(dir.path(), "a", &data);
        data[CHUNK] = 8;
        let b = write(dir.path(), "b", &data);
        assert!(!same_file(&a, &b).unwrap());
    }

    #[test]
    fn shorter_file_is_detected() {
        let dir = tempdir().unwrap();
        let a = write(dir.path(), "a", b"abcdef");
        let b = write(dir.path(), "b", b"abc");
        assert!(!same_file(&a, &b).unwrap());
    }

    #[test]
    fn missing_file_is_an_error() {
        let dir = tempdir().unwrap();
        let a = write(dir.path(), "a", b"x");
        assert!(same_file(&a, &dir.path().join("nope")).is_err());
    }

    #[test]
    fn compare_all_pairs_relative_paths_and_sorts() {
        let root = tempdir().unwrap();
        let a = root.path().join("a");
        let b = root.path().join("b");
        fs::create_dir_all(a.join("nested")).unwrap();
        fs::create_dir_all(b.join("nested")).unwrap();
        fs::write(a.join("nested/same"), b"same").unwrap();
        fs::write(b.join("nested/same"), b"same").unwrap();
        fs::write(a.join("diff"), b"one").unwrap();
        fs::write(b.join("diff"), b"two").unwrap();

        let rels = vec![
            PathBuf::from("nested/same"),
            PathBuf::from("diff"),
            PathBuf::from("nested/same"),
        ];
        let results = compare_all(&rels, &a, &b, 2);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].rel, PathBuf::from("diff"));
        assert!(!results[0].result.as_ref().unwrap());
        assert_eq!(results[1].rel, PathBuf::from("nested/same"));
        assert!(results[1].result.as_ref().unwrap());
    }
}
