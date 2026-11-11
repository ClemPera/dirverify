use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn trees() -> (TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    (temp, a, b)
}

fn put(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn dirverify(a: &Path, b: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dirverify"))
        .arg(a)
        .arg(b)
        .args(extra)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn identical_trees_print_only_the_ok_line() {
    let (_temp, a, b) = trees();
    put(&a, "nested/file.bin", b"payload");
    put(&b, "nested/file.bin", b"payload");

    let output = dirverify(&a, &b, &[]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "otherwise okay\n");
}

#[test]
fn missing_files_name_the_side_that_lacks_them() {
    let (_temp, a, b) = trees();
    put(&a, "only-a.txt", b"abc");
    put(&b, "only-b.txt", b"abcd");

    let output = dirverify(&a, &b, &[]);
    let text = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        text.contains("only-a.txt: missing in B (A has 3 B)"),
        "{text}"
    );
    assert!(
        text.contains("only-b.txt: missing in A (B has 4 B)"),
        "{text}"
    );
    assert!(!text.contains("otherwise okay"));
}

#[test]
fn size_difference_names_the_larger_side() {
    let (_temp, a, b) = trees();
    put(&a, "movie.mkv", &[0u8; 100 << 20]);
    put(&b, "movie.mkv", &[0u8; 80 << 20]);

    let text = stdout(&dirverify(&a, &b, &[]));

    assert!(
        text.contains("movie.mkv: size differs, A is larger by 20.0 MiB (A 100.0 MiB, B 80.0 MiB)"),
        "{text}"
    );
}

#[test]
fn same_size_different_bytes_are_reported_as_content_differences() {
    let (_temp, a, b) = trees();
    put(&a, "same.bin", b"aaaa");
    put(&b, "same.bin", b"aaab");

    let output = dirverify(&a, &b, &[]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "same.bin: content differs\n");
}

#[test]
fn size_mode_skips_the_byte_pass() {
    let (_temp, a, b) = trees();
    put(&a, "same.bin", b"aaaa");
    put(&b, "same.bin", b"aaab");

    let output = dirverify(&a, &b, &["--mode", "size"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), "otherwise okay\n");
}

#[test]
fn content_mode_compares_uneven_files_too() {
    let (_temp, a, b) = trees();
    put(&a, "cut.mp4", b"0123456789");
    put(&b, "cut.mp4", b"01234");

    let output = dirverify(&a, &b, &["--mode", "content"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "cut.mp4: content differs\n");
}

#[test]
fn verbose_lists_matches_and_prints_a_summary() {
    let (_temp, a, b) = trees();
    put(&a, "same.bin", b"abcd");
    put(&b, "same.bin", b"abcd");

    let text = stdout(&dirverify(&a, &b, &["-v"]));

    assert!(text.contains("same.bin: identical"), "{text}");
    assert!(
        text.contains("A: 1 files, B: 1 files, 1 compared byte-for-byte"),
        "{text}"
    );
}

#[test]
fn unreadable_root_is_an_error() {
    let (_temp, a, _b) = trees();
    let output = dirverify(&a, Path::new("/nonexistent/dirverify"), &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("error:"));
}
