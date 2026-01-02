use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::{cli::OutputFormat, metadata::TrackMetadata};

pub struct OutputWriter {
    writer: Option<BufWriter<File>>,
    format: OutputFormat,
}

impl OutputWriter {
    pub fn create(path: &Path, format: OutputFormat, dry_run: bool) -> Result<Self> {
        if dry_run {
            return Ok(Self {
                writer: None,
                format,
            });
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("failed to open output file '{}'", path.display()))?;

        Ok(Self {
            writer: Some(BufWriter::new(file)),
            format,
        })
    }

    pub fn write_entry(&mut self, metadata: &TrackMetadata) -> Result<()> {
        if let Some(writer) = self.writer.as_mut() {
            match self.format {
                OutputFormat::Text => {
                    let block = format_block(metadata);
                    writer
                        .write_all(block.as_bytes())
                        .context("failed to append lyrics to output file")?;
                }
                OutputFormat::Json => {
                    let json = serde_json::to_string(metadata)
                        .context("failed to serialize track metadata as JSON")?;
                    writer
                        .write_all(json.as_bytes())
                        .context("failed to append JSON lyrics to output file")?;
                    writer
                        .write_all(b"\n")
                        .context("failed to append newline to JSON lyrics output")?;
                }
            }
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if let Some(writer) = self.writer.as_mut() {
            writer
                .flush()
                .context("failed to flush buffered lyrics to output file")?;
        }
        Ok(())
    }
}

pub fn format_block(metadata: &TrackMetadata) -> String {
    let normalized_lyrics = metadata.lyrics.trim_end_matches(['\n', '\r']).to_string();

    format!(
        "=== {title} ===\nArtist: {artist}\n{lyrics}\n\n",
        title = metadata.title,
        artist = metadata.artist,
        lyrics = normalized_lyrics
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use tempfile::NamedTempFile;

    fn sample_metadata() -> TrackMetadata {
        TrackMetadata {
            artist: "Studio Band".to_string(),
            title: "Echoes".to_string(),
            lyrics: "Line one\nLine two\n".to_string(),
        }
    }

    #[test]
    fn dry_run_does_not_create_file() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.into_temp_path();
        std::fs::remove_file(&path).unwrap();

        OutputWriter::create(&path, OutputFormat::Text, true).expect("create dry-run writer");
        assert!(!path.exists(), "dry-run should not touch the filesystem");
    }

    #[test]
    fn writes_blocks_and_appends() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        {
            let mut writer = OutputWriter::create(path, OutputFormat::Text, false).unwrap();
            writer.write_entry(&sample_metadata()).unwrap();
            writer.write_entry(&sample_metadata()).unwrap();
            writer.flush().unwrap();
        }

        let contents = fs::read_to_string(path).unwrap();
        let expected = format_block(&sample_metadata());
        assert_eq!(contents, format!("{expected}{expected}"));
    }

    #[test]
    fn formats_block_with_clean_trailing_newline() {
        let metadata = sample_metadata();
        let formatted = format_block(&metadata);
        assert!(formatted.ends_with("\n\n"));
        assert!(formatted.contains("=== Echoes ==="));
        assert!(formatted.contains("Artist: Studio Band"));
    }

    #[test]
    fn writes_json_lines_when_configured() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        {
            let mut writer = OutputWriter::create(path, OutputFormat::Json, false).unwrap();
            writer.write_entry(&sample_metadata()).unwrap();
            writer.write_entry(&sample_metadata()).unwrap();
            writer.flush().unwrap();
        }

        let contents = fs::read_to_string(path).unwrap();
        let mut lines = contents.lines();

        let first: TrackMetadata = serde_json::from_str(lines.next().unwrap()).unwrap();
        let second: TrackMetadata = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert!(lines.next().is_none());

        assert_eq!(first, sample_metadata());
        assert_eq!(second, sample_metadata());
    }
}
