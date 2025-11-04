use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cli::{CliArgs, OutputFormat};

#[derive(Debug, Clone)]
pub struct Config {
    pub root: PathBuf,
    pub output: PathBuf,
    pub dry_run: bool,
    pub artist_filter: String,
    pub extensions: Vec<String>,
    pub output_format: OutputFormat,
    pub max_depth: Option<usize>,
    pub follow_symlinks: bool,
    pub summary_json: Option<PathBuf>,
    pub quiet: bool,
}

impl Config {
    pub fn from_args(args: CliArgs) -> Result<Self> {
        let root = normalize_root(args.root)?;
        let output = normalize_output(&root, args.output)?;
        let summary_json = args.summary_json.map(|path| make_absolute(&root, path));
        let extensions = parse_extensions(args.extensions);

        Ok(Self {
            root,
            output,
            dry_run: args.dry_run,
            artist_filter: args.artist_filter,
            extensions,
            output_format: args.format,
            max_depth: args.max_depth,
            follow_symlinks: args.follow_symlinks,
            summary_json,
            quiet: args.quiet,
        })
    }
}

fn normalize_root(root: Option<PathBuf>) -> Result<PathBuf> {
    match root {
        Some(path) => {
            let resolved = absolutize(&path)?;
            ensure_directory(&resolved)?;
            Ok(resolved)
        }
        None => {
            let cwd =
                std::env::current_dir().context("could not resolve current working directory")?;
            ensure_directory(&cwd)?;
            Ok(cwd)
        }
    }
}

fn normalize_output(root: &Path, output: Option<PathBuf>) -> Result<PathBuf> {
    let output_path = match output {
        Some(path) if path.is_absolute() => path,
        Some(path) => root.join(path),
        None => root.join("lyrics.txt"),
    };

    Ok(output_path)
}

fn make_absolute(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}
fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = std::env::current_dir().context("could not resolve current working directory")?;
        Ok(cwd.join(path))
    }
}

fn ensure_directory(path: &Path) -> Result<()> {
    anyhow::ensure!(
        path.is_dir(),
        "The provided root path '{}' is not an existing directory.",
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use tempfile::TempDir;

    #[test]
    fn defaults_to_current_directory() {
        let cwd = std::env::current_dir().unwrap();
        let args = CliArgs {
            root: None,
            output: None,
            dry_run: false,
            artist_filter: "udio".into(),
            extensions: "mp3".into(),
            format: OutputFormat::Text,
            max_depth: None,
            follow_symlinks: false,
            summary_json: None,
            quiet: false,
        };

        let config = Config::from_args(args).expect("config");

        assert_eq!(config.root, cwd);
        assert_eq!(config.output, cwd.join("lyrics.txt"));
        assert!(!config.dry_run);
        assert_eq!(config.artist_filter, "udio");
        assert_eq!(config.extensions, vec!["mp3"]);
        assert_eq!(config.output_format, OutputFormat::Text);
        assert_eq!(config.max_depth, None);
        assert!(!config.follow_symlinks);
        assert_eq!(config.summary_json, None);
        assert!(!config.quiet);
    }

    #[test]
    fn relative_root_is_resolved_against_cwd() {
        let temp_dir = TempDir::new().unwrap();
        let nested = temp_dir.path().join("library");
        fs::create_dir(&nested).unwrap();

        let _guard = CwdGuard::set(temp_dir.path());

        let args = CliArgs {
            root: Some(PathBuf::from("library")),
            output: Some(PathBuf::from("custom.txt")),
            dry_run: true,
            artist_filter: "mix".into(),
            extensions: "mp3,flac".into(),
            format: OutputFormat::Json,
            max_depth: Some(2),
            follow_symlinks: true,
            summary_json: Some(PathBuf::from("summary.json")),
            quiet: true,
        };

        let config = Config::from_args(args).expect("config");

        assert_eq!(config.root, nested);
        assert_eq!(config.output, nested.join("custom.txt"));
        assert!(config.dry_run);
        assert_eq!(config.artist_filter, "mix");
        assert_eq!(config.extensions, vec!["mp3", "flac"]);
        assert_eq!(config.output_format, OutputFormat::Json);
        assert_eq!(config.max_depth, Some(2));
        assert!(config.follow_symlinks);
        assert_eq!(config.summary_json, Some(nested.join("summary.json")));
        assert!(config.quiet);
    }

    #[test]
    fn absolute_output_is_preserved() {
        let temp_dir = TempDir::new().unwrap();
        let nested = temp_dir.path().join("library");
        fs::create_dir(&nested).unwrap();

        let output_path = temp_dir.path().join("lyrics").join("stash.txt");
        fs::create_dir_all(output_path.parent().unwrap()).unwrap();

        let args = CliArgs {
            root: Some(nested.clone()),
            output: Some(output_path.clone()),
            dry_run: false,
            artist_filter: "udio".into(),
            extensions: "mp3".into(),
            format: OutputFormat::Text,
            max_depth: None,
            follow_symlinks: false,
            summary_json: None,
            quiet: false,
        };

        let config = Config::from_args(args).expect("config");

        assert_eq!(config.root, nested);
        assert_eq!(config.output, output_path);
    }

    #[test]
    fn missing_root_yields_error() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("missing");

        let args = CliArgs {
            root: Some(nonexistent.clone()),
            output: None,
            dry_run: false,
            artist_filter: "udio".into(),
            extensions: "mp3".into(),
            format: OutputFormat::Text,
            max_depth: None,
            follow_symlinks: false,
            summary_json: None,
            quiet: false,
        };

        let error = Config::from_args(args).unwrap_err();
        let message = format!("{error:#}");
        assert!(
            message.contains(nonexistent.to_string_lossy().as_ref()),
            "unexpected error message: {message}"
        );
    }

    struct CwdGuard {
        original: PathBuf,
    }

    impl CwdGuard {
        fn set(path: &Path) -> Self {
            let original = std::env::current_dir().unwrap();
            std::env::set_current_dir(path).unwrap();
            Self { original }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }
}
fn parse_extensions(raw: String) -> Vec<String> {
    let mut exts: Vec<String> = raw
        .split(',')
        .map(|ext| ext.trim())
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.trim_start_matches('.').to_ascii_lowercase())
        .collect();

    if exts.is_empty() {
        exts.push("mp3".to_string());
    }

    exts
}
