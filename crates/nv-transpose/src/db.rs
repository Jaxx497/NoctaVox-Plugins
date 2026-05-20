use crate::{
    domain::{Playlist, Song},
    errors::TransposeError,
    utils::get_config_path,
};
use anyhow::Result;
use rusqlite::{Connection, params};
use std::{path::PathBuf, time::Duration};

pub struct Database {
    pub conn: Connection,
}

const DATABASE_FILENAME: &str = "noctavox.db";

impl Database {
    pub(crate) fn open() -> Result<Self> {
        let path = get_config_path().join(DATABASE_FILENAME);
        let conn = Connection::open(path)?;

        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;

        Ok(Database { conn })
    }

    pub(crate) fn fetch_playlists(&self) -> anyhow::Result<Vec<Playlist>> {
        let playlists = self
            .conn
            .prepare(GET_PLAYLISTS)?
            .query_map([], |row| {
                Ok(Playlist {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(playlists)
    }

    pub(crate) fn fetch_songs(&self, playlist_id: i64) -> anyhow::Result<Vec<Song>> {
        let songs = self
            .conn
            .prepare(GET_PLAYLIST_SONGS)?
            .query_map([playlist_id], |row| {
                Ok(Song {
                    id: None,
                    title: row.get("title")?,
                    artist: row.get("artist")?,
                    album: row.get("album")?,
                    duration: Duration::from_secs_f32(row.get("duration")?),
                    path: row.get("path")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(songs)
    }

    pub(crate) fn resolve_songs_by_path(&mut self, songs: &Vec<Song>) -> Result<Vec<Option<u64>>> {
        let mut stmt = self.conn.prepare(GET_IDS_FROM_PATH)?;
        let mut resolved = Vec::with_capacity(songs.len());

        for song in songs.iter() {
            let canon = PathBuf::from(&song.path)
                .canonicalize()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| song.path.clone());

            match stmt.query_row([canon], |row| row.get::<_, Vec<u8>>("id")) {
                Ok(bytes) => resolved.push(Some(convert_from_bytes(bytes)?)),
                Err(rusqlite::Error::QueryReturnedNoRows) => resolved.push(None),
                Err(e) => return Err(e.into()),
            }
        }

        Ok(resolved)
    }

    pub fn write_new_playlist<S: AsRef<str>>(&mut self, n: S, song_ids: &[u64]) -> Result<()> {
        if song_ids.is_empty() {
            return Err(TransposeError::NoSongsResolved.into());
        }

        let name = n.as_ref();
        let tx = self.conn.transaction()?;

        let rows_inserted = tx.execute(CREATE_NEW_PLAYLIST, params![name])?;
        if rows_inserted == 0 {
            return Err(TransposeError::PlaylistExists(name.to_string()).into());
        }

        let playlist_id = tx.last_insert_rowid();

        {
            let mut stmt = tx.prepare(ADD_SONG_TO_PLAYLIST_WITH_POSITION)?;

            for (idx, song_id) in song_ids.iter().enumerate() {
                let position = (idx + 1) as i64;
                let song_id_bytes = song_id.to_le_bytes();
                stmt.execute(params![song_id_bytes, playlist_id, position])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn load_library(&self) -> Result<Vec<Song>> {
        self.conn
            .prepare(GET_ALL_LIBRARY_SONGS)?
            .query_map(params![], |row| {
                Ok(Song {
                    id: Some(convert_from_bytes(row.get::<_, Vec<u8>>("id")?).unwrap_or(0)),
                    title: row.get("title")?,
                    artist: row.get("artist")?,
                    album: row.get("album")?,
                    duration: Duration::from_secs_f32(row.get("duration")?),
                    path: row.get("path")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn convert_from_bytes(raw_bytes: Vec<u8>) -> Result<u64> {
    let hash_array: [u8; 8] = raw_bytes
        .try_into()
        .map_err(|_| TransposeError::MalformedSongId)?;
    Ok(u64::from_le_bytes(hash_array))
}

pub const GET_PLAYLISTS: &str = r"
SELECT id, name 
    FROM playlists 
    ORDER BY updated_at DESC";

pub const GET_PLAYLIST_SONGS: &str = r"
SELECT 
    s.title AS title, 
    COALESCE(ar.name, '[EMPTY]') AS artist,
    COALESCE(al.title, '[EMPTY]') AS album,
    s.duration AS duration,
    s.path AS path
    FROM playlist_songs ps
INNER JOIN songs s ON ps.song_id = s.id
LEFT JOIN artists ar ON s.artist_id = ar.id
LEFT JOIN albums al ON s.album_id = al.id
WHERE ps.playlist_id = ?1
ORDER BY ps.position";

pub const GET_IDS_FROM_PATH: &str = "SELECT id from SONGS WHERE path = ?1";

pub const CREATE_NEW_PLAYLIST: &str = "
    INSERT OR IGNORE INTO playlists (name, updated_at) 
        VALUES (?, strftime('%s', 'now'))";

pub const ADD_SONG_TO_PLAYLIST_WITH_POSITION: &str = "
    INSERT INTO playlist_songs (
        song_id, 
        playlist_id, 
        position
    )
    VALUES (
        ?1, 
        ?2, 
        ?3
    )";

pub const GET_ALL_LIBRARY_SONGS: &str = "
  SELECT
      s.id AS id,
      s.title AS title,
      COALESCE(ar.name, '')  AS artist,
      COALESCE(al.title, '') AS album,
      s.duration AS duration,
      s.path AS path
  FROM songs s
  LEFT JOIN artists ar ON s.artist_id = ar.id
  LEFT JOIN albums al ON s.album_id = al.id
  ";
