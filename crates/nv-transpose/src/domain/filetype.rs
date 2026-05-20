use anyhow::{anyhow, bail};
use crossterm::style::Stylize;
use owo_colors::OwoColorize;
use std::path::PathBuf;

use crate::{
    db::Database, domain::Song, fuzzy::resolve_by_headers, import::ImportSummary,
    prompts::set_playlist_name, readers, writers,
};

pub enum Filetype {
    CSV,
    M3U,
    JSON,
}

impl Filetype {
    pub fn from_extension<S: AsRef<str>>(xt: S) -> Option<Filetype> {
        let ext = xt.as_ref();

        match ext {
            "csv" => Some(Filetype::CSV),
            "json" => Some(Filetype::JSON),
            "m3u" | "m3u8" => Some(Filetype::M3U),
            _ => None,
        }
    }

    pub fn get_output_types() -> Vec<Filetype> {
        vec![Filetype::CSV, Filetype::M3U, Filetype::JSON]
    }

    pub fn extension(&self) -> &str {
        match &self {
            Filetype::CSV => "csv",
            Filetype::JSON => "json",
            Filetype::M3U => "m3u",
        }
    }

    pub fn import(&self, path: &PathBuf) -> anyhow::Result<ImportSummary> {
        let (valid, invalid) = match self {
            Filetype::CSV => readers::read_csv(path),
            Filetype::JSON => bail!("Importing from JSON is not supported"),
            Filetype::M3U => readers::read_m3u(path),
        }?;

        let default_name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Failed to determine file name"))?;

        let playlist_name = set_playlist_name(default_name)?;

        let mut db = Database::open()?;
        let mut resolution = db.resolve_songs_by_path(&valid)?;

        let missed: Vec<(usize, &Song)> = resolution
            .iter()
            .enumerate()
            .filter_map(|(idx, slot)| slot.is_none().then(|| (idx, &valid[idx])))
            .collect();

        let unresolved_count = if !missed.is_empty() {
            println!(
                " {}  Searching library for {} unmatched tracks...",
                "MATCHING".black().on_blue(),
                missed.len().to_string().cyan(),
            );

            let lib = db.load_library()?;
            let (fuzzy_hits, still_missing) = resolve_by_headers(&lib, &missed);

            let recovered = fuzzy_hits.len();
            for (idx, id) in fuzzy_hits {
                resolution[idx] = Some(id);
            }

            if recovered > 0 {
                println!(
                    "             {} recovered {} by metadata",
                    "↳".dimmed(),
                    recovered.to_string().green(),
                );
            }

            still_missing.len()
        } else {
            0
        };

        let resolved: Vec<u64> = resolution.into_iter().flatten().collect();
        db.write_new_playlist(&playlist_name, &resolved)?;

        Ok(ImportSummary {
            playlist_name,
            imported: resolved.len(),
            parse_errors: invalid,
            unresolved: unresolved_count,
        })
    }

    pub fn export(&self, songs: &[Song], path: &PathBuf) -> anyhow::Result<()> {
        match &self {
            Filetype::CSV => writers::write_csv(songs, path),
            Filetype::M3U => writers::write_m3u(songs, path),
            Filetype::JSON => writers::write_json(songs, path),
        }
    }

    pub fn colorize_line(&self, line: &str, idx: usize) -> String {
        match self {
            Filetype::M3U => {
                if line.starts_with("#EXTM3U") {
                    line.dimmed().to_string()
                } else if line.starts_with("#EXTINF") {
                    line.dark_yellow().to_string()
                } else {
                    line.dimmed().to_string()
                }
            }
            Filetype::CSV => {
                if idx == 0 {
                    line.bold().bright_cyan().to_string()
                } else if idx % 2 == 0 {
                    line.to_string()
                } else {
                    line.dimmed().to_string()
                }
            }
            Filetype::JSON => {
                let trimmed = line.trim_start();
                if matches!(
                    trimmed.chars().next(),
                    Some('[') | Some(']') | Some('{') | Some('}')
                ) {
                    line.dimmed().to_string()
                } else if let Some((key, val)) = line.split_once(':') {
                    format!("{}:{}", key.bright_cyan(), val)
                } else {
                    line.to_string()
                }
            }
        }
    }
}

impl std::fmt::Display for Filetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Filetype::CSV => f.write_str("CSV"),
            Filetype::JSON => f.write_str("JSON"),
            Filetype::M3U => f.write_str("M3U"),
        }
    }
}
