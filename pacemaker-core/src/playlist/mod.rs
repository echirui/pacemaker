//! Playlist management and Auto-DJ continuous playback engine.
//!
//! Provides:
//! - `Playlist`: Ordered, dynamic track queue with repeat modes and shuffle support.
//! - `AutoDjEngine`: Seamless dual-deck (Deck A / Deck B) continuous playback controller
//!   with configurable crossfading, BPM beat-matching, and pre-cueing.
//! - M3U/M3U8 import & export serialization.

use crate::types::{
    DeckId, PlaybackOrder, PlaybackState, PlaylistError, PlaylistId, RepeatMode,
    TrackId, TrackMetadata, TransitionType,
};
use crate::engine::PacemakerMixer;

#[derive(Debug, Clone, PartialEq)]
pub struct Playlist {
    pub id: Option<PlaylistId>,
    pub name: String,
    tracks: Vec<TrackMetadata>,
    current_index: Option<usize>,
    repeat_mode: RepeatMode,
    playback_order: PlaybackOrder,
    shuffle_indices: Vec<usize>,
    shuffle_pointer: usize,
}

impl Playlist {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            tracks: Vec::new(),
            current_index: None,
            repeat_mode: RepeatMode::Off,
            playback_order: PlaybackOrder::Sequential,
            shuffle_indices: Vec::new(),
            shuffle_pointer: 0,
        }
    }

    pub fn with_tracks(name: impl Into<String>, tracks: Vec<TrackMetadata>) -> Self {
        let mut pl = Self::new(name);
        pl.tracks = tracks;
        if !pl.tracks.is_empty() {
            pl.current_index = Some(0);
            pl.rebuild_shuffle_indices();
        }
        pl
    }

    #[inline]
    pub fn tracks(&self) -> &[TrackMetadata] {
        &self.tracks
    }

    #[inline]
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn total_duration_secs(&self) -> u32 {
        self.tracks.iter().map(|t| t.play_time_secs).sum()
    }

    #[inline]
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn current_track(&self) -> Option<&TrackMetadata> {
        self.current_index.and_then(|idx| self.tracks.get(idx))
    }

    pub fn add_track(&mut self, track: TrackMetadata) {
        self.tracks.push(track);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
        self.rebuild_shuffle_indices();
    }

    pub fn add_tracks(&mut self, tracks: Vec<TrackMetadata>) {
        if tracks.is_empty() {
            return;
        }
        self.tracks.extend(tracks);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
        self.rebuild_shuffle_indices();
    }

    pub fn insert_track(&mut self, index: usize, track: TrackMetadata) -> Result<(), PlaylistError> {
        if index > self.tracks.len() {
            return Err(PlaylistError::IndexOutOfBounds(index));
        }
        self.tracks.insert(index, track);
        if let Some(curr) = self.current_index {
            if index <= curr {
                self.current_index = Some(curr + 1);
            }
        } else {
            self.current_index = Some(0);
        }
        self.rebuild_shuffle_indices();
        Ok(())
    }

    pub fn remove_track_at(&mut self, index: usize) -> Option<TrackMetadata> {
        if index >= self.tracks.len() {
            return None;
        }
        let removed = self.tracks.remove(index);
        if self.tracks.is_empty() {
            self.current_index = None;
        } else if let Some(curr) = self.current_index {
            if index < curr {
                self.current_index = Some(curr - 1);
            } else if curr >= self.tracks.len() {
                self.current_index = Some(self.tracks.len() - 1);
            }
        }
        self.rebuild_shuffle_indices();
        Some(removed)
    }

    pub fn remove_track_by_id(&mut self, track_id: TrackId) -> bool {
        if let Some(pos) = self.tracks.iter().position(|t| t.id == track_id) {
            self.remove_track_at(pos);
            true
        } else {
            false
        }
    }

    pub fn move_track(&mut self, from_index: usize, to_index: usize) -> Result<(), PlaylistError> {
        let len = self.tracks.len();
        if from_index >= len {
            return Err(PlaylistError::IndexOutOfBounds(from_index));
        }
        if to_index >= len {
            return Err(PlaylistError::IndexOutOfBounds(to_index));
        }
        if from_index == to_index {
            return Ok(());
        }

        let item = self.tracks.remove(from_index);
        self.tracks.insert(to_index, item);

        // Adjust current_index if affected
        if let Some(curr) = self.current_index {
            if curr == from_index {
                self.current_index = Some(to_index);
            } else if from_index < curr && to_index >= curr {
                self.current_index = Some(curr - 1);
            } else if from_index > curr && to_index <= curr {
                self.current_index = Some(curr + 1);
            }
        }

        self.rebuild_shuffle_indices();
        Ok(())
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = None;
        self.shuffle_indices.clear();
        self.shuffle_pointer = 0;
    }

    pub fn seek_to(&mut self, index: usize) -> Result<&TrackMetadata, PlaylistError> {
        if index >= self.tracks.len() {
            return Err(PlaylistError::IndexOutOfBounds(index));
        }
        self.current_index = Some(index);
        if self.playback_order == PlaybackOrder::Shuffle {
            if let Some(pos) = self.shuffle_indices.iter().position(|&i| i == index) {
                self.shuffle_pointer = pos;
            }
        }
        Ok(&self.tracks[index])
    }

    pub fn select_track_by_id(&mut self, track_id: TrackId) -> Option<&TrackMetadata> {
        let pos = self.tracks.iter().position(|t| t.id == track_id)?;
        self.seek_to(pos).ok()
    }

    pub fn set_repeat_mode(&mut self, mode: RepeatMode) {
        self.repeat_mode = mode;
    }

    #[inline]
    pub fn repeat_mode(&self) -> RepeatMode {
        self.repeat_mode
    }

    pub fn set_playback_order(&mut self, order: PlaybackOrder) {
        if self.playback_order != order {
            self.playback_order = order;
            if order == PlaybackOrder::Shuffle {
                self.rebuild_shuffle_indices();
                if let Some(curr) = self.current_index {
                    if let Some(pos) = self.shuffle_indices.iter().position(|&i| i == curr) {
                        self.shuffle_pointer = pos;
                    }
                }
            }
        }
    }

    #[inline]
    pub fn playback_order(&self) -> PlaybackOrder {
        self.playback_order
    }

    /// Advance to the next track in the playlist according to current playback order and repeat mode
    pub fn next_track(&mut self) -> Option<&TrackMetadata> {
        if self.tracks.is_empty() {
            return None;
        }

        if self.repeat_mode == RepeatMode::One {
            return self.current_track();
        }

        match self.playback_order {
            PlaybackOrder::Sequential => {
                let next_idx = match self.current_index {
                    Some(curr) => {
                        if curr + 1 < self.tracks.len() {
                            curr + 1
                        } else if self.repeat_mode == RepeatMode::All {
                            0
                        } else {
                            return None;
                        }
                    }
                    None => 0,
                };
                self.current_index = Some(next_idx);
                Some(&self.tracks[next_idx])
            }
            PlaybackOrder::Shuffle => {
                if self.shuffle_indices.is_empty() {
                    self.rebuild_shuffle_indices();
                }

                if self.shuffle_pointer + 1 < self.shuffle_indices.len() {
                    self.shuffle_pointer += 1;
                } else if self.repeat_mode == RepeatMode::All {
                    self.shuffle_indices_permutation();
                    self.shuffle_pointer = 0;
                } else {
                    return None;
                }

                let target_idx = self.shuffle_indices[self.shuffle_pointer];
                self.current_index = Some(target_idx);
                Some(&self.tracks[target_idx])
            }
        }
    }

    /// Move to the previous track in the playlist
    pub fn previous_track(&mut self) -> Option<&TrackMetadata> {
        if self.tracks.is_empty() {
            return None;
        }

        if self.repeat_mode == RepeatMode::One {
            return self.current_track();
        }

        match self.playback_order {
            PlaybackOrder::Sequential => {
                let prev_idx = match self.current_index {
                    Some(curr) if curr > 0 => curr - 1,
                    Some(_) if self.repeat_mode == RepeatMode::All => self.tracks.len().saturating_sub(1),
                    Some(_) => 0,
                    None => 0,
                };
                self.current_index = Some(prev_idx);
                Some(&self.tracks[prev_idx])
            }
            PlaybackOrder::Shuffle => {
                if self.shuffle_pointer > 0 {
                    self.shuffle_pointer -= 1;
                } else if self.repeat_mode == RepeatMode::All && !self.shuffle_indices.is_empty() {
                    self.shuffle_pointer = self.shuffle_indices.len() - 1;
                }
                let target_idx = self.shuffle_indices[self.shuffle_pointer];
                self.current_index = Some(target_idx);
                Some(&self.tracks[target_idx])
            }
        }
    }

    /// Inspect the track that would play next without modifying current playhead
    pub fn peek_next_track(&self) -> Option<&TrackMetadata> {
        if self.tracks.is_empty() {
            return None;
        }

        if self.repeat_mode == RepeatMode::One {
            return self.current_track();
        }

        match self.playback_order {
            PlaybackOrder::Sequential => match self.current_index {
                Some(curr) => {
                    if curr + 1 < self.tracks.len() {
                        Some(&self.tracks[curr + 1])
                    } else if self.repeat_mode == RepeatMode::All {
                        Some(&self.tracks[0])
                    } else {
                        None
                    }
                }
                None => Some(&self.tracks[0]),
            },
            PlaybackOrder::Shuffle => {
                if self.shuffle_pointer + 1 < self.shuffle_indices.len() {
                    let next_idx = self.shuffle_indices[self.shuffle_pointer + 1];
                    Some(&self.tracks[next_idx])
                } else if self.repeat_mode == RepeatMode::All && !self.shuffle_indices.is_empty() {
                    let next_idx = self.shuffle_indices[0];
                    Some(&self.tracks[next_idx])
                } else {
                    None
                }
            }
        }
    }

    /// Exports playlist in standard M3U/M3U8 extended format
    pub fn to_m3u(&self) -> String {
        let mut out = String::from("#EXTM3U\n");
        for t in &self.tracks {
            out.push_str(&format!(
                "#EXTINF:{},{} - {}\n{}\n",
                t.play_time_secs,
                t.artist.replace(',', ""),
                t.title.replace(',', ""),
                t.virtual_location
            ));
        }
        out
    }

    /// Parses an M3U format string into a Playlist
    pub fn from_m3u(name: &str, content: &str) -> Result<Self, PlaylistError> {
        let mut tracks = Vec::new();
        let lines = content.lines();

        let mut current_title: Option<String> = None;
        let mut current_artist: Option<String> = None;
        let mut current_secs: u32 = 0;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("#EXTM3U") {
                continue;
            }

            if let Some(extinf) = trimmed.strip_prefix("#EXTINF:") {
                let parts: Vec<&str> = extinf.splitn(2, ',').collect();
                if let Ok(secs) = parts[0].trim().parse::<i32>() {
                    current_secs = if secs > 0 { secs as u32 } else { 0 };
                }
                if parts.len() > 1 {
                    let artist_title: Vec<&str> = parts[1].splitn(2, " - ").collect();
                    if artist_title.len() == 2 {
                        current_artist = Some(artist_title[0].trim().to_string());
                        current_title = Some(artist_title[1].trim().to_string());
                    } else {
                        current_title = Some(parts[1].trim().to_string());
                    }
                }
            } else if !trimmed.starts_with('#') {
                // File location
                let title = current_title.take().unwrap_or_else(|| {
                    std::path::Path::new(trimmed)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown Title")
                        .to_string()
                });
                let artist = current_artist.take().unwrap_or_else(|| "Unknown Artist".to_string());

                let track = TrackMetadata {
                    id: (tracks.len() + 1) as i64,
                    title,
                    artist,
                    play_time_secs: current_secs,
                    virtual_location: trimmed.to_string(),
                    ..Default::default()
                };

                tracks.push(track);
                current_secs = 0;
            }
        }

        Ok(Self::with_tracks(name, tracks))
    }

    fn rebuild_shuffle_indices(&mut self) {
        self.shuffle_indices = (0..self.tracks.len()).collect();
        self.shuffle_indices_permutation();
        self.shuffle_pointer = 0;
    }

    /// Low-memory embedded permutation (Fisher-Yates with deterministic Linear Congruential Generator)
    fn shuffle_indices_permutation(&mut self) {
        let n = self.shuffle_indices.len();
        if n <= 1 {
            return;
        }

        // LCG seed using track count, length, and sum of IDs
        let mut seed: u64 = 0x5deece66d ^ (n as u64).wrapping_mul(0xdeadbeef);
        for i in (1..n).rev() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (seed % (i as u64 + 1)) as usize;
            self.shuffle_indices.swap(i, j);
        }
    }
}

