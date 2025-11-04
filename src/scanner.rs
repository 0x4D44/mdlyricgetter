use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use walkdir::{DirEntry, IntoIter, WalkDir};

pub struct Scanner {
    root: PathBuf,
    max_depth: Option<usize>,
    follow_symlinks: bool,
    extensions: Arc<Vec<String>>,
    skipped_due_to_depth: Arc<AtomicUsize>,
    skipped_paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl Scanner {
    pub fn new(
        root: &Path,
        max_depth: Option<usize>,
        follow_symlinks: bool,
        extensions: Vec<String>,
    ) -> Self {
        Self {
            root: root.to_path_buf(),
            max_depth,
            follow_symlinks,
            extensions: Arc::new(extensions),
            skipped_due_to_depth: Arc::new(AtomicUsize::new(0)),
            skipped_paths: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn walk(&self) -> ScannerIter {
        let walkdir = WalkDir::new(&self.root).follow_links(self.follow_symlinks);

        ScannerIter {
            inner: walkdir.into_iter(),
            max_depth: self.max_depth,
            extensions: Arc::clone(&self.extensions),
            skipped_due_to_depth: Arc::clone(&self.skipped_due_to_depth),
            skipped_paths: Arc::clone(&self.skipped_paths),
        }
    }

    pub fn skipped_due_to_depth(&self) -> usize {
        self.skipped_due_to_depth.load(Ordering::Relaxed)
    }

    pub fn depth_skipped_paths(&self) -> Vec<PathBuf> {
        let guard = self
            .skipped_paths
            .lock()
            .expect("poisoned depth skip paths");
        guard.clone()
    }
}

pub struct ScannerIter {
    inner: IntoIter,
    max_depth: Option<usize>,
    extensions: Arc<Vec<String>>,
    skipped_due_to_depth: Arc<AtomicUsize>,
    skipped_paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl Iterator for ScannerIter {
    type Item = Result<PathBuf, walkdir::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(entry) = self.inner.next() {
            match entry {
                Ok(entry) => {
                    if let Some(limit) = self.max_depth {
                        if entry.depth() > limit {
                            continue;
                        }
                        if entry.depth() == limit && entry.file_type().is_dir() {
                            self.skipped_due_to_depth.fetch_add(1, Ordering::Relaxed);
                            if let Ok(mut paths) = self.skipped_paths.lock() {
                                paths.push(entry.path().to_path_buf());
                            }
                            self.inner.skip_current_dir();
                            continue;
                        }
                    }

                    if is_target(&entry, &self.extensions) {
                        return Some(Ok(entry.into_path()));
                    }
                }
                Err(error) => return Some(Err(error)),
            }
        }
        None
    }
}

fn is_target(entry: &DirEntry, extensions: &[String]) -> bool {
    entry.file_type().is_file() && has_allowed_extension(entry.path(), extensions)
}

fn has_allowed_extension(path: &Path, extensions: &[String]) -> bool {
    let ext = match path.extension().and_then(|ext| ext.to_str()) {
        Some(value) => value.to_ascii_lowercase(),
        None => return false,
    };

    extensions.iter().any(|allowed| allowed == &ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use tempfile::TempDir;

    #[test]
    fn collects_mp3_files_recursively() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("sub");
        fs::create_dir(&nested).unwrap();

        let song1 = temp.path().join("song1.mp3");
        let song2 = nested.join("song2.MP3");
        fs::write(&song1, b"fake").unwrap();
        fs::write(&song2, b"fake").unwrap();

        fs::write(temp.path().join("readme.txt"), b"ignore").unwrap();
        fs::create_dir(temp.path().join("not_audio.mp3")).unwrap();

        let scanner = Scanner::new(temp.path(), None, false, vec!["mp3".into()]);
        let mut collected: Vec<PathBuf> = scanner.walk().map(|res| res.expect("entry")).collect();
        collected.sort();

        assert_eq!(collected, vec![song1, song2]);
        assert_eq!(scanner.skipped_due_to_depth(), 0);
        assert!(scanner.depth_skipped_paths().is_empty());
    }

    #[test]
    fn respects_max_depth_and_tracks_skips() {
        let temp = TempDir::new().unwrap();
        let child = temp.path().join("sub");
        let grandchild = child.join("deep");
        fs::create_dir_all(&grandchild).unwrap();

        let shallow = temp.path().join("root.mp3");
        let mid = child.join("mid.mp3");
        let deep = grandchild.join("deep.mp3");
        fs::write(&shallow, b"fake").unwrap();
        fs::write(&mid, b"fake").unwrap();
        fs::write(&deep, b"fake").unwrap();

        let scanner = Scanner::new(temp.path(), Some(2), false, vec!["mp3".into()]);
        let mut collected: Vec<PathBuf> = scanner.walk().map(|res| res.expect("entry")).collect();
        collected.sort();

        assert_eq!(collected, vec![shallow, mid]);
        assert_eq!(scanner.skipped_due_to_depth(), 1);
        let skipped = scanner.depth_skipped_paths();
        assert_eq!(skipped, vec![grandchild]);
    }

    #[test]
    fn propagates_walkdir_errors() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let file = root.join("music.mp3");
        fs::write(&file, b"fake").unwrap();

        let scanner = Scanner::new(&root, None, false, vec!["mp3".into()]);
        drop(temp);

        let mut iter = scanner.walk();
        let first = iter.next().unwrap();
        assert!(first.is_err());
    }

    #[test]
    fn filters_multiple_extensions() {
        let temp = TempDir::new().unwrap();
        let mp3 = temp.path().join("song.mp3");
        let flac = temp.path().join("track.flac");
        let txt = temp.path().join("notes.txt");
        fs::write(&mp3, b"fake").unwrap();
        fs::write(&flac, b"fake").unwrap();
        fs::write(&txt, b"fake").unwrap();

        let scanner = Scanner::new(temp.path(), None, false, vec!["mp3".into(), "flac".into()]);
        let mut collected: Vec<PathBuf> = scanner.walk().map(|res| res.expect("entry")).collect();
        collected.sort();

        let mut expected = vec![flac, mp3];
        expected.sort();
        assert_eq!(collected, expected);
    }
}
