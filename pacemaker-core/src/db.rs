//! SQLite reader and writer for Tonium Pacemaker `music.db`.

use rusqlite::{params, Connection, Result};
use crate::types::{CaseItem, TrackId, TrackMetadata, TimeMs};

pub struct PacemakerDb {
    conn: Connection,
}

impl PacemakerDb {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        // Prevent database is locked issues on slow FAT32 storage (Issue #1)
        let _ = conn.execute_batch("
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
        ");
        Ok(Self { conn })
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
            sql.push_str(
                "SELECT t.track_id, t.title, t.artist, t.album, t.genre, t.year, \
                 t.bpm, t.play_time_secs, t.cue_point, t.loop_in, t.loop_out, \
                 t.location, t.key, t.rating, t.comments \
                 FROM tracks t \
                 JOIN casetracks ct ON t.track_id = ct.track_id \
                 WHERE ct.case_id = ?"
            );
            params.push(Box::new(cid));
        } else {
            sql.push_str(
                "SELECT track_id, title, artist, album, genre, year, \
                 bpm, play_time_secs, cue_point, loop_in, loop_out, \
                 location, key, rating, comments \
                 FROM tracks WHERE 1=1"
            );
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
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
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
        })?;

        let mut tracks = Vec::new();
        for r in rows {
            tracks.push(r?);
        }
        Ok(tracks)
    }

    pub fn get_track(&self, track_id: TrackId) -> Result<Option<TrackMetadata>> {
        let sql = "SELECT track_id, title, artist, album, genre, year, \
                   bpm, play_time_secs, cue_point, loop_in, loop_out, \
                   location, key, rating, comments \
                   FROM tracks WHERE track_id = ?1";

        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query_map(params![track_id], |row| {
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
        })?;

        match rows.next() {
            Some(res) => Ok(Some(res?)),
            None => Ok(None),
        }
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
        if let Some(t) = title {
            let ind = t.chars().take(32).collect::<String>().to_lowercase();
            self.conn.execute(
                "UPDATE tracks SET title = ?1, ind_title = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![t, ind, now, track_id],
            )?;
        }
        if let Some(a) = artist {
            let ind = a.chars().take(32).collect::<String>().to_lowercase();
            self.conn.execute(
                "UPDATE tracks SET artist = ?1, ind_artist = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![a, ind, now, track_id],
            )?;
        }
        if let Some(al) = album {
            let ind = al.chars().take(32).collect::<String>().to_lowercase();
            self.conn.execute(
                "UPDATE tracks SET album = ?1, ind_album = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![al, ind, now, track_id],
            )?;
        }
        if let Some(g) = genre {
            let ind = g.chars().take(32).collect::<String>().to_lowercase();
            self.conn.execute(
                "UPDATE tracks SET genre = ?1, ind_genre = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![g, ind, now, track_id],
            )?;
        }
        if let Some(y) = year {
            self.conn.execute(
                "UPDATE tracks SET year = ?1, date_modified = ?2 WHERE track_id = ?3",
                params![y, now, track_id],
            )?;
        }
        if let Some(b) = bpm {
            let ind_b = ((b / 10.0).floor() as i64) * 10;
            self.conn.execute(
                "UPDATE tracks SET bpm = ?1, ind_bpm = ?2, date_modified = ?3 WHERE track_id = ?4",
                params![b, ind_b, now, track_id],
            )?;
        }
        if let Some(k) = key {
            self.conn.execute(
                "UPDATE tracks SET key = ?1, date_modified = ?2 WHERE track_id = ?3",
                params![k, now, track_id],
            )?;
        }
        if let Some(c) = comments {
            self.conn.execute(
                "UPDATE tracks SET comments = ?1, date_modified = ?2 WHERE track_id = ?3",
                params![c, now, track_id],
            )?;
        }
        Ok(())
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
}

fn chrono_now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
