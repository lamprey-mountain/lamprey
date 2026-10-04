use lamprey_macros::record;
use url::Url;

use crate::{util::registry::export_models, v1::types::Embed};

pub mod log {
    use lamprey_macros::record;
    use url::Url;

    use crate::{util::registry::export_models, v1::types::metadata::Metadata};

    pub use crate::v1::types::redex::EvalLogLevel as Level;

    /// an unfurler log/span entry
    #[record]
    pub struct Entry {
        /// unique identifier for this entry
        pub id: u64,

        /// when this entry started
        ///
        /// timestamp in milliseconds since the unfurl started
        pub started: u64,

        /// when this entry ended
        ///
        /// timestamp in milliseconds since the unfurl started
        pub ended: u64,

        /// the log level
        pub level: Level,

        /// child spans
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub children: Vec<Entry>,

        #[serde(flatten)]
        pub kind: EntryKind,
    }

    /// structured data for an unfurler log entry
    #[record]
    pub enum EntryKind {
        /// http request
        Http {
            url: Url,
            status: u16,
            headers: Vec<(String, String)>,
            body_preview: Option<String>,
            // TODO(?): include both request and response headers
        },

        /// generic log entry
        Other {
            /// a human readable message
            message: String,

            /// arbitrary metadata associated with this log line
            #[serde(default, skip_serializing_if = "Metadata::is_empty")]
            attributes: Metadata,
        },
    }

    export_models!(Entry, EntryKind);
}

#[record]
pub struct UnfurlerDebugRequest {
    /// the url to try to unfurl
    pub url: Url,
}

#[record]
pub struct UnfurlerDebugResponse {
    /// unfurler log
    pub log: Vec<log::Entry>,

    /// the generated embeds
    pub embeds: Vec<Embed>,
}

#[record]
pub struct UnfurlerRequest {
    #[schema(min_length = 1, max_length = 4)]
    #[validate(length(min = 1, max = 4))]
    pub urls: Vec<Url>,
}

#[record]
pub struct UnfurlerResponse {
    pub embeds: Vec<Embed>,
}

export_models!(use log, UnfurlerDebugRequest, UnfurlerDebugResponse, UnfurlerRequest, UnfurlerResponse);
