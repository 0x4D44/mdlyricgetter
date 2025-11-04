use std::path::PathBuf;

use log::{info, warn};
use serde::Serialize;

#[derive(Debug, Default)]
pub struct Report {
    pub scanned: usize,
    pub matched: usize,
    pub skipped_artist: usize,
    pub missing_lyrics: usize,
    pub depth_skipped_dirs: usize,
    pub depth_skip_paths: Vec<PathBuf>,
    pub walk_errors: usize,
    pub tag_errors: usize,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub scanned: usize,
    pub matched: usize,
    pub skipped_artist: usize,
    pub missing_lyrics: usize,
    pub walk_errors: usize,
    pub tag_errors: usize,
    pub depth_skipped_dirs: usize,
    pub depth_skip_paths: Vec<PathBuf>,
}

impl Report {
    pub fn record_scan(&mut self) {
        self.scanned += 1;
    }

    pub fn record_match(&mut self) {
        self.matched += 1;
    }

    pub fn record_artist_skip(&mut self) {
        self.skipped_artist += 1;
    }

    pub fn record_missing_lyrics(&mut self) {
        self.missing_lyrics += 1;
    }

    pub fn record_walk_error(&mut self) {
        self.walk_errors += 1;
    }

    pub fn record_tag_error(&mut self) {
        self.tag_errors += 1;
    }

    pub fn record_depth_skips(&mut self, count: usize, paths: Vec<PathBuf>) {
        self.depth_skipped_dirs += count;
        self.depth_skip_paths.extend(paths);
    }

    pub fn summary(&self) -> Summary {
        Summary {
            scanned: self.scanned,
            matched: self.matched,
            skipped_artist: self.skipped_artist,
            missing_lyrics: self.missing_lyrics,
            walk_errors: self.walk_errors,
            tag_errors: self.tag_errors,
            depth_skipped_dirs: self.depth_skipped_dirs,
            depth_skip_paths: self.depth_skip_paths.clone(),
        }
    }

    pub fn emit_summary(&self) {
        info!(
            "Scanned {scanned} MP3 files -- matched {matched}, artist skips {skipped}, missing lyrics {missing}, directories at depth limit {depth_skipped}",
            scanned = self.scanned,
            matched = self.matched,
            skipped = self.skipped_artist,
            missing = self.missing_lyrics,
            depth_skipped = self.depth_skipped_dirs,
        );

        if !self.depth_skip_paths.is_empty() {
            for path in &self.depth_skip_paths {
                info!(
                    "Depth limit prevented descent into directory '{}'",
                    path.display()
                );
            }
        }

        if self.walk_errors > 0 || self.tag_errors > 0 {
            warn!(
                "Encountered {walk_errors} traversal errors and {tag_errors} tag read failures.",
                walk_errors = self.walk_errors,
                tag_errors = self.tag_errors
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_reflects_collected_counts() {
        let mut report = Report::default();
        report.record_scan();
        report.record_scan();
        report.record_match();
        report.record_artist_skip();
        report.record_missing_lyrics();
        report.record_walk_error();
        report.record_tag_error();
        report.record_depth_skips(1, vec![PathBuf::from("deep")]);

        let summary = report.summary();

        assert_eq!(summary.scanned, 2);
        assert_eq!(summary.matched, 1);
        assert_eq!(summary.skipped_artist, 1);
        assert_eq!(summary.missing_lyrics, 1);
        assert_eq!(summary.walk_errors, 1);
        assert_eq!(summary.tag_errors, 1);
        assert_eq!(summary.depth_skipped_dirs, 1);
        assert_eq!(summary.depth_skip_paths, vec![PathBuf::from("deep")]);
    }
}
