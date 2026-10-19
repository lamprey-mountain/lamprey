//! frontend specific preferences

use std::collections::HashMap;

use lamprey_macros::record;

use super::room_sidebar;
use crate::util::registry::export_models;

/// how messages are displayed in the chat
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageStyle {
    /// show avatars and group consecutive messages
    #[default]
    Cozy,

    /// use a more dense layout
    Compact,
}

/// how spoilers are revealed
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Spoilers {
    /// reveal on click
    #[default]
    Click,

    /// reveal on hover
    Hover,

    /// always reveal
    Always,
}

/// the color theme to use
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    /// follow the system theme
    #[default]
    Auto,

    /// follow the system theme, with high contrast
    AutoHighcontrast,

    /// light theme
    Light,

    /// dark theme
    Dark,

    /// light theme, with high contrast
    LightHighcontrast,

    /// dark theme, with high contrast
    DarkHighcontrast,
}

/// how dates are formatted
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DateFormat {
    /// follow the system locale
    #[default]
    Auto,

    /// dd/mm/yyyy
    Standard,

    /// mm/dd/yyyy
    American,

    /// yyyy-mm-dd
    Iso,
}

/// how times are formatted
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
pub enum TimeFormat {
    /// follow the system locale
    #[default]
    #[serde(rename = "auto")]
    Auto,

    #[serde(rename = "12h")]
    TwelveHour,

    #[serde(rename = "24h")]
    TwentyFourHour,
}

/// how voice input is activated
#[record]
#[derive(Default, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VoiceInputMode {
    /// activate on voice activity
    #[default]
    Vad,

    /// activate while a key is held
    Ptt,

    /// always active
    Open,
}

/// config specific to the frontend
#[record]
#[serde(default)]
pub struct PreferencesGlobalFrontend {
    /// room navigation sidebar
    pub room_sidebar: room_sidebar::Sidebar,

    // appearance
    /// underline links in messages
    pub underline_links: bool,

    /// show an explicit send button in the message composer
    pub show_send_button: bool,

    /// how messages are displayed
    pub message_style: MessageStyle,

    /// spacing between message groups, in pixels
    pub message_spacing: f64,

    /// chat font scale, as a percentage
    pub chat_font_scale: f64,

    /// overall application scale, as a percentage
    pub app_scale: f64,

    /// color saturation, as a percentage
    pub saturation: f64,

    /// reduce motion in the ui
    pub reduced_motion: bool,

    /// sync reduced motion with the system setting
    pub reduced_motion_sync: bool,

    /// automatically play animated emoji
    pub autoplay_emoji: bool,

    /// automatically play gifs
    pub autoplay_gifs: bool,

    /// the color theme to use
    pub theme: Theme,

    // chat
    /// preview attachments inline
    pub preview_attachments: bool,

    /// show attachment descriptions
    pub preview_attachments_descriptions: bool,

    /// show link previews
    pub link_previews: bool,

    /// show typing indicators
    pub typing_indicators: bool,

    /// how spoilers are revealed
    pub spoilers: Spoilers,

    /// open text threads in the sidebar
    pub threads_sidebar_text: bool,

    /// open document threads in the sidebar
    pub threads_sidebar_document: bool,

    /// open forum threads in the sidebar
    pub threads_sidebar_forum: bool,

    // language
    /// how dates are formatted
    pub date_format: DateFormat,

    /// how times are formatted
    pub time_format: TimeFormat,

    // notifications
    /// show desktop notifications
    pub desktop_notifs: bool,

    /// subscribe to web push notifications
    pub push_notifs: bool,

    /// read notifications aloud with text to speech
    pub tts_notifs: bool,

    // voice
    /// the input device id, or the system default if unset
    pub input_device: Option<String>,

    /// the output device id, or the system default if unset
    pub output_device: Option<String>,

    /// the video device id, or the system default if unset
    pub video_device: Option<String>,

    /// microphone volume, as a percentage
    pub mic_volume: f64,

    /// speaker volume, as a percentage
    pub speaker_volume: f64,

