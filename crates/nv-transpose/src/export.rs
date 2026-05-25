use crate::{
    MAX_WIDTH,
    db::Database,
    domain::Filetype,
    prompts::{get_filename, get_output_dir, get_output_type, get_playlist},
    utils::{draw_box_fixed, get_config_path, truncate},
};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

const OUTPUT_MAX: usize = 15;

pub fn export() -> Result<()> {
    let db = Database::open()?;
    let playlists = db.fetch_playlists()?;

    if playlists.is_empty() {
        println!(
            "{} No playlists found in database, exiting.",
            " WARNING:".on_bright_yellow()
        );
        return Ok(());
    }

    println!();
    let playlist = get_playlist(playlists)?;
    let output_type = get_output_type()?;
    let songs = db.fetch_songs(playlist.id)?;

    let filename = get_filename(&playlist.name)?;
    let output_dir = get_output_dir(&get_config_path())?;
    let path = output_dir.join(format!("{}.{}", filename, output_type.extension()));

    output_type.export(&songs, &path)?;
    preview_output(&path, &output_type);

    println!(
        " {} Wrote {} tracks from {} to\n\t{}",
        "SUCCESS!".yellow(),
        format!("{}", songs.len()).green(),
        playlist.name.cyan(),
        path.display().dimmed()
    );

    Ok(())
}

fn preview_output(path: &PathBuf, io_type: &Filetype) {
    let Ok(file) = File::open(path) else { return };

    let content_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(MAX_WIDTH)
        .min(MAX_WIDTH)
        .saturating_sub(5);

    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .take(OUTPUT_MAX)
        .enumerate()
        .map(|(i, line)| {
            let raw = truncate(&line.unwrap_or_default(), content_width);
            io_type.colorize_line(&raw, i)
        })
        .collect();

    draw_box_fixed(&lines, content_width);
}
