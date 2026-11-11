# dirverify

[![CI](https://github.com/ClemPera/dirverify/actions/workflows/ci.yml/badge.svg)](https://github.com/ClemPera/dirverify/actions/workflows/ci.yml)

Check that two directory trees really match. Prints only what differs.

Two passes:

1. **size** — walks both trees, compares relative paths and byte sizes. Instant, even for
   terabytes.
2. **bytes** — for every pair with equal size, reads both files in 4 MiB chunks and stops at
   the first differing chunk. Runs on all cores.

## Usage

```
dirverify <A> <B> [--mode size|content|both] [--jobs N] [--progress auto|always|never] [--verbose]
```

`A` is the reference tree, `B` the tree being verified. Both names appear in the output.

| flag | meaning |
| --- | --- |
| `-m, --mode size` | sizes only; same-size files are assumed identical |
| `-m, --mode content` | bytes only; uneven files are compared too (reported as content differences) |
| `-m, --mode both` | size pass, then byte pass for equal-size pairs (default) |
| `-j, --jobs N` | worker threads for the byte pass (default: available parallelism) |
| `--progress auto\|always\|never` | progress on stderr; `auto` is live only when stderr is a terminal |
| `-v, --verbose` | also list matching files, plus pass timings and totals |

## Progress

Progress goes to stderr, never to stdout, so the report stays pipeable. Three steps are
announced: `scanning A`, `scanning B`, `comparing`.

```
comparing:  42.4%  1.4 GiB / 3.3 GiB  1.5 GiB/s  eta 0:01  archive/rip.mkv
```

On a terminal the line is redrawn in place and cleared when the step ends. With
`--progress always` in a pipe it becomes one line per step plus a refresh every 30 s, which
is what you want in a log. `--progress never` silences it entirely.

## Output

Silent when everything matches, except for a final `otherwise okay`:

```
$ dirverify a b
otherwise okay
```

Otherwise one line per problem, sorted by relative path:

```
archive/2023/rip.mkv: missing in B (A has 1.4 GiB)
archive/2023/extra.mkv: missing in A (B has 812.3 MiB)
archive/2024/movie.mkv: size differs, A is larger by 204.0 MiB (A 1.4 GiB, B 1.2 GiB)
archive/2024/clip.mkv: content differs
```

Exit codes: `0` identical, `1` differences found, `2` failure (bad path, unreadable file).

Read errors go to stderr and also make the run exit `1`.

## Notes

- Symlinks to files are followed and compared by content; symlinks to directories are never
  descended into, so loops are impossible.
- Content that differs only in size is caught by the size pass, so the byte pass never needs to
  read files that are already known to differ.
- Full-tree hashing is not used: a direct byte compare reads each file once and can bail out
  early. Hashing only wins when you compare repeatedly against a stored manifest.
- `ffprobe` is still the right first check for truncated video specifically; `dirverify` cannot
  tell you a container is damaged, only that two files are not byte-identical.

## Build

```
cargo build --release
./target/release/dirverify --help
```

Or install it onto your `PATH`:

```
cargo install --path .
```
