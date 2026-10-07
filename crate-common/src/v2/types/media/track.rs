// WORK IN PROGRESS PLANNING

use lamprey_macros::record;
use uuid::Uuid;

// new api endpoints
// POST /media/{media_id}/track -- media_track_create -> 201 created
// GET /media/{media_id}/track -- media_track_list -> 200 ok
// PATCH /media/{media_id}/track/{target} -- media_track_update -> 200 ok
// DELETE /media/{media_id}/track/{target} -- media_track_delete -> 204 no content
// GET /media/{media_id}/track/{target} -- media_track_get -> 200 ok

// new cdn endpoint
// GET /media/{media_id}/track/{id} -- get track contents
// existing thumbnail endpoint is kept for backwards compat

// add field
// pub struct Media {
//     pub tracks: Vec<Track>,
// }

#[record]
pub struct TrackCreate {
    // option to generate, upload, etc
    #[serde(rename = "type")]
    pub ty: TrackType,
    pub lang: Option<String>,
    pub label: Option<String>,
}

#[record]
pub struct TrackUpdate {
    /// the type of a track
    ///
    /// only valid transition is subtitles to/from captions
    #[serde(rename = "type")]
    pub ty: Option<TrackType>,
    pub lang: Option<Option<String>>,
    pub label: Option<Option<String>>,
}

#[record]
pub struct Track {
    pub id: Uuid, // TODO: create dedicated MediaTrackId
    pub source: TrackSource,

    #[serde(rename = "type")]
    pub ty: TrackType,

    /// the language of a track
    pub lang: Option<String>,

    /// a human readable name for a track
    pub label: Option<String>,

    /// file size in bytes
    pub size: u64,
}

/// where a track came from
#[record]
#[serde(tag = "source")]
pub enum TrackSource {
    /// embedded in the source file
    Embedded,

    /// manually uploaded
    Uploaded,

    // /// another piece of media attached to this one
    // Attached { media_id: MediaId },
    /// generated automatically
    Generated,
    // /// attempt to generate a sidecar for this automatically
    // ///
    // /// only some [`SidecarKind`]s can be generated automatically
    // // - transcriptions are auto translated if lang is Some(valid bcp code)
    // // - should i cache captions once and translate them or should i regenerate them with whisper?
    // Generate {
    //     generator_name: String,
    //     generator_version: String,
    // },
}

/// the type of a track
#[record]
pub enum TrackType {
    /// transcription of spoken text; lyrics
    ///
    /// formatted as webvtt
    Subtitles,

    /// transcription incluing relevant sound effects
    ///
    /// formatted as webvtt
    Captions,
    // TODO: thumbnail, chapters, audio
}

// message sync media track create/update/delete

#[record]
enum Error {
    // cannot generate track
    // default thumbnail cannot be deleted
    // embedded track cannot be deleted
    // thumbnails must be in a known image format
    // captions must be encoded in webvtt
    // subtitles must be encoded in webvtt
}
