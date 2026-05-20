use crate::domain::{Filetype, Playlist, SongField};
use anyhow::Result;
use inquire::{
    MultiSelect, Select, Text,
    list_option::ListOption,
    ui::{Color, ErrorMessageRenderConfig, RenderConfig, Styled},
    validator::{ErrorMessage, Validation},
};

const SELECTOR: &str = " ";
const ICON_PLAYLIST: &str = " 󰲸";
const ICON_FILE: &str = " ";
const ICON_NAME: &str = " ";
const ICON_TAG: &str = " ";

const PROMPT_WIDTH: usize = 22;

fn pad(label: &str) -> String {
    format!("{label:<PROMPT_WIDTH$}")
}

pub(crate) fn get_output_type() -> Result<Filetype> {
    let output_type = Filetype::get_output_types();
    Ok(Select::new(&pad("Select output type:"), output_type)
        .with_vim_mode(true)
        .without_filtering()
        .with_render_config(render_config(ICON_FILE))
        .prompt()?)
}

pub(crate) fn get_playlist(playlists: Vec<Playlist>) -> Result<Playlist> {
    let select_playlist = format!("{}", pad("Select a playlist:"));

    Ok(Select::new(&select_playlist, playlists)
        .with_vim_mode(true)
        .without_filtering()
        .with_render_config(render_config(ICON_PLAYLIST))
        .prompt()?)
}

pub(crate) fn get_import() -> Result<String> {
    Ok(Text::new(&pad("Enter file to import:"))
        .with_placeholder("Absolute path!")
        .with_render_config(render_config(ICON_FILE))
        .prompt()?)
}

pub(crate) fn get_filename(playlist_name: &str) -> Result<String> {
    let input = Text::new(&pad("Output name:"))
        .with_default(playlist_name)
        .with_render_config(render_config(ICON_NAME))
        .prompt()?;

    let filename = input
        .trim()
        .is_empty()
        .then(|| playlist_name)
        .unwrap_or(&input);

    Ok(filename.split('.').next().unwrap().trim().to_string())
}

pub(crate) fn set_playlist_name(default: &str) -> Result<String> {
    let input = Text::new(&pad("Set playlist name"))
        .with_default(default)
        .with_render_config(render_config(ICON_PLAYLIST))
        .prompt()?;

    let filename = input.trim().is_empty().then(|| default).unwrap_or(&input);

    Ok(filename.split('.').next().unwrap().trim().to_string())
}

pub(crate) fn select_attributes() -> Result<Vec<SongField>> {
    Ok(
        MultiSelect::new(&pad("Select attributes:"), SongField::get_fields())
            .with_vim_mode(true)
            .without_filtering()
            .with_validator(
                |selections: &[ListOption<&SongField>]| match selections.is_empty() {
                    true => Ok(Validation::Invalid(ErrorMessage::from(
                        "Please select at least one attribute",
                    ))),

                    false => Ok(Validation::Valid),
                },
            )
            .with_render_config(render_config(ICON_TAG))
            .prompt()?,
    )
}

fn render_config(icon: &'static str) -> RenderConfig<'static> {
    let mut cfg = RenderConfig::default()
        .with_prompt_prefix(Styled::new(icon).with_fg(Color::LightGreen))
        .with_answered_prompt_prefix(Styled::new(icon).with_fg(Color::DarkGreen))
        .with_highlighted_option_prefix(Styled::new(SELECTOR).with_fg(Color::LightCyan))
        .with_error_message(
            ErrorMessageRenderConfig::default_colored()
                .with_prefix(Styled::new(" X").with_fg(Color::LightRed)),
        );

    cfg.unhighlighted_option_prefix = Styled::new("  ").with_fg(Color::LightCyan);
    cfg
}
