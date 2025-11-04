# Rust MP3 Lyric Extractor Implementation Plan

## Phase 0 - Groundwork
- Confirm `Cargo.toml` dependencies (`clap`, `walkdir`, `id3`, `anyhow`, `log`, `env_logger`, `assert_cmd`, `tempfile`) and add them with appropriate feature flags.
- Introduce top-level `modules` in `src/` (`cli.rs`, `config.rs`, `scanner.rs`, `metadata.rs`, `writer.rs`, `report.rs`) with minimal scaffolding and unit-test stubs to keep the compiler happy.
- Decide on crate-level logging defaults and configure `env_logger` initialization in `main.rs`.
- Acceptance: `cargo check` succeeds with the new module stubs and dependencies.

## Phase 1 - CLI and Configuration Layer
- Implement `cli.rs` using `clap::Parser`, exposing flags described in the design.
- Add `config.rs` to normalize the root path, resolve the output file (absolute when provided), and carry the dry-run and quiet options.
- Unit-test configuration normalization (e.g., relative vs. absolute paths) using temporary directories and `std::env::set_current_dir`.
- Acceptance: `cargo test config` covers edge cases; running `cargo run -- --help` shows the expected CLI synopsis.

## Phase 2 - Directory Scanning
- Implement `scanner.rs` with a public iterator/wrapper around `WalkDir` that yields `PathBuf` entries for `.mp3` files.
- Filter on file extension case-insensitively and collect traversal errors for logging and metrics.
- Include unit tests using a temp directory tree with nested folders and dummy files to verify filtering and recursion behavior.
- Acceptance: `cargo test scanner` passes; running `cargo run -- --dry-run --root <tmp>` emits logs confirming visited MP3s.

## Phase 3 - Metadata Extraction Helpers
- Implement artist normalization and matching helpers in `metadata.rs`, including fallback lookup for `TPE2`.
- Extract title with sensible fallback and aggregate lyrics from `UnsynchronizedLyricsFrame`.
- Unit-test helpers with synthetic `id3::Tag` instances created in memory (use `Tag::new` and `add_frame`).
- Acceptance: `cargo test metadata` verifies artist matching, lyric aggregation, and title fallbacks.

## Phase 4 - Output Writer and Formatting
- Build `writer.rs` that encapsulates buffered appending, supports dry-run no-op behavior, and handles periodic flushing.
- Define record formatting in a dedicated function to centralize the block layout.
- Add unit tests that write to `tempfile::NamedTempFile`, asserting append semantics and dry-run behavior.
- Acceptance: `cargo test writer` confirms output formatting and append logic.

## Phase 5 - Orchestration in `main.rs`
- Wire together CLI parsing, configuration, scanner iteration, metadata filtering, and writer output.
- Add `report.rs` to track counts (files visited, matches written, errors) and emit a final summary respecting `--quiet`.
- Ensure all error paths use `anyhow::Context` so logs include file paths.
- Acceptance: `cargo run -- --dry-run` over a prepared fixture tree prints an accurate summary; `cargo clippy` produces no warnings.

## Phase 6 - Integration Tests and Polish
- Build integration tests with `assert_cmd` and temp directories containing tiny MP3 placeholders with ID3 tags written via `id3::Tag::write_to_path`.
- Cover scenarios: matching artist with lyrics, artist mismatch, missing lyrics, dry-run output, and repeated append runs.
- Update project documentation (`README.md`, `DESIGN.md`) with build/test instructions and behavior overview; ensure `IMPLEMENTATION_PLAN.md` reflects any scope adjustments.
- Acceptance: `cargo test` passes end-to-end; manual spot-check by opening the generated `lyrics.txt` confirms formatting.
