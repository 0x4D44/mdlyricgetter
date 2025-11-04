use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
}

/// Command-line options for mdlyricgetter.
#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Scan MP3 files and extract lyrics when the artist matches a filter."
)]
pub struct CliArgs {
    /// Root directory to scan; defaults to current working directory.
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// Output file to append lyrics to; defaults to lyrics.txt in the working directory.
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// When set, perform the scan without writing to the output file.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Case-insensitive substring to look for within the artist name.
    #[arg(long, default_value = crate::metadata::DEFAULT_ARTIST_FILTER)]
    pub artist_filter: String,

    /// Comma-separated list of file extensions to scan (case-insensitive).
    #[arg(long, default_value = "mp3")]
    pub extensions: String,

    /// Output formatting strategy for matched tracks.
    #[arg(long, value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Limit recursion depth when scanning (0 means root only).
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// Follow directory symlinks while scanning.
    #[arg(long, default_value_t = false)]
    pub follow_symlinks: bool,

    /// Write a JSON summary report to the specified file.
    #[arg(long)]
    pub summary_json: Option<PathBuf>,

    /// Reduce log verbosity to errors only.
    #[arg(long, default_value_t = false)]
    pub quiet: bool,
}

impl CliArgs {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
