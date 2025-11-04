use std::fs;
use std::path::{Path, PathBuf};

use id3::frame::Lyrics;
use id3::{Tag, TagLike, Version};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn writes_matching_tracks_to_output_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let matching = root.join("match.mp3");
    write_track(
        &matching,
        Some("Studio Heroes"),
        None,
        Some("Hit Single"),
        &["Verse one", "Verse two"],
    );

    let mismatched = root.join("ignore.mp3");
    write_track(
        &mismatched,
        Some("Composer"),
        None,
        Some("Ambient"),
        &["Nope"],
    );

    let missing_lyrics = root.join("no_lyrics.mp3");
    write_track(
        &missing_lyrics,
        Some("Audio Crew"),
        None,
        Some("Silent"),
        &[],
    );

    let output = root.join("lyrics.txt");

    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .assert()
        .success();

    let contents = fs::read_to_string(&output).expect("lyrics written");
    assert!(
        contents.contains("=== Hit Single ==="),
        "expected block for matched track: {contents}"
    );
    assert!(contents.contains("Artist: Studio Heroes"));
    assert!(contents.contains("Verse one\n\nVerse two"));
    assert!(
        !contents.contains("Ambient"),
        "non-matching artist should not appear"
    );
    assert!(
        !contents.contains("Silent"),
        "track with missing lyrics should be skipped"
    );
}

#[test]
fn dry_run_does_not_create_output_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let track = root.join("song.mp3");
    write_track(&track, Some("Audio Stars"), None, Some("Demo"), &["Lyrics"]);

    let output = root.join("lyrics.txt");
    if output.exists() {
        fs::remove_file(&output).unwrap();
    }

    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .arg("--dry-run")
        .assert()
        .success();

    assert!(
        !output.exists(),
        "dry-run should not create the output file"
    );
}

#[test]
fn respects_custom_artist_filter() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    let track = root.join("signal.mp3");
    write_track(
        &track,
        Some("Cosmic Choir"),
        None,
        Some("Signal"),
        &["Echoes in the void"],
    );

    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .arg("--artist-filter")
        .arg("choir")
        .assert()
        .success();

    let output = root.join("lyrics.txt");
    let contents = fs::read_to_string(&output).expect("lyrics written");
    assert!(contents.contains("Cosmic Choir"));
    assert!(contents.contains("Echoes in the void"));
}

#[test]
fn emits_json_when_requested() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_track(
        &root.join("one.mp3"),
        Some("Audio Ensemble"),
        None,
        Some("Sunrise"),
        &["Golden light"],
    );
    write_track(
        &root.join("two.mp3"),
        Some("Audio Ensemble"),
        None,
        Some("Midday"),
        &["Bright sky"],
    );

    let output = root.join("lyrics.txt");

    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    let contents = fs::read_to_string(&output).expect("lyrics written");
    let mut lines = contents.lines();
    let first: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    let second: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert!(lines.next().is_none(), "expected exactly two JSON objects");

    assert_eq!(first["title"], "Sunrise");
    assert_eq!(first["artist"], "Audio Ensemble");
    assert_eq!(first["lyrics"], "Golden light");

    assert_eq!(second["title"], "Midday");
    assert_eq!(second["lyrics"], "Bright sky");
}

#[test]
fn scans_additional_extensions() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_track(
        &root.join("song.flac"),
        Some("Studio Heroes"),
        None,
        Some("FLAC Song"),
        &["Alternate format"],
    );

    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .arg("--extensions")
        .arg("mp3,flac")
        .assert()
        .success();

    let output = root.join("lyrics.txt");
    let contents = fs::read_to_string(&output).expect("lyrics written");
    assert!(
        contents.contains("FLAC Song"),
        "Expected to capture track with custom extension"
    );
}

#[test]
fn honors_max_depth_limit() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let sub = root.join("sub");
    let deeper = sub.join("deep");
    std::fs::create_dir_all(&deeper).unwrap();

    write_track(
        &root.join("surface.mp3"),
        Some("Audio Layer"),
        None,
        Some("Surface"),
        &["Top level"],
    );
    write_track(
        &deeper.join("buried.mp3"),
        Some("Audio Layer"),
        None,
        Some("Buried"),
        &["Hidden"],
    );

    let output = root.join("lyrics.txt");
    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .arg("--max-depth")
        .arg("2")
        .assert()
        .success()
        .stderr(
            contains("Max depth 2 prevented descending into 1 directories")
                .and(contains("Skipped due to depth limit")),
        );

    let contents = fs::read_to_string(&output).expect("lyrics written");
    assert!(contents.contains("Surface"));
    assert!(!contents.contains("Buried"));
}

#[cfg(unix)]
#[test]
fn follows_symlinks_when_requested() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let target_dir = TempDir::new().unwrap();
    let target = target_dir.path();
    std::fs::create_dir_all(target).unwrap();

    write_track(
        &target.join("linked.mp3"),
        Some("Audio Layer"),
        None,
        Some("Linked Song"),
        &["Inside symlinked dir"],
    );

    let link = root.join("link");
    symlink(target, &link).unwrap();

    let plain_output = root.join("plain.txt");
    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .arg("--output")
        .arg(&plain_output)
        .assert()
        .success();

    let plain_contents = std::fs::read_to_string(&plain_output).unwrap();
    assert!(
        !plain_contents.contains("Linked Song"),
        "should not traverse symlinks without flag"
    );

    let follow_output = root.join("follow.txt");
    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .arg("--output")
        .arg(&follow_output)
        .arg("--follow-symlinks")
        .assert()
        .success();

    let follow_contents = std::fs::read_to_string(&follow_output).unwrap();
    assert!(
        follow_contents.contains("Linked Song"),
        "expected to include file reached via symlink"
    );
}

#[test]
fn writes_summary_json_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    write_track(
        &root.join("song.mp3"),
        Some("Audio Ensemble"),
        None,
        Some("Summary Tune"),
        &["Lines"],
    );

    let summary_path = root.join("summary").join("run.json");
    assert_cmd::cargo::cargo_bin_cmd!("mdlyricgetter")
        .current_dir(root)
        .arg("--summary-json")
        .arg("summary/run.json")
        .arg("--output")
        .arg("out.txt")
        .assert()
        .success();

    let summary = std::fs::read_to_string(&summary_path).expect("summary written");
    let json: serde_json::Value = serde_json::from_str(&summary).expect("valid json");
    assert_eq!(json["matched"], 1);
    assert_eq!(json["scanned"], 1);
    assert_eq!(json["depth_skipped_dirs"], 0);
    assert!(json["depth_skip_paths"].as_array().unwrap().is_empty());
}

fn write_track(
    path: &Path,
    artist: Option<&str>,
    album_artist: Option<&str>,
    title: Option<&str>,
    lyrics: &[&str],
) -> PathBuf {
    let mut tag = Tag::new();
    if let Some(artist) = artist {
        tag.set_artist(artist);
    }
    if let Some(album_artist) = album_artist {
        tag.set_album_artist(album_artist);
    }
    if let Some(title) = title {
        tag.set_title(title);
    }
    for (index, line) in lyrics.iter().enumerate() {
        tag.add_frame(Lyrics {
            lang: "eng".to_string(),
            description: format!("segment{index}"),
            text: line.to_string(),
        });
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    // Seed file with placeholder audio bytes so the tag writer can update it in place.
    fs::write(path, [0_u8; 16]).unwrap();

    tag.write_to_path(path, Version::Id3v24).expect("write tag");
    path.to_path_buf()
}
