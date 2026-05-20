use crate::{domain::Song, prompts::select_attributes};
use anyhow::Result;
use csv::Writer as CSV_Writer;
use serde_json::Map;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

const M3U_PREFIX: &str = "#EXTINF:";

pub(crate) fn write_csv(songs: &[Song], output: &PathBuf) -> Result<()> {
    let attrs = select_attributes()?;
    let mut wtr = CSV_Writer::from_path(output)?;

    wtr.write_record(attrs.iter().map(|f| f.to_string()))?; // HEADERS

    for song in songs {
        wtr.write_record(attrs.iter().map(|f| f.get_value(&song)))?;
    }

    Ok(())
}

pub(crate) fn write_json(songs: &[Song], output: &PathBuf) -> Result<()> {
    let attrs = select_attributes()?;

    let entries: Vec<Map<String, serde_json::Value>> = songs
        .iter()
        .map(|song| {
            attrs
                .iter()
                .map(|f| (f.key().to_string(), f.get_json_value(song)))
                .collect()
        })
        .collect();

    let file = File::create(output)?;
    let wtr = BufWriter::new(file);
    serde_json::to_writer_pretty(wtr, &entries)?;

    Ok(())
}

pub(crate) fn write_m3u(songs: &[Song], output: &PathBuf) -> Result<()> {
    let file = File::create(output)?;
    let mut wtr = BufWriter::new(file);

    write!(wtr, "#EXTM3U\r\n")?;
    for song in songs {
        let title = &song.title;
        let artist = &song.artist;
        let dur = song.duration.as_secs_f32().to_string();

        let path = PathBuf::from(&song.path).canonicalize()?;

        let path_str = path.to_string_lossy();
        let path_str = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);

        writeln!(wtr, "{M3U_PREFIX}{dur},{artist} - {title}\r")?;
        writeln!(wtr, "{path_str}\r")?;
    }

    Ok(())
}
