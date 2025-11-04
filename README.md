# mdlyricgetter

`mdlyricgetter` is a small Rust CLI that walks a directory tree, finds MP3 files whose artist metadata contains the substring `udio`, and appends their titles and lyrics to an output file.

## Usage

```
mdlyricgetter [OPTIONS]
```

- `--root <PATH>`: root directory to scan (defaults to the current directory).
- `--output <FILE>`: file to append matched lyrics to (defaults to `lyrics.txt` within the root).
- `--dry-run`: scan and report without creating or appending to the output file.
- `--artist-filter <TEXT>`: case-insensitive substring that must appear in the artist name (defaults to `udio`).
- `--extensions <LIST>`: comma-separated list of audio file extensions to inspect (defaults to `mp3`).
- `--format <text|json>`: choose between the human-readable text blocks and newline-delimited JSON records (defaults to `text`).
- `--max-depth <N>`: limit recursion depth when traversing directories (root is depth 0).
- `--follow-symlinks`: traverse directory symlinks in addition to regular folders.
- `--summary-json <FILE>`: write a JSON run summary (counts, skips, errors) to the given file.
- `--quiet`: only emit error logs.

Example:

```
mdlyricgetter --root C:\Music --output collected.txt
```

## Development

```
cargo check
cargo test
```

The project includes unit tests for each module and integration tests that exercise the binary end-to-end. Structuring changes around these tests helps validate ID3 handling and output formatting before trying the tool on a real music library.
