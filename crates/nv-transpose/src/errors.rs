use std::fmt;

#[derive(Debug)]
pub enum TransposeError {
    PlaylistExists(String),
    NotEnoughAttributes,
    CsvRowError(String),
    NoSongsResolved,
    MalformedSongId,
}

impl fmt::Display for TransposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlaylistExists(name) => write!(f, "playlist already exists: {name}"),
            Self::CsvRowError(e) => write!(f, "Failed to parse csv row: {e}"),
            Self::NotEnoughAttributes => {
                write!(f, "Could not determine song from provided attributes.")
            }
            Self::NoSongsResolved => write!(
                f,
                "no songs from the import file could be matched against the library"
            ),
            Self::MalformedSongId => write!(f, "malformed song id in database (expected 8 bytes)"),
        }
    }
}

impl std::error::Error for TransposeError {}

impl TransposeError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::PlaylistExists(_) => "playlist already exists",
            Self::NoSongsResolved => "no songs resolved",
            Self::MalformedSongId => "malformed song id in database",
            Self::CsvRowError { .. } => "malformed row",
            Self::NotEnoughAttributes => "missing required fields",
        }
    }
}