/// Auto-DJ dual-deck playback state controller.
///
/// Seamlessly plays consecutive tracks from a `Playlist` across `DeckA` and `DeckB`
/// in `PacemakerMixer`, automating smooth crossfading transitions and beat sync.
pub struct AutoDjEngine {
    pub playlist: Playlist,
    pub transition_type: TransitionType,
    pub active_deck: DeckId,
    pub is_transitioning: bool,
    pub transition_progress: f32, // 0.0 to 1.0
    transition_duration_secs: f32,
}

impl AutoDjEngine {
    pub fn new(playlist: Playlist, transition_type: TransitionType) -> Self {
        let duration = match transition_type {
            TransitionType::Cut => 0.0,
            TransitionType::Crossfade { duration_secs } => duration_secs.max(0.5),
            TransitionType::BeatMatch { duration_secs } => duration_secs.max(1.0),
        };

        Self {
            playlist,
            transition_type,
            active_deck: DeckId::DeckA,
            is_transitioning: false,
            transition_progress: 0.0,
            transition_duration_secs: duration,
        }
    }

    /// Starts playing the playlist on Deck A and pre-cues next track on Deck B
    pub fn start(&mut self, mixer: &mut PacemakerMixer) -> Result<(), PlaylistError> {
        let track = self.playlist.current_track().ok_or(PlaylistError::EmptyPlaylist)?;
        self.active_deck = DeckId::DeckA;
        self.is_transitioning = false;
        self.transition_progress = 0.0;

        // Position crossfader all the way to Deck A (-1.0)
        mixer.crossfader.position = -1.0;

        // Load and play Deck A
        let d_a = mixer.deck_mut(DeckId::DeckA);
        d_a.load_track(track.clone());
        d_a.play();

        // Pre-cue next track on Deck B if available
        if let Some(next_t) = self.playlist.peek_next_track() {
            let d_b = mixer.deck_mut(DeckId::DeckB);
            d_b.load_track(next_t.clone());
            d_b.pause();
        }

        Ok(())
    }

