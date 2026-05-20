use anyhow::{Result, anyhow, bail};
use csv::ReaderBuilder;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    domain::{Song, SongField},
    errors::TransposeError,
};

pub(crate) fn read_csv(input: &PathBuf) -> Result<(Vec<Song>, Vec<(usize, TransposeError)>)> {
    let mut reader = ReaderBuilder::new().has_headers(true).from_path(input)?;
    let base_dir = input.parent();

    let column_map: Vec<Option<SongField>> = reader
        .headers()
        .map_err(|_| anyhow!("CSV reader could not parse headers"))?
        .iter()
        .map(SongField::from_str)
        .collect();

    if column_map.iter().all(|f| f.is_none()) {
        bail!(
            "No recognized headers found. Please ensure your CSV has legal headers: Title, Artist, Duration, Path"
        );
    }

    let mut songs = Vec::new();
    let mut invalid = Vec::new();
    for (idx, result) in reader.records().enumerate() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                invalid.push((idx, TransposeError::CsvRowError(e.to_string())));
                continue;
            }
        };

        let mut song = Song::default();

        for (field, value) in column_map.iter().zip(record.iter()) {
            match field {
                Some(SongField::Title) => song.title = value.to_string(),
                Some(SongField::Artist) => song.artist = value.to_string(),
                Some(SongField::Album) => song.album = value.to_string(),
                Some(SongField::Path) => song.path = resolve_path(value, base_dir),
                Some(SongField::Duration) => {
                    song.duration = value
                        .parse::<u64>()
                        .map(Duration::from_secs)
                        .unwrap_or_default();
                }
                Some(SongField::DurationMs) => {
                    song.duration = value
                        .parse::<u64>()
                        .map(Duration::from_millis)
                        .unwrap_or_default();
                }
                None => {} // skip invalid fields
            }
        }

        if song.path.is_empty() && song.title.is_empty() {
            invalid.push((idx, TransposeError::NotEnoughAttributes));
            continue;
        }

        songs.push(song);
    }

    Ok((songs, invalid))
}

pub(crate) fn read_m3u(path: &PathBuf) -> Result<(Vec<Song>, Vec<(usize, TransposeError)>)> {
    let raw = std::fs::read_to_string(path)?;
    let content = raw.strip_prefix('\u{FEFF}').unwrap_or(&raw);
    let base_dir = path.parent();

    let mut songs = Vec::new();
    let mut invalid = Vec::new();
    let mut pending: Option<Song> = None;

    for (idx, line) in content.lines().enumerate() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            pending = Some(parse_extinf(rest));
            continue;
        }

        if line.starts_with('#') {
            continue;
        }

        let mut song = pending.take().unwrap_or_default();
        song.path = resolve_path(line, base_dir);

        if song.path.is_empty() {
            invalid.push((idx, TransposeError::NotEnoughAttributes));
            continue;
        }
        songs.push(song);
    }

    Ok((songs, invalid))
}

fn parse_extinf(rest: &str) -> Song {
    let (dur_str, label) = rest.split_once(',').unwrap_or((rest, ""));

    let duration = dur_str
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|s| *s >= 0.0)
        .map(Duration::from_secs_f32)
        .unwrap_or_default();

    let (artist, title) = match label.split_once(" - ") {
        Some((a, t)) => (a.trim().to_string(), t.trim().to_string()),
        None => (String::new(), label.trim().to_string()),
    };

    Song {
        title,
        artist,
        duration,
        ..Song::default()
    }
}

fn resolve_path(line: &str, base_dir: Option<&Path>) -> String {
    let p = Path::new(line);
    if p.is_absolute() {
        return line.to_string();
    }

    match base_dir {
        Some(base) => base.join(p).to_string_lossy().into_owned(),
        None => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extinf_standard_format() {
        let s = parse_extinf("123,Radiohead - Karma Police");
        assert_eq!(s.title, "Karma Police");
        assert_eq!(s.artist, "Radiohead");
        assert_eq!(s.duration, Duration::from_secs(123));
    }

    #[test]
    fn parse_extinf_float_duration() {
        let s = parse_extinf("123.45,Artist - Title");
        assert!((s.duration.as_secs_f32() - 123.45).abs() < 1e-3);
    }

    #[test]
    fn parse_extinf_negative_duration_becomes_zero() {
        let s = parse_extinf("-1,Artist - Title");
        assert_eq!(s.duration, Duration::ZERO);
    }

    #[test]
    fn parse_extinf_unparseable_duration_becomes_zero() {
        let s = parse_extinf("notanumber,Artist - Title");
        assert_eq!(s.duration, Duration::ZERO);
    }

    #[test]
    fn parse_extinf_no_separator_treats_label_as_title() {
        let s = parse_extinf("120,Just a Title");
        assert_eq!(s.title, "Just a Title");
        assert_eq!(s.artist, "");
    }

    #[test]
    fn parse_extinf_empty_label() {
        let s = parse_extinf("120,");
        assert_eq!(s.title, "");
        assert_eq!(s.artist, "");
        assert_eq!(s.duration, Duration::from_secs(120));
    }

    #[test]
    fn parse_extinf_trims_whitespace() {
        let s = parse_extinf("120,  Artist  -  Title  ");
        assert_eq!(s.title, "Title");
        assert_eq!(s.artist, "Artist");
    }

    #[test]
    fn parse_extinf_id_is_none() {
        // Imported songs should never carry a database id.
        let s = parse_extinf("120,Artist - Title");
        assert!(s.id.is_none());
    }

    #[test]
    fn resolve_path_absolute_passes_through() {
        let base = Path::new("/some/playlist/dir");
        #[cfg(unix)]
        {
            let result = resolve_path("/absolute/path/song.flac", Some(base));
            assert_eq!(result, "/absolute/path/song.flac");
        }
        #[cfg(windows)]
        {
            let result = resolve_path(r"C:\absolute\path\song.flac", Some(base));
            assert_eq!(result, r"C:\absolute\path\song.flac");
        }
    }

    #[test]
    fn resolve_path_relative_joins_base_dir() {
        let base = Path::new("/playlists");
        let result = resolve_path("tracks/song.flac", Some(base));
        // The join may use platform-native separators; check both ends.
        assert!(result.starts_with("/playlists"));
        assert!(result.ends_with("song.flac"));
    }

    #[test]
    fn resolve_path_relative_with_no_base_returns_input() {
        let result = resolve_path("tracks/song.flac", None);
        assert_eq!(result, "tracks/song.flac");
    }
}
