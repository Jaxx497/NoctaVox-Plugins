use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{MAX_WIDTH, db::Database};

const ID_COL: usize = 3;
const GAP: usize = 5;

pub(crate) fn draw_box_fixed(lines: &[String], content_width: usize) {
    let bar = "─".repeat(content_width + 2);
    let pipe = "│".dimmed();

    println!(" {}", format!("╭{bar}╮").dimmed());
    for line in lines {
        println!(" {pipe} {line} {pipe}");
    }
    println!(" {}", format!("╰{bar}╯").dimmed());
}

pub(crate) fn truncate<S: AsRef<str>>(s: S, max: usize) -> String {
    let s = s.as_ref();

    let total = UnicodeWidthStr::width(s);
    if total <= max {
        return format!("{s}{}", " ".repeat(max - total));
    }
    let mut width = 0;
    let mut end = 0;
    for (i, c) in s.char_indices() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(0);
        if width + cw + 1 > max {
            break;
        }
        width += cw;
        end = i + c.len_utf8();
    }
    format!(
        "{}…{}",
        &s[..end],
        " ".repeat(max.saturating_sub(width + 1))
    )
}

pub fn list_playlists() -> Result<()> {
    let db = Database::open()?;
    let mut playlists = db.fetch_playlists()?;

    playlists.sort_by_key(|a| a.id);

    let format_raw = |id: i64, name: &str| format!("{:ID_COL$}.{:<GAP$}{}", id, "", name);

    let content_width = playlists
        .iter()
        .map(|p| UnicodeWidthStr::width(format_raw(p.id, &p.name).as_str())) // id col + spacing
        .max()
        .unwrap_or(0)
        .min(MAX_WIDTH);

    let template = |id: i64, name: &str| {
        let raw = format_raw(id, name);
        let styled = format!("{:>ID_COL$}.{:<GAP$}{}", id.cyan(), "", name.green());
        let padding =
            " ".repeat(content_width.saturating_sub(UnicodeWidthStr::width(raw.as_str())));
        format!("{}{}", styled, padding)
    };

    let lines = playlists
        .iter()
        .map(|p| template(p.id, &p.name))
        .collect::<Vec<String>>();

    draw_box_fixed(&lines, content_width);

    Ok(())
}

pub fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not resolve config directory")
        .join(crate::CONFIG_DIRECTORY)
}
