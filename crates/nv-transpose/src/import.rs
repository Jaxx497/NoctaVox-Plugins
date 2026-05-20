use crate::{domain::Filetype, errors::TransposeError, prompts::get_import};
use anyhow::{Result, anyhow};
use owo_colors::OwoColorize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct ImportSummary {
    pub playlist_name: String,
    pub imported: usize,
    pub parse_errors: Vec<(usize, TransposeError)>,
    pub unresolved: usize,
}

pub fn import() -> Result<()> {
    let usr_input = get_import()?;
    let path = validate_path(strip_quotes(&usr_input))?;
    let playlist_type = determine_filetype(&path)?;
    let summary = playlist_type.import(&path)?;

    report_summary(&summary);

    Ok(())
}

fn determine_filetype<P: AsRef<Path>>(p: P) -> Result<Filetype> {
    let path = p.as_ref();

    let ext = path
        .extension()
        .ok_or_else(|| anyhow!("Failed to detect extension"))?
        .to_string_lossy();

    Filetype::from_extension(ext)
        .ok_or_else(|| anyhow!("Illegal file extension. Please try again with a csv, m3u or m3u8"))
}

fn strip_quotes(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s)
}

fn validate_path<P: AsRef<Path>>(p: P) -> Result<PathBuf> {
    let path = p.as_ref();

    PathBuf::from(path)
        .canonicalize()
        .map_err(|_| anyhow!("Path not found on system: {}", path.display().dimmed()))
}

fn report_summary(s: &ImportSummary) {
    println!(
        " {}  Imported {} songs to {}",
        "SUCCESS".black().on_green(),
        s.imported.to_string().green(),
        s.playlist_name.cyan(),
    );

    let total_skipped = s.parse_errors.len() + s.unresolved;
    if total_skipped == 0 {
        return;
    }

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, err) in &s.parse_errors {
        *counts.entry(err.kind()).or_insert(0) += 1;
    }
    if s.unresolved > 0 {
        counts.insert("not found in library", s.unresolved);
    }

    println!(
        " {}  {} entries",
        "SKIPPED".black().on_yellow(),
        total_skipped.to_string().yellow(),
    );

    for (reason, n) in counts {
        println!(
            "             {} {} {}",
            "↳".dimmed(),
            n.yellow(),
            reason.dimmed(),
        );
    }
}
