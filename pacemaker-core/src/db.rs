//! SQLite reader and writer for Tonium Pacemaker `music.db`.

use rusqlite::{params, Connection, OptionalExtension, Result};
use crate::types::{CaseItem, TrackId, TrackMetadata, TimeMs};
use crate::playlist::Playlist;

const TRACK_SELECT_COLS: &str = "track_id, title, artist, album, genre, year, \
    bpm, play_time_secs, cue_point, loop_in, loop_out, \
    location, key, rating, comments";

const TRACK_JOIN_COLS: &str = "t.track_id, t.title, t.artist, t.album, t.genre, t.year, \
    t.bpm, t.play_time_secs, t.cue_point, t.loop_in, t.loop_out, \
    t.location, t.key, t.rating, t.comments";

fn map_track_row(row: &rusqlite::Row) -> rusqlite::Result<TrackMetadata> {
    Ok(TrackMetadata {
        id: row.get(0)?,
        title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        artist: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        album: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        genre: row.get::<_, Option<String>>(4)?.unwrap_or_else(|| "Music".into()),
        year: row.get::<_, Option<u32>>(5)?.unwrap_or(0),
        bpm: row.get::<_, Option<f64>>(6)?.unwrap_or(120.0),
        play_time_secs: row.get::<_, Option<u32>>(7)?.unwrap_or(0),
        cue_point_ms: row.get::<_, Option<i64>>(8)?.unwrap_or(-1),
        loop_in_ms: row.get::<_, Option<i64>>(9)?.unwrap_or(-1),
        loop_out_ms: row.get::<_, Option<i64>>(10)?.unwrap_or(-1),
        virtual_location: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
        key: row.get::<_, Option<String>>(12)?.unwrap_or_default(),
        rating: row.get::<_, Option<u8>>(13)?.unwrap_or(0),
        comments: row.get::<_, Option<String>>(14)?.unwrap_or_default(),
    })
}

pub struct PacemakerDb {
    conn: Connection,
}

impl PacemakerDb {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        // Ultra-low memory profile for embedded & memory-constrained hardware:
        // - busy_timeout: 5s timeout preventing database locked errors on slow FAT32.
        // - synchronous = NORMAL: balanced durability without high I/O.
        // - cache_size = -512: strictly caps SQLite page cache to 512 KB in RAM.
        // - temp_store = FILE: prevents temporary B-Trees from consuming scarce RAM.
        // - mmap_size = 0: zero memory mapping to eliminate virtual memory bloat.
        // - threads = 0: single-threaded operation saves thread stack allocations.
        // - CREATE INDEX: eliminates in-memory sorting B-Trees during queries.
        let _ = conn.execute_batch("
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -512;
            PRAGMA temp_store = FILE;
            PRAGMA mmap_size = 0;
            PRAGMA threads = 0;
            CREATE INDEX IF NOT EXISTS idx_casetracks_case_id ON casetracks (case_id);
            CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks (artist);
            CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks (title);
        ");
        Ok(Self { conn })
    }

    /// Releases freed SQLite pages and memory buffers back to the operating system
    pub fn shrink_memory(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA shrink_memory;")?;
        Ok(())
    }

