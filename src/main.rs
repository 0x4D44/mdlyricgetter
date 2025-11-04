mod cli;
mod config;
mod metadata;
mod report;
mod scanner;
mod writer;

use anyhow::{Context, Result};
use env_logger::Builder;
use log::LevelFilter;
use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:?}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli_args = cli::CliArgs::parse();

    let config = config::Config::from_args(cli_args)?;
    init_logging(config.quiet);
    let mut writer =
        writer::OutputWriter::create(&config.output, config.output_format, config.dry_run)?;
    let scanner = scanner::Scanner::new(
        &config.root,
        config.max_depth,
        config.follow_symlinks,
        config.extensions.clone(),
    );
    let mut report = report::Report::default();
    let artist_filter = config.artist_filter.clone();

    for entry in scanner.walk() {
        match entry {
            Ok(path) => {
                report.record_scan();
                process_file(&path, &artist_filter, &mut writer, &mut report)?;
            }
            Err(error) => {
                report.record_walk_error();
                let path = error.path().map(|p| p.display().to_string());
                match path {
                    Some(path) => log::warn!("Traversal error on '{}': {error}", path),
                    None => log::warn!("Traversal error: {error}"),
                }
            }
        }
    }

    let depth_skipped = scanner.skipped_due_to_depth();
    if depth_skipped > 0 {
        let skipped_paths = scanner.depth_skipped_paths();
        report.record_depth_skips(depth_skipped, skipped_paths.clone());
        if let Some(limit) = config.max_depth {
            log::warn!("Max depth {limit} prevented descending into {depth_skipped} directories.");
            for path in skipped_paths {
                log::info!("Skipped due to depth limit: {}", path.display());
            }
        }
    }

    writer.flush()?;
    report.emit_summary();

    if let Some(summary_path) = &config.summary_json {
        write_summary(summary_path, &report)?;
    }

    Ok(())
}

fn init_logging(quiet: bool) {
    let default_level = if quiet { "error" } else { "info" };

    let mut builder =
        Builder::from_env(env_logger::Env::default().default_filter_or(default_level));
    if quiet {
        builder.filter_level(LevelFilter::Error);
    }
    let _ = builder.try_init();
}

fn process_file(
    path: &Path,
    artist_filter: &str,
    writer: &mut writer::OutputWriter,
    report: &mut report::Report,
) -> Result<()> {
    match id3::Tag::read_from_path(path) {
        Ok(tag) => handle_tag(path, tag, artist_filter, writer, report),
        Err(error) => {
            report.record_tag_error();
            log::warn!("Failed to read ID3 tags from '{}': {error}", path.display());
            Ok(())
        }
    }
}

fn handle_tag(
    path: &Path,
    tag: id3::Tag,
    artist_filter: &str,
    writer: &mut writer::OutputWriter,
    report: &mut report::Report,
) -> Result<()> {
    match metadata::extract_metadata(&tag, artist_filter) {
        Some(track) => {
            writer.write_entry(&track)?;
            report.record_match();
            log::info!(
                "Captured lyrics for '{title}' by {artist}",
                title = track.title,
                artist = track.artist
            );
        }
        None => {
            if let Some(artist) = metadata::match_artist(&tag, artist_filter) {
                report.record_missing_lyrics();
                let title = metadata::resolve_title(&tag);
                log::info!(
                    "Skipping '{title}' by {artist} in file '{file}' -- no lyrics frames found.",
                    title = title,
                    artist = artist,
                    file = path.display()
                );
            } else {
                report.record_artist_skip();
            }
        }
    }

    Ok(())
}

fn write_summary(path: &Path, report: &report::Report) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create directories for summary '{}'",
                parent.display()
            )
        })?;
    }
    let file = std::fs::File::create(path)
        .with_context(|| format!("failed to create summary file '{}'", path.display()))?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &report.summary())
        .with_context(|| format!("failed to write JSON summary to '{}'", path.display()))?;
    Ok(())
}
