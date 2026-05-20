mod db;
mod domain;
mod errors;
mod export;
mod fuzzy;
mod import;
mod prompts;
mod readers;
mod utils;
mod writers;

use clap::{ArgGroup, CommandFactory, Parser, ValueEnum};
use owo_colors::OwoColorize;

use crate::utils::list_playlists;

const MAX_WIDTH: usize = 80;
const CONFIG_DIRECTORY: &str = "noctavox";

fn main() {
    let cli = Cli::parse();

    let res = if cli.import {
        import::import()
    } else if cli.export {
        export::export()
    } else if cli.list {
        list_playlists()
    } else {
        Cli::command().print_help().expect("Could not print help");
        println!();
        Ok(())
    };

    if let Err(e) = res {
        eprintln!(" {}  {e}", "ERROR".on_bright_red())
    }

    println!()
}

#[derive(Parser, Debug)]
#[command(
    name = "nv-transpose",
    version,
    about = "NoctaVox extension to import/export playlists"
)]
#[command(group(
      ArgGroup::new("mode")
          .args(["import", "export", "list"]),
  ))]

struct Cli {
    /// Run import session
    #[arg(long)]
    import: bool,

    /// Run export session
    #[arg(long)]
    export: bool,

    /// List playlists in the library and exit.
    #[arg(long)]
    list: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum FormatArg {
    Csv,
    M3u,
    M3u8,
}