    pub fn get_tracks(
        &self,
        search: Option<&str>,
        case_id: Option<i64>,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
    ) -> Result<Vec<TrackMetadata>> {
        let mut sql = String::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(cid) = case_id {
            sql.push_str(&format!(
                "SELECT {} FROM tracks t \
                 JOIN casetracks ct ON t.track_id = ct.track_id \
                 WHERE ct.case_id = ?",
                TRACK_JOIN_COLS
            ));
            params.push(Box::new(cid));
        } else {
            sql.push_str(&format!(
                "SELECT {} FROM tracks WHERE 1=1",
                TRACK_SELECT_COLS
            ));
        }

        if let Some(term) = search {
            let trimmed = term.trim();
            if !trimmed.is_empty() {
                let pattern = format!("%{}%", trimmed);
                sql.push_str(" AND (title LIKE ? OR artist LIKE ? OR album LIKE ? OR genre LIKE ?)");
                params.push(Box::new(pattern.clone()));
                params.push(Box::new(pattern.clone()));
                params.push(Box::new(pattern.clone()));
                params.push(Box::new(pattern));
            }
        }

        let allowed_cols = ["artist", "title", "album", "bpm", "play_time_secs", "date_added", "year"];
        let col = match sort_by {
            Some(c) if allowed_cols.contains(&c) => c,
            _ => "artist",
        };
        let ord = match sort_order {
            Some(o) if o.eq_ignore_ascii_case("DESC") => "DESC",
            _ => "ASC",
        };

        sql.push_str(&format!(" ORDER BY {} {}", col, ord));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), map_track_row)?;

        let mut tracks = Vec::new();
        for r in rows {
            tracks.push(r?);
        }
        Ok(tracks)
    }

    pub fn get_track(&self, track_id: TrackId) -> Result<Option<TrackMetadata>> {
        let sql = format!("SELECT {} FROM tracks WHERE track_id = ?1", TRACK_SELECT_COLS);
        self.conn.query_row(&sql, params![track_id], map_track_row).optional()
    }

    pub fn set_cue_point(&self, track_id: TrackId, cue_point_ms: TimeMs) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET cue_point = ?1, date_modified = ?2 WHERE track_id = ?3",
            params![cue_point_ms, chrono_now_ms(), track_id],
        )?;
        Ok(())
    }

    pub fn set_loop_points(&self, track_id: TrackId, loop_in_ms: TimeMs, loop_out_ms: TimeMs) -> Result<()> {
        self.conn.execute(
            "UPDATE tracks SET loop_in = ?1, loop_out = ?2, date_modified = ?3 WHERE track_id = ?4",
            params![loop_in_ms, loop_out_ms, chrono_now_ms(), track_id],
        )?;
        Ok(())
    }

    /// Safely writes back analysis results (BPM, CUE, loop points, KEY)
    /// into SQLite with atomic commits, preventing the silent data drop observed
    /// in Tonium firmware `pmplayer` (TrackAnalyzer exception swallowing - Issue #1).
    pub fn write_analysis_result(
        &mut self,
        track_id: TrackId,
        bpm: Option<f64>,
        cue_point: Option<TimeMs>,
        loop_in: Option<TimeMs>,
        loop_out: Option<TimeMs>,
        key: Option<&str>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        let now = chrono_now_ms();

        if let Some(b) = bpm {
            let ind_b = ((b / 10.0).floor() as i64) * 10;
            tx.execute(
                "UPDATE tracks SET bpm = ?1, ind_bpm = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![b, ind_b, now, track_id],
            )?;
        }
        if let Some(cue) = cue_point {
            tx.execute(
                "UPDATE tracks SET cue_point = ?1, date_modified = ?2 WHERE track_id = ?3",
                params![cue, now, track_id],
            )?;
        }
        if let (Some(l_in), Some(l_out)) = (loop_in, loop_out) {
            tx.execute(
                "UPDATE tracks SET loop_in = ?1, loop_out = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![l_in, l_out, now, track_id],
            )?;
        }
        if let Some(k) = key {
            tx.execute(
                "UPDATE tracks SET key = ?1, date_modified = ?2 WHERE track_id = ?3",
                params![k, now, track_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn update_track_metadata(
        &self,
        track_id: TrackId,
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        genre: Option<&str>,
        year: Option<u32>,
        bpm: Option<f64>,
        key: Option<&str>,
        comments: Option<&str>,
    ) -> Result<()> {
        let now = chrono_now_ms();
        let tx = self.conn.unchecked_transaction()?;

        if let Some(t) = title {
            let ind = t.chars().take(32).collect::<String>().to_lowercase();
            tx.execute(
                "UPDATE tracks SET title = ?1, ind_title = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![t, ind, now, track_id],
            )?;
        }
        if let Some(a) = artist {
            let ind = a.chars().take(32).collect::<String>().to_lowercase();
            tx.execute(
                "UPDATE tracks SET artist = ?1, ind_artist = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![a, ind, now, track_id],
            )?;
        }
        if let Some(al) = album {
            let ind = al.chars().take(32).collect::<String>().to_lowercase();
            tx.execute(
                "UPDATE tracks SET album = ?1, ind_album = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![al, ind, now, track_id],
            )?;
        }
        if let Some(g) = genre {
            let ind = g.chars().take(32).collect::<String>().to_lowercase();
            tx.execute(
                "UPDATE tracks SET genre = ?1, ind_genre = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![g, ind, now, track_id],
            )?;
        }
        if let Some(y) = year {
            tx.execute(
                "UPDATE tracks SET year = ?1, date_modified = ?2 WHERE track_id = ?3",
                params![y, now, track_id],
            )?;
        }
        if let Some(b) = bpm {
            let ind_b = ((b / 10.0).floor() as i64) * 10;
            tx.execute(
                "UPDATE tracks SET bpm = ?1, ind_bpm = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![b, ind_b, now, track_id],
            )?;
        }
        if let Some(k) = key {
            tx.execute(
                "UPDATE tracks SET key = ?1, date_modified = ?2 WHERE track_id = ?3",
                params![k, now, track_id],
            )?;
        }
        if let Some(c) = comments {
            tx.execute(
                "UPDATE tracks SET comments = ?1, date_modified = ?2 WHERE track_id = ?3",
                params![c, now, track_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_unindexed_bpm_tracks(&self) -> Result<Vec<(TrackId, String)>> {
        let mut stmt = self.conn.prepare("SELECT track_id, location FROM tracks WHERE bpm IS NULL OR bpm <= 0")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default()))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn delete_track(&self, track_id: TrackId) -> Result<()> {
        self.conn.execute("DELETE FROM tracks WHERE track_id = ?1", params![track_id])?;
        self.conn.execute("DELETE FROM casetracks WHERE track_id = ?1", params![track_id])?;
        self.conn.execute("DELETE FROM mixtracks WHERE track_id = ?1", params![track_id])?;
        Ok(())
    }

    pub fn get_cases(&self) -> Result<Vec<CaseItem>> {
        let mut stmt = self.conn.prepare("SELECT case_id, name, date_created, date_modified FROM cases ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(CaseItem {
                case_id: row.get(0)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                date_created: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                date_modified: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
            })
        })?;

        let mut cases = Vec::new();
        for r in rows {
            cases.push(r?);
        }
        Ok(cases)
    }

    pub fn create_case(&self, name: &str) -> Result<i64> {
        let now = chrono_now_ms();
        self.conn.execute(
            "INSERT INTO cases (name, date_created, times_played, rating, date_modified) VALUES (?1, ?2, 0, 0, ?2)",
            params![name, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_case(&self, case_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM cases WHERE case_id = ?1", params![case_id])?;
        self.conn.execute("DELETE FROM casetracks WHERE case_id = ?1", params![case_id])?;
        Ok(())
    }

    pub fn add_track_to_case(&self, case_id: i64, track_id: TrackId) -> Result<()> {
        let exists: bool = self.conn.query_row(
            "SELECT 1 FROM casetracks WHERE case_id = ?1 AND track_id = ?2",
            params![case_id, track_id],
            |_| Ok(true),
        ).unwrap_or(false);

        if !exists {
            self.conn.execute(
                "INSERT INTO casetracks (case_id, track_id) VALUES (?1, ?2)",
                params![case_id, track_id],
            )?;
        }
        Ok(())
    }

    pub fn remove_track_from_case(&self, case_id: i64, track_id: TrackId) -> Result<()> {
        self.conn.execute(
            "DELETE FROM casetracks WHERE case_id = ?1 AND track_id = ?2",
            params![case_id, track_id],
        )?;
        Ok(())
    }

    /// Loads a `Playlist` from an existing Pacemaker Case
    pub fn get_playlist(&self, case_id: i64) -> Result<Option<Playlist>> {
        let name_res: Option<String> = self.conn.query_row(
            "SELECT name FROM cases WHERE case_id = ?1",
            params![case_id],
            |row| row.get(0),
        ).optional()?;

        let Some(name) = name_res else {
            return Ok(None);
        };

        let sql = format!(
            "SELECT {} FROM tracks t \
             JOIN casetracks ct ON t.track_id = ct.track_id \
             WHERE ct.case_id = ?1 \
             ORDER BY ct.rowid ASC",
            TRACK_JOIN_COLS
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![case_id], map_track_row)?;
        let mut tracks = Vec::new();
        for r in rows {
            tracks.push(r?);
        }

        let mut playlist = Playlist::with_tracks(name, tracks);
        playlist.id = Some(case_id);
        Ok(Some(playlist))
    }

    /// Atomically persists or updates a `Playlist` to `cases` and `casetracks`
    pub fn save_playlist(&self, playlist: &Playlist) -> Result<i64> {
        let tx = self.conn.unchecked_transaction()?;
        let now = chrono_now_ms();

        let case_id = if let Some(id) = playlist.id {
            let exists: bool = tx.query_row(
                "SELECT 1 FROM cases WHERE case_id = ?1",
                params![id],
                |_| Ok(true),
            ).unwrap_or(false);

            if exists {
                tx.execute(
                    "UPDATE cases SET name = ?1, date_modified = ?2 WHERE case_id = ?3",
                    params![playlist.name, now, id],
                )?;
                tx.execute("DELETE FROM casetracks WHERE case_id = ?1", params![id])?;
                id
            } else {
                tx.execute(
                    "INSERT INTO cases (name, date_created, times_played, rating, date_modified) VALUES (?1, ?2, 0, 0, ?2)",
                    params![playlist.name, now],
                )?;
                tx.last_insert_rowid()
            }
        } else {
            tx.execute(
                "INSERT INTO cases (name, date_created, times_played, rating, date_modified) VALUES (?1, ?2, 0, 0, ?2)",
                params![playlist.name, now],
            )?;
            tx.last_insert_rowid()
        };

        {
            let mut insert_stmt = tx.prepare("INSERT INTO casetracks (case_id, track_id) VALUES (?1, ?2)")?;
            for track in playlist.tracks() {
                insert_stmt.execute(params![case_id, track.id])?;
            }
        }

        tx.commit()?;
        Ok(case_id)
    }

    /// Inserts a newly imported track record into Tonium Pacemaker `tracks` table
    pub fn insert_track(&self, input: &NewTrackInput) -> Result<i64> {
        let now = chrono_now_ms();
        let ind_title = input.title.to_lowercase();
        let ind_artist = input.artist.to_lowercase();
        let ind_album = input.album.as_deref().unwrap_or("").to_lowercase();
        let ind_genre = input.genre.as_deref().unwrap_or("").to_lowercase();
        let ind_bpm = input.bpm.map(|b| ((b / 10.0).floor() as i64) * 10).unwrap_or(0);
        let bpm_val = input.bpm.unwrap_or(0.0);
        let album_val = input.album.as_deref().unwrap_or("");
        let genre_val = input.genre.as_deref().unwrap_or("");
        let year_val = input.year.unwrap_or(0);
        let key_val = input.key.as_deref().unwrap_or("");
        let comments_val = input.comments.as_deref().unwrap_or("");

        self.conn.execute(
            "INSERT INTO tracks (
                title, artist, album, genre, year,
                location, format, bit_rate, sample_rate, file_size,
                play_time_secs, rating, comments, date_added, last_played,
                times_played, cue_point, rc_mixes, bpm, label,
                track_flags, loop_in, loop_out, ind_title, ind_artist,
                ind_album, ind_genre, ind_bpm, key, date_modified, modified_by_ed,
                analyzed_by_ed, analysis_ver
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, 0, ?12, ?13, -1,
                0, -1, 0, ?14, '',
                2, -1, -1, ?15, ?16,
                ?17, ?18, ?19, ?20, ?13, '2.0.2.14170',
                '2.0.2.14170', 1
            )",
            params![
                input.title,
                input.artist,
                album_val,
                genre_val,
                year_val,
                input.location,
                input.format,
                input.bit_rate,
                input.sample_rate,
                input.file_size as i64,
                input.play_time_secs,
                comments_val,
                now,
                bpm_val,
                ind_title,
                ind_artist,
                ind_album,
                ind_genre,
                ind_bpm,
                key_val,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewTrackInput {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub play_time_secs: u32,
    pub file_size: u64,
    pub bit_rate: u32,
    pub sample_rate: u32,
    pub format: String,
    pub location: String,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub comments: Option<String>,
}

fn chrono_now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_query_track() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE tracks (
                track_id INTEGER PRIMARY KEY AUTOINCREMENT,
                artist VARCHAR(255), album_artist VARCHAR(255), title VARCHAR(255), composer VARCHAR(255),
                album VARCHAR(255), track_number NUMBER(4), year NUMBER(4), genre VARCHAR(255),
                is_part_of_c NUMBER(1), location VARCHAR(255), format VARCHAR(32), bit_rate NUMBER(10),
                sample_rate NUMBER(10), file_size NUMBER(10), play_time_secs NUMBER(10), rating NUMBER(1),
                comments VARCHAR(255), date_added NUMBER(20), last_played NUMBER(20), times_played NUMBER(10),
                cue_point NUMBER(20), rc_mixes NUMBER(10), bpm REAL, label VARCHAR(255),
                track_flags NUMBER(10) DEFAULT 0, global_id VARCHAR(255), loop_in NUMBER(20), loop_out NUMBER(20),
                structured_ct VARCHAR(511), ind_title VARCHAR(32), ind_artist VARCHAR(32), ind_album VARCHAR(32),
                ind_genre VARCHAR(32), ind_bpm INTEGER, discid VARCHAR(255), producer VARCHAR(255),
                remixer VARCHAR(255), key VARCHAR(4), number_of_tracks NUMBER(4), disc_number NUMBER(4),
                number_of_discs NUMBER(4), date_modified NUMBER(20), modified_by_ed VARCHAR(32),
                analyzed_by_ed VARCHAR(32), analysis_ver INTEGER
            );
            CREATE TABLE cases (
                case_id INTEGER PRIMARY KEY AUTOINCREMENT,
                name VARCHAR(255), date_created NUMBER(20), times_played NUMBER(10),
                rating NUMBER(1), date_modified NUMBER(20)
            );
            CREATE TABLE casetracks (
                case_id INTEGER, track_id INTEGER
            );
        ").unwrap();

        let db = PacemakerDb { conn };
        let new_track = NewTrackInput {
            title: "Test YouTube Track".into(),
            artist: "Test Artist".into(),
            album: Some("Test Album".into()),
            genre: Some("Electronic".into()),
            year: Some(2026),
            play_time_secs: 180,
            file_size: 4000000,
            bit_rate: 320,
            sample_rate: 44100,
            format: "MPEG-1 Layer 3".into(),
            location: "/pmdb_tracks/a/b/12345678.mp3".into(),
            bpm: Some(128.0),
            key: Some("8A".into()),
            comments: Some("Downloaded from YouTube".into()),
        };

        let id = db.insert_track(&new_track).expect("Insert track should succeed");
        assert!(id > 0);

        let tracks = db.get_tracks(Some("YouTube"), None, None, None).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Test YouTube Track");
        assert_eq!(tracks[0].artist, "Test Artist");
        assert_eq!(tracks[0].bpm, 128.0);
    }

    #[test]
    fn test_playlist_save_and_load() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE tracks (
                track_id INTEGER PRIMARY KEY AUTOINCREMENT,
                artist VARCHAR(255), album_artist VARCHAR(255), title VARCHAR(255), composer VARCHAR(255),
                album VARCHAR(255), track_number NUMBER(4), year NUMBER(4), genre VARCHAR(255),
                is_part_of_c NUMBER(1), location VARCHAR(255), format VARCHAR(32), bit_rate NUMBER(10),
                sample_rate NUMBER(10), file_size NUMBER(10), play_time_secs NUMBER(10), rating NUMBER(1),
                comments VARCHAR(255), date_added NUMBER(20), last_played NUMBER(20), times_played NUMBER(10),
                cue_point NUMBER(20), rc_mixes NUMBER(10), bpm REAL, label VARCHAR(255),
                track_flags NUMBER(10) DEFAULT 0, global_id VARCHAR(255), loop_in NUMBER(20), loop_out NUMBER(20),
                structured_ct VARCHAR(511), ind_title VARCHAR(32), ind_artist VARCHAR(32), ind_album VARCHAR(32),
                ind_genre VARCHAR(32), ind_bpm INTEGER, discid VARCHAR(255), producer VARCHAR(255),
                remixer VARCHAR(255), key VARCHAR(4), number_of_tracks NUMBER(4), disc_number NUMBER(4),
                number_of_discs NUMBER(4), date_modified NUMBER(20), modified_by_ed VARCHAR(32),
                analyzed_by_ed VARCHAR(32), analysis_ver INTEGER
            );
            CREATE TABLE cases (
                case_id INTEGER PRIMARY KEY AUTOINCREMENT,
                name VARCHAR(255), date_created NUMBER(20), times_played NUMBER(10),
                rating NUMBER(1), date_modified NUMBER(20)
            );
            CREATE TABLE casetracks (
                case_id INTEGER, track_id INTEGER
            );
        ").unwrap();

        let db = PacemakerDb { conn };
        let track1 = NewTrackInput {
            title: "Track 1".into(),
            artist: "Artist A".into(),
            album: None,
            genre: None,
            year: None,
            play_time_secs: 120,
            file_size: 1000,
            bit_rate: 320,
            sample_rate: 44100,
            format: "MP3".into(),
            location: "/loc1.mp3".into(),
            bpm: Some(120.0),
            key: None,
            comments: None,
        };
        let track2 = NewTrackInput {
            title: "Track 2".into(),
            artist: "Artist B".into(),
            album: None,
            genre: None,
            year: None,
            play_time_secs: 150,
            file_size: 1200,
            bit_rate: 320,
            sample_rate: 44100,
            format: "MP3".into(),
            location: "/loc2.mp3".into(),
            bpm: Some(124.0),
            key: None,
            comments: None,
        };

        let id1 = db.insert_track(&track1).unwrap();
        let id2 = db.insert_track(&track2).unwrap();

        let t1 = db.get_track(id1).unwrap().unwrap();
        let t2 = db.get_track(id2).unwrap().unwrap();

        let mut pl = Playlist::new("Festival Set");
        pl.add_track(t1);
        pl.add_track(t2);

        let case_id = db.save_playlist(&pl).expect("Save playlist should succeed");
        assert!(case_id > 0);

        let loaded = db.get_playlist(case_id).unwrap().expect("Playlist should exist");
        assert_eq!(loaded.name, "Festival Set");
        assert_eq!(loaded.track_count(), 2);
        assert_eq!(loaded.tracks()[0].title, "Track 1");
        assert_eq!(loaded.tracks()[1].title, "Track 2");

        // Now update playlist
        let mut pl_updated = loaded;
        pl_updated.name = "Festival Set Remastered".to_string();
        pl_updated.remove_track_at(0);
        let updated_id = db.save_playlist(&pl_updated).expect("Update should succeed");
        assert_eq!(updated_id, case_id);

        let reloaded = db.get_playlist(case_id).unwrap().unwrap();
        assert_eq!(reloaded.name, "Festival Set Remastered");
        assert_eq!(reloaded.track_count(), 1);
        assert_eq!(reloaded.tracks()[0].title, "Track 2");
    }
}
