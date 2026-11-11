# dirverify

[![CI](https://github.com/ClemPera/dirverify/actions/workflows/ci.yml/badge.svg)](https://github.com/ClemPera/dirverify/actions/workflows/ci.yml)

Verify that two directory trees are identical, reporting only the entries that differ.

## Install

Prebuilt binaries are attached to each tagged release. To build from source:

```sh
cargo build --release
cargo install --path .
```

## Usage

```
dirverify [OPTIONS] <A> <B>
```

`A` is the reference tree, `B` the tree being verified.

| Option | Description |
| --- | --- |
| `-m, --mode <size\|content\|both>` | Compare sizes only, contents only, or both (default) |
| `-j, --jobs <N>` | Worker threads for the content pass (default: available parallelism) |
| `--progress <auto\|always\|never>` | Progress on stderr (default: `auto`) |
| `-v, --verbose` | List matching files and print a summary |

## Output

Nothing is written to stdout when the trees match. Each difference is one line, sorted by
relative path:

```
archive/2023/a.mkv: missing from B, present in A (1.4 GiB)
archive/2023/b.mkv: missing from A, present in B (812.3 MiB)
archive/2024/c.mkv: size differs, A is larger by 204.0 MiB (A 1.4 GiB, B 1.2 GiB)
archive/2024/d.mkv: content differs
```

Exit status:

| Code | Meaning |
| --- | --- |
| `0` | No differences |
| `1` | Differences found, or a file could not be read |
| `2` | Invalid arguments or an inaccessible root |

Read errors are reported on stderr.

With `-v`, matching files are listed as well and a summary is printed:

```
A:        12,483 files, 1.4 TiB
B:        12,483 files, 1.4 TiB
compared: 12,400 files, 1.3 TiB
elapsed:  41.2s
result:   no differences
```

## Behavior

- Both trees are walked once and keyed by relative path. Sizes come from `lstat`, so the size
  pass never opens a file.
- Pairs of equal size are compared byte by byte in 4 MiB chunks on worker threads. The
  comparison stops at the first differing chunk.
- Files are compared directly instead of hashed: one sequential read per file, and an early
  exit on mismatch.
- `--mode content` compares every pair regardless of size and reports a size mismatch as a
  content difference.
- File symlinks are followed. Directory symlinks are not traversed.
- The total byte count is known before the content pass starts, which is what makes the
  percentage and ETA possible.

## Limitations

- Trees are not snapshotted. A file modified during a run can produce a result that matches
  neither the state before nor after the change.
- Comparison is byte equality only. It makes no statement about whether a file is valid for
  its format.
- CI covers Linux and macOS. Windows is not tested, and path separators in the output follow
  the host platform.

## Development

```sh
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```
