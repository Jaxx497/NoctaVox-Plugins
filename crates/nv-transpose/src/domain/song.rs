use std::time::Duration;

#[derive(Default, Debug)]
pub struct Song {
    pub id: Option<u64>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
    pub path: String,
}

pub enum SongField {
    Title,
    Artist,
    Album,
    Duration,
    DurationMs,
    Path,
}

impl SongField {
    pub fn get_fields() -> Vec<SongField> {
        use SongField::*;
        // DurationMs is input-only (Spotify-style imports); exports always use seconds.
        vec![Title, Artist, Album, Duration, Path]
    }

    pub fn from_str(s: &str) -> Option<SongField> {
        let lower = s.trim().to_lowercase();
        match lower.as_str() {
            "title" | "track name" | "name" => Some(SongField::Title),
            "artist" | "artists" | "artist name" | "artist name(s)" | "artist names" => {
                Some(SongField::Artist)
            }
            "album" | "album name" => Some(SongField::Album),
            "path" | "file" | "filename" | "location" => Some(SongField::Path),
            "track duration (ms)" | "duration (ms)" | "duration_ms" | "length (ms)" => {
                Some(SongField::DurationMs)
            }
            "duration" | "length" | "time" | "track duration" => Some(SongField::Duration),
            _ => None,
        }
    }

    pub fn get_value(&self, song: &Song) -> String {
        match self {
            SongField::Title => song.title.to_string(),
            SongField::Artist => song.artist.to_string(),
            SongField::Album => song.album.to_string(),
            SongField::Duration | SongField::DurationMs => format!("{}", song.duration.as_secs()),
            SongField::Path => song.path.to_string(),
        }
    }

    pub fn get_json_value(&self, song: &Song) -> serde_json::Value {
        match self {
            SongField::Title => song.title.clone().into(),
            SongField::Artist => song.artist.clone().into(),
            SongField::Album => song.album.clone().into(),
            SongField::Duration | SongField::DurationMs => song.duration.as_secs().into(),
            SongField::Path => song.path.clone().into(),
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            SongField::Title => "title",
            SongField::Artist => "artist",
            SongField::Album => "album",
            SongField::Duration | SongField::DurationMs => "duration",
            SongField::Path => "path",
        }
    }
}

impl std::fmt::Display for SongField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SongField::Title => f.write_str("Title"),
            SongField::Artist => f.write_str("Artist"),
            SongField::Album => f.write_str("Album"),
            SongField::Duration | SongField::DurationMs => f.write_str("Duration"),
            SongField::Path => f.write_str("Path"),
        }
    }
}