    /// Progresses auto-DJ engine state during playback
    pub fn update(&mut self, delta_secs: f32, mixer: &mut PacemakerMixer) {
        if self.playlist.is_empty() {
            return;
        }

        let active_id = self.active_deck;
        let opposite_id = active_id.opposite();

        // Check active deck status
        let (current_ms, total_secs, active_bpm) = {
            let deck = mixer.deck(active_id);
            if deck.state != PlaybackState::Playing && !self.is_transitioning {
                return;
            }
            let max_secs = deck.current_track.as_ref().map(|t| t.play_time_secs).unwrap_or(0);
            let bpm = deck.current_track.as_ref().map(|t| t.bpm).unwrap_or(120.0);
            (deck.current_time_ms, max_secs, bpm)
        };

        let total_ms = (total_secs as i64) * 1000;
        let remaining_secs = ((total_ms - current_ms).max(0) as f32) / 1000.0;

        // Check if we should trigger transition
        if !self.is_transitioning && remaining_secs <= self.transition_duration_secs && remaining_secs > 0.05 {
            // Begin transition to opposite deck
            if let Some(next_t) = self.playlist.peek_next_track() {
                let next_clone = next_t.clone();
                let opp_bpm = next_clone.bpm;
                let d_opp = mixer.deck_mut(opposite_id);

                // Ensure next track is loaded
                if d_opp.current_track.as_ref().map(|t| t.id) != Some(next_clone.id) {
                    d_opp.load_track(next_clone);
                }

                // If BeatMatch, sync BPM of incoming track to current active track
                if matches!(self.transition_type, TransitionType::BeatMatch { .. })
                    && opp_bpm > 30.0 && active_bpm > 30.0
                {
                    d_opp.set_speed((active_bpm / opp_bpm) as f32);
                }

                d_opp.play();
                self.is_transitioning = true;
                self.transition_progress = 0.0;
            }
        }

        // Animate crossfader during transition
        if self.is_transitioning {
            if self.transition_duration_secs <= 0.05 {
                self.transition_progress = 1.0;
            } else {
                self.transition_progress += delta_secs / self.transition_duration_secs;
            }

            let t = self.transition_progress.clamp(0.0, 1.0);

            // Interpolate crossfader position:
            // If active_deck was DeckA (-1.0), move towards DeckB (+1.0)
            // If active_deck was DeckB (+1.0), move towards DeckA (-1.0)
            let start_pos = if active_id == DeckId::DeckA { -1.0 } else { 1.0 };
            let target_pos = if active_id == DeckId::DeckA { 1.0 } else { -1.0 };
            mixer.crossfader.position = start_pos + (target_pos - start_pos) * t;

            if self.transition_progress >= 1.0 {
                // Transition complete!
                self.is_transitioning = false;
                mixer.deck_mut(active_id).stop();
                self.active_deck = opposite_id;
                mixer.crossfader.position = target_pos;

                // Advance playlist pointer
                self.playlist.next_track();

                // Pre-cue subsequent track on now-free deck
                if let Some(subsequent_t) = self.playlist.peek_next_track() {
                    let d_free = mixer.deck_mut(active_id);
                    d_free.load_track(subsequent_t.clone());
                    d_free.pause();
                }
            }
        }
    }

