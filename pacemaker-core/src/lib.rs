//! # Pacemaker Core (pacemaker-core)
//! 
//! Clean-room Rust implementation of the Tonium Pacemaker firmware audio engine,
//! firmware uploader/inspector, and library database management.

pub mod types;
pub mod hasher;
pub mod waveform;
pub mod db;
pub mod dsp;
pub mod engine;
pub mod firmware;
pub mod metadata;
pub mod hardware;

pub use types::{DeckId, PlaybackState, TimeMs, TrackId, TrackMetadata};
pub use hasher::{generate_track_paths, jenkins_hash, pjw_hash, TrackPaths};
pub use waveform::TrackAnalysedData;
pub use db::PacemakerDb;
pub use dsp::{AudioPeakLimiter, AudioProcessBuffer, CompoundCrossfader, CrossfaderCurve, ParametricEq, SincInterpolator};
pub use engine::{AudioCoreEvent, AudioLoop, BeatJump, Deck, PacemakerMixer};
pub use firmware::{PfwInfo, detect_pacemaker_device};
pub use metadata::{read_id3_safely, ParsedMetadata};
pub use hardware::{HardwareController, HardwareVerificationReport, PhysicalButton, CsiaMode, verify_hardware_controls};