    /// enable echo cancellation
    pub voice_echo_cancellation: bool,

    /// enable noise suppression
    pub voice_noise_suppression: bool,

    /// how voice input is activated
    pub voice_input_mode: VoiceInputMode,

    /// voice activity detection threshold, as a percentage
    pub voice_activity_threshold: f64,

    /// how long of silence before voice activity deactivates, in milliseconds
    pub voice_activity_timeout: f64,

    // calendar
    /// the day of the week the calendar starts on (0 = sunday)
    pub calendar_day_starts_at: u8,

    /// custom and unicode emoji reaction keys marked as favorites
    pub favorite_emoji: Vec<String>,

    /// extra implementation defined config (TEMPORARY?)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[record]
#[derive(Default)]
pub struct PreferencesRoomFrontend {
    /// extra implementation defined config (TEMPORARY?)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// config specific to the frontend for a channel
#[record]
#[serde(default)]
pub struct PreferencesChannelFrontend {
    /// display voice streams in a grid
    pub grid_view: bool,

    /// show members who are not sharing video
    pub show_people_without_video: bool,

    /// extra implementation defined config (TEMPORARY?)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// config specific to the frontend for another user
#[record]
#[serde(default)]
pub struct PreferencesUserFrontend {
    /// a private note about the user, only visible to the local user
    pub note: Option<String>,

    /// whether the user is locally muted
    pub user_mute: bool,

    /// local volume for the user, defaults to 1 (100%)
    pub user_volume: f64,

    /// whether the user's video is locally hidden
    pub user_video: bool,

    /// whether the user's streams are locally muted
    pub stream_mute: bool,

    /// local volume for the user's streams, defaults to 1 (100%)
    pub stream_volume: f64,

    /// extra implementation defined config (TEMPORARY?)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for PreferencesUserFrontend {
    fn default() -> Self {
        Self {
            note: None,
            user_mute: false,
            user_volume: 1.0,
            user_video: false,
            stream_mute: false,
            stream_volume: 1.0,
            extra: HashMap::new(),
        }
    }
}

impl Default for PreferencesGlobalFrontend {
    fn default() -> Self {
        Self {
            room_sidebar: Default::default(),

            underline_links: false,
            show_send_button: false,
            message_style: MessageStyle::Cozy,
            message_spacing: 8.0,
            chat_font_scale: 100.0,
            app_scale: 100.0,
            saturation: 100.0,
            reduced_motion: false,
            reduced_motion_sync: true,
            autoplay_emoji: true,
            autoplay_gifs: true,
            theme: Theme::Auto,

            preview_attachments: true,
            preview_attachments_descriptions: false,
            link_previews: true,
            typing_indicators: true,
            spoilers: Spoilers::Click,
            threads_sidebar_text: true,
            threads_sidebar_document: true,
            threads_sidebar_forum: true,

            date_format: DateFormat::Auto,
            time_format: TimeFormat::Auto,

            desktop_notifs: false,
            push_notifs: false,
            tts_notifs: false,

            input_device: None,
            output_device: None,
            video_device: None,
            mic_volume: 50.0,
            speaker_volume: 75.0,
            voice_echo_cancellation: false,
            voice_noise_suppression: false,
            voice_input_mode: VoiceInputMode::Vad,
            voice_activity_threshold: 30.0,
            voice_activity_timeout: 1000.0,

            calendar_day_starts_at: 0,

            favorite_emoji: Vec::new(),

            extra: HashMap::new(),
        }
    }
}

impl Default for PreferencesChannelFrontend {
    fn default() -> Self {
        Self {
            grid_view: false,
            show_people_without_video: true,
            extra: HashMap::new(),
        }
    }
}

export_models!(
    MessageStyle,
    Spoilers,
    Theme,
    DateFormat,
    TimeFormat,
    VoiceInputMode,
    PreferencesGlobalFrontend,
    PreferencesRoomFrontend,
    PreferencesChannelFrontend,
    PreferencesUserFrontend,
);