    /// Skips immediately to the next track in the playlist
    pub fn skip_next(&mut self, mixer: &mut PacemakerMixer) -> Result<(), PlaylistError> {
        let next_track = self.playlist.next_track().ok_or(PlaylistError::EmptyPlaylist)?.clone();
        let target_deck = self.active_deck.opposite();

        mixer.deck_mut(self.active_deck).stop();
        let target = mixer.deck_mut(target_deck);
        target.load_track(next_track);
        target.play();

        self.active_deck = target_deck;
        mixer.crossfader.position = if target_deck == DeckId::DeckA { -1.0 } else { 1.0 };
        self.is_transitioning = false;
        self.transition_progress = 0.0;

        // Pre-cue following track
        if let Some(peek) = self.playlist.peek_next_track() {
            let free_deck = target_deck.opposite();
            let free = mixer.deck_mut(free_deck);
            free.load_track(peek.clone());
            free.pause();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_track(id: i64, title: &str, artist: &str, secs: u32, bpm: f64) -> TrackMetadata {
        TrackMetadata {
            id,
            title: title.into(),
            artist: artist.into(),
            album: "Album".into(),
            genre: "EDM".into(),
            year: 2026,
            bpm,
            play_time_secs: secs,
            cue_point_ms: 0,
            loop_in_ms: -1,
            loop_out_ms: -1,
            virtual_location: format!("/music/{}.mp3", id),
            key: "8A".into(),
            rating: 5,
            comments: String::new(),
        }
    }

    #[test]
    fn test_playlist_basic_operations() {
        let mut pl = Playlist::new("Test Playlist");
        assert!(pl.is_empty());
        assert_eq!(pl.track_count(), 0);

        let t1 = sample_track(1, "Track 1", "Artist A", 180, 128.0);
        let t2 = sample_track(2, "Track 2", "Artist B", 210, 130.0);
        let t3 = sample_track(3, "Track 3", "Artist C", 240, 126.0);

        pl.add_track(t1.clone());
        pl.add_track(t2.clone());
        pl.add_track(t3.clone());

        assert_eq!(pl.track_count(), 3);
        assert_eq!(pl.total_duration_secs(), 180 + 210 + 240);
        assert_eq!(pl.current_index(), Some(0));
        assert_eq!(pl.current_track().unwrap().title, "Track 1");

        // Next
        let next = pl.next_track().unwrap();
        assert_eq!(next.title, "Track 2");
        assert_eq!(pl.current_index(), Some(1));

        // Previous
        let prev = pl.previous_track().unwrap();
        assert_eq!(prev.title, "Track 1");

        // Move track 0 to index 2
        pl.move_track(0, 2).unwrap();
        assert_eq!(pl.tracks()[0].title, "Track 2");
        assert_eq!(pl.tracks()[2].title, "Track 1");

        // Remove track
        let removed = pl.remove_track_at(0).unwrap();
        assert_eq!(removed.title, "Track 2");
        assert_eq!(pl.track_count(), 2);
    }

    #[test]
    fn test_playlist_repeat_and_shuffle() {
        let mut pl = Playlist::with_tracks(
            "Shuffle Mix",
            vec![
                sample_track(1, "T1", "A", 100, 120.0),
                sample_track(2, "T2", "A", 100, 120.0),
                sample_track(3, "T3", "A", 100, 120.0),
            ],
        );

        // Repeat One
        pl.set_repeat_mode(RepeatMode::One);
        assert_eq!(pl.next_track().unwrap().title, "T1");
        assert_eq!(pl.next_track().unwrap().title, "T1");

        // Repeat All Sequential
        pl.set_repeat_mode(RepeatMode::All);
        assert_eq!(pl.next_track().unwrap().title, "T2");
        assert_eq!(pl.next_track().unwrap().title, "T3");
        // Wraps around
        assert_eq!(pl.next_track().unwrap().title, "T1");

        // Shuffle
        pl.set_playback_order(PlaybackOrder::Shuffle);
        assert_eq!(pl.shuffle_indices.len(), 3);
        let s1 = pl.next_track();
        assert!(s1.is_some());
    }

    #[test]
    fn test_playlist_m3u_roundtrip() {
        let t1 = sample_track(1, "One More Time", "Daft Punk", 320, 123.0);
        let t2 = sample_track(2, "Harder Better", "Daft Punk", 224, 123.0);
        let pl = Playlist::with_tracks("Daft Punk Set", vec![t1, t2]);

        let m3u_str = pl.to_m3u();
        assert!(m3u_str.contains("#EXTM3U"));
        assert!(m3u_str.contains("Daft Punk - One More Time"));

        let imported = Playlist::from_m3u("Imported Set", &m3u_str).unwrap();
        assert_eq!(imported.track_count(), 2);
        assert_eq!(imported.tracks()[0].title, "One More Time");
        assert_eq!(imported.tracks()[0].artist, "Daft Punk");
        assert_eq!(imported.tracks()[0].play_time_secs, 320);
    }

    #[test]
    fn test_auto_dj_dual_deck_transition() {
        let mut mixer = PacemakerMixer::new(44100.0);
        let tracks = vec![
            sample_track(1, "Track A", "DJ 1", 10, 128.0),
            sample_track(2, "Track B", "DJ 2", 10, 128.0),
        ];

        let pl = Playlist::with_tracks("Mix", tracks);
        let mut auto_dj = AutoDjEngine::new(
            pl,
            TransitionType::Crossfade { duration_secs: 2.0 },
        );

        auto_dj.start(&mut mixer).unwrap();
        assert_eq!(auto_dj.active_deck, DeckId::DeckA);
        assert_eq!(mixer.deck(DeckId::DeckA).state, PlaybackState::Playing);
        assert_eq!(mixer.crossfader.position, -1.0);

        // Advance to 8.5 seconds (remaining time: 1.5s <= 2.0s duration)
        // Deck A sample advance: 8.5s * 44100
        mixer.deck_mut(DeckId::DeckA).advance_samples(8 * 44100, 44100.0);
        auto_dj.update(0.1, &mut mixer);

        // Transition should have triggered: Deck B started playing
        assert!(auto_dj.is_transitioning);
        assert_eq!(mixer.deck(DeckId::DeckB).state, PlaybackState::Playing);

        // Progress crossfade to completion (simulate 2 seconds pass)
        auto_dj.update(2.0, &mut mixer);
        assert!(!auto_dj.is_transitioning);
        assert_eq!(auto_dj.active_deck, DeckId::DeckB);
        assert_eq!(mixer.crossfader.position, 1.0);
        assert_eq!(mixer.deck(DeckId::DeckA).state, PlaybackState::Stopped);
    }
}
