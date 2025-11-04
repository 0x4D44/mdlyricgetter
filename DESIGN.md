# Rust MP3 Lyric Extractor Design

## Goals and Scope
- Recursively traverse a directory tree (default: current working directory).
- Identify `.mp3` files whose artist tag contains the substring `udio` (case-insensitive).
- For each match, extract the song title and lyrics and append them to the output file (default: `lyrics.txt` in the working directory).
- Keep scanning even when individual files fail to parse, recording errors for the final report or logs.

## CLI Interface
- `mdlyricgetter [OPTIONS]`
- `--root <PATH>`: optional root directory to scan. Defaults to `.` when omitted.
- `--output <FILE>`: output path to append results to. Defaults to `lyrics.txt`.
- `--dry-run`: perform the scan and report matches without mutating the output file.
- `--max-depth <N>` (optional future flag): constrain recursion depth for very large trees.
- `--quiet`: suppress non-error logs; useful when composing the tool in scripts.

Arguments map into a `Config` struct that resolves the root path, normalizes the output path (absolute when provided, otherwise relative to the selected root), and stores the dry-run flag.

## Dependencies
- `clap`: declarative CLI parsing with helpful usage text.
- `walkdir`: efficient, cross-platform recursive directory traversal with error handling hooks.
- `id3`: read ID3v2/v1 tags, access artist, title, and unsynchronized lyrics frames.
- `anyhow`: ergonomic error propagation with context annotations.
- `log` + `env_logger`: runtime configurable diagnostics.
- std library only for everything else (path handling, file I/O, string utilities).

## High-Level Flow
1. Initialize logging early with `env_logger::init`.
2. Parse CLI arguments into `Config`; exit with code 2 on invalid input.
3. Create an output writer when not in dry-run mode:
   - Use `OpenOptions::new().create(true).append(true)` to avoid clobbering existing data.
   - Wrap in `BufWriter<File>` to minimize syscalls.
4. Walk the directory tree with `WalkDir`:
   - Filter entries by file type (`file_type().is_file()`), and extension `mp3` (case-insensitive).
   - On traversal errors (permissions, symlink loops), log a warning and continue.
5. For each MP3 path:
   - Attempt `Tag::read_from_path`. On failure, log and continue.
   - Fetch artist string via `tag.artist()`; additionally inspect `TPE2` (band) if `TPE1` missing. Normalize to lowercase and test `contains("udio")`.
   - Skip if no artist match.
   - Extract title via `tag.title()` with fallback `"Unknown Title"`.
   - Aggregate lyrics frames via `tag.lyrics()` (vector of `Lyric { lang, text, ... }`): join texts separated by double newlines. If no lyrics present, skip writing and log a notice.
6. Format the record into a text block:

```
=== Title ===
Artist: <artist>
<lyrics text>

```

   - The separator line aids manual reading and future parsing.
7. If not dry-run, write the block plus a trailing newline to the output writer, and flush periodically (or rely on drop).
8. Maintain counters for scanned files, matches, skips, and errors to print a final summary when the program exits.

## Module Layout
- `main.rs`: entry point; wires logging, config parsing, and orchestrates the scan.
- `cli.rs`: exports `CliArgs` and `parse()` built with `clap`.
- `config.rs`: resolves paths and holds normalized options (`root: PathBuf`, `output: PathBuf`, `dry_run: bool`).
- `scanner.rs`: wraps `WalkDir` iteration and yields candidate MP3 paths while collecting traversal errors.
- `metadata.rs`: helper functions for artist normalization, title fallback, and lyric extraction from the `id3::Tag`.
- `writer.rs`: output abstraction that accepts formatted blocks and no-ops in dry-run mode. Internally owns `Option<BufWriter<File>>`.
- `report.rs` (lightweight): tracks statistics and prints the final summary.

This modular split keeps `main` focused on orchestration and eases unit testing.

## Error Handling and Logging
- Use `anyhow::Context` to annotate failures (e.g., include file path when reading tags or writing output).
- Non-fatal issues (read errors, missing artist, missing lyrics) emit `warn!` logs and increment counters; fatal issues (cannot open output file) abort with error code 1.
- Provide a `--quiet` flag that sets the default log level to `error`, otherwise default to `info` to show progress.
- Optionally add `trace` logs behind `RUST_LOG=mdlyricgetter=debug` for debugging.

## Testing Strategy
- Unit-test pure helpers:
  - Artist matching logic handles case-insensitivity and multi-value separators such as semicolons or slashes.
  - Lyrics aggregation handles multiple `Lyric` frames and trims trailing whitespace.
- Integration tests with temporary directories:
  - Use `id3::Tag::write_to_path` on temp files to synthesize MP3 metadata (writing a tiny zeroed file to host the tag) and verify CLI behavior with `assert_cmd`.
  - Cover dry-run output, actual file append, and skip cases (missing lyrics, artist).
- Smoke test that repeated runs append (not overwrite) output.
- Add doc tests for formatting helpers if applicable.

## Future Enhancements
- Allow configurable artist needle (expose a `--artist-filter` option).
- Support additional tag sources (Vorbis comments, FLAC) by abstracting metadata readers.
- Emit JSON or CSV output for downstream tooling.
- Parallelize traversal with a worker queue when dealing with very large libraries.
