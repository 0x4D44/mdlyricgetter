use id3::{
    frame::{Comment, Content, ExtendedText, Lyrics as LyricsFrame},
    Tag, TagLike,
};
use serde::{Deserialize, Serialize};

pub const DEFAULT_ARTIST_FILTER: &str = "udio";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub artist: String,
    pub title: String,
    pub lyrics: String,
}

pub fn extract_metadata(tag: &Tag, needle: &str) -> Option<TrackMetadata> {
    let artist = match_artist(tag, needle)?;
    let lyrics = collect_lyrics(tag)?;
    let title = resolve_title(tag);

    Some(TrackMetadata {
        artist,
        title,
        lyrics,
    })
}

pub(crate) fn match_artist(tag: &Tag, needle: &str) -> Option<String> {
    let artist = resolve_artist(tag)?;
    if !matches_artist(&artist, needle) {
        return None;
    }

    Some(artist)
}

pub(crate) fn resolve_artist(tag: &Tag) -> Option<String> {
    tag.artist()
        .or_else(|| tag.album_artist())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_owned())
}

fn matches_artist(artist: &str, needle: &str) -> bool {
    let normalized_artist = artist.to_ascii_lowercase();
    let normalized_needle = needle.trim().to_ascii_lowercase();

    normalized_needle.is_empty() || normalized_artist.contains(&normalized_needle)
}

pub(crate) fn resolve_title(tag: &Tag) -> String {
    tag.title()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(|title| title.to_owned())
        .unwrap_or_else(|| "Unknown Title".to_string())
}

pub(crate) fn collect_lyrics(tag: &Tag) -> Option<String> {
    let mut blocks = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for lyric in tag.lyrics() {
        push_block(&mut blocks, &mut seen, lyric.text.as_str());
    }

    for frame in tag.frames() {
        match frame.content() {
            Content::ExtendedText(ExtendedText { description, value })
                if description.eq_ignore_ascii_case("lyrics") =>
            {
                push_block(&mut blocks, &mut seen, value);
            }
            Content::Comment(Comment {
                description, text, ..
            }) if description.eq_ignore_ascii_case("lyrics") => {
                push_block(&mut blocks, &mut seen, text);
            }
            Content::Lyrics(LyricsFrame { text, .. }) => {
                push_block(&mut blocks, &mut seen, text);
            }
            Content::Text(value) if frame.id().eq_ignore_ascii_case("lyrics") => {
                push_block(&mut blocks, &mut seen, value);
            }
            _ => {}
        }
    }

    if blocks.is_empty() {
        None
    } else {
        Some(blocks.join("\n\n"))
    }
}

fn push_block(
    blocks: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    candidate: &str,
) {
    let text = candidate.trim();
    if !text.is_empty() && seen.insert(text.to_owned()) {
        blocks.push(text.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use id3::frame::{Content, ExtendedText, Frame, Lyrics};

    fn lyric(description: &str, text: &str) -> Lyrics {
        Lyrics {
            lang: "eng".to_string(),
            description: description.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn extracts_metadata_when_artist_matches() {
        let mut tag = Tag::new();
        tag.set_artist("Studio Master");
        tag.set_title("Anthem");
        tag.add_frame(lyric("verse1", "Line one"));
        tag.add_frame(lyric("verse2", "Line two"));

        let metadata =
            extract_metadata(&tag, DEFAULT_ARTIST_FILTER).expect("metadata should be extracted");

        assert_eq!(metadata.artist, "Studio Master");
        assert_eq!(metadata.title, "Anthem");
        assert_eq!(metadata.lyrics, "Line one\n\nLine two");
    }

    #[test]
    fn uses_album_artist_when_primary_missing() {
        let mut tag = Tag::new();
        tag.set_album_artist("Audio Collective");
        tag.add_frame(lyric("", "Words"));

        let metadata =
            extract_metadata(&tag, DEFAULT_ARTIST_FILTER).expect("metadata should be extracted");

        assert_eq!(metadata.artist, "Audio Collective");
        assert_eq!(metadata.title, "Unknown Title");
        assert_eq!(metadata.lyrics, "Words");
    }

    #[test]
    fn skips_when_artist_does_not_match() {
        let mut tag = Tag::new();
        tag.set_artist("Composer");
        tag.add_frame(lyric("", "Words"));

        assert!(extract_metadata(&tag, DEFAULT_ARTIST_FILTER).is_none());
    }

    #[test]
    fn skips_when_lyrics_missing() {
        let mut tag = Tag::new();
        tag.set_artist("Studio Duo");

        assert!(extract_metadata(&tag, DEFAULT_ARTIST_FILTER).is_none());
    }

    #[test]
    fn ignores_empty_lyrics_frames() {
        let mut tag = Tag::new();
        tag.set_artist("Studio Duo");
        tag.add_frame(lyric("empty1", ""));
        tag.add_frame(lyric("empty2", "   "));
        tag.add_frame(lyric("lyric", "Verse"));

        let metadata =
            extract_metadata(&tag, DEFAULT_ARTIST_FILTER).expect("metadata should be extracted");
        assert_eq!(metadata.lyrics, "Verse");
    }

    #[test]
    fn extracts_lyrics_from_extended_text_frame() {
        let mut tag = Tag::new();
        tag.set_artist("Studio Duo");
        tag.add_frame(Frame::with_content(
            "TXXX",
            Content::ExtendedText(ExtendedText {
                description: "LYRICS".to_string(),
                value: "Block A".to_string(),
            }),
        ));
        tag.add_frame(Frame::with_content(
            "TXXX",
            Content::ExtendedText(ExtendedText {
                description: "Other".to_string(),
                value: "Ignore me".to_string(),
            }),
        ));

        let metadata =
            extract_metadata(&tag, DEFAULT_ARTIST_FILTER).expect("metadata should be extracted");
        assert_eq!(metadata.lyrics, "Block A");
    }
}
