use std::collections::HashMap;

/// Trait for receiving unfurler log events.
///
/// Implement this to capture debug information during unfurling,
/// such as HTTP fetches, errors, and failures.
pub trait LogSink: Send + Sync {
    /// Handle a log entry.
    ///
    /// This is called synchronously during unfurling.
    /// Implementations should be fast or offload work asynchronously.
    fn handle(&mut self, entry: LogEntry);
}

/// A log entry from the unfurler.
#[derive(Debug, Clone)]
pub enum LogEntry {
    /// HTTP fetch event (initial request or redirect)
    Fetch(FetchEntry),

    /// Plugin selection event
    SelectPlugin(SelectPluginEntry),

    /// Non-fatal error during embed generation (e.g., invalid HTML)
    Error(ErrorEntry),

    /// Fatal failure that prevented embed generation
    Failed(FailedEntry),
}

/// HTTP fetch log entry.
#[derive(Debug, Clone)]
pub struct FetchEntry {
    /// Whether this is the initial fetch or a redirect
    pub reason: FetchReason,

    /// HTTP status code received
    pub http_status: u16,

    /// HTTP headers received
    pub http_headers: HashMap<String, String>,

    /// First ~4KB of the response body
    pub http_body: String,
}

/// Plugin selection log entry.
#[derive(Debug, Clone)]
pub struct SelectPluginEntry {
    /// The name of the selected plugin
    pub plugin_name: &'static str,

    /// Whether the plugin was selected via URL or response
    pub reason: SelectPluginReason,
}

/// Reason for plugin selection.
#[derive(Debug, Clone, Copy)]
pub enum SelectPluginReason {
    /// Plugin was selected via `process_url` (URL-based matching)
    Url,

    /// Plugin was selected via `accepts_response` (HTTP response matching)
    Response,
}

/// Reason for an HTTP fetch.
#[derive(Debug, Clone)]
pub enum FetchReason {
    /// Initial URL fetch
    Initial,

    /// Redirect to a new URL
    Redirect,
}

/// Non-fatal error during embed generation.
#[derive(Debug, Clone)]
pub struct ErrorEntry {
    /// The error code
    pub code: ErrorCode,

    /// Human-readable error message
    pub message: String,

    /// Additional context (e.g., field name, URL)
    pub context: Option<String>,
}

/// Error codes for non-fatal errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// HTML was malformed or invalid
    InvalidHtml,

    /// Missing expected metadata
    MissingMetadata,

    /// Media URL could not be resolved
    MediaUrlInvalid,

    /// Other parsing error
    ParseError,

    /// Timeout during processing
    Timeout,

    /// Resource too large
    ResourceTooLarge,
}

/// Fatal failure that prevented embed generation.
#[derive(Debug, Clone)]
pub struct FailedEntry {
    /// The failure code
    pub code: FailedCode,

    /// Human-readable failure message
    pub message: String,
}

/// Failure codes for fatal failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailedCode {
    /// Connection timed out
    ConnectionTimeout,

    /// Connection failed (e.g., refused, reset)
    ConnectionFailed,

    /// DNS lookup failed
    DnsLookupFailed,

    /// Invalid HTTP status code (1xx, 4xx, 5xx)
    InvalidStatusCode,

    /// Unsupported protocol (not http/https)
    UnsupportedProtocol,

    /// No plugin could handle the response
    NoPluginMatch,

    /// Forbidden from unfurling this URL
    Forbidden,

    /// Request cancelled
    Cancelled,

    /// Other failure
    Other,
}

impl FetchEntry {
    pub fn new(
        reason: FetchReason,
        http_status: u16,
        http_headers: HashMap<String, String>,
        http_body: String,
    ) -> Self {
        Self {
            reason,
            http_status,
            http_headers,
            http_body,
        }
    }
}

impl SelectPluginEntry {
    pub fn new(plugin_name: &'static str, reason: SelectPluginReason) -> Self {
        Self {
            plugin_name,
            reason,
        }
    }
}

impl ErrorEntry {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn invalid_html(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidHtml,
            message: message.into(),
            context: None,
        }
    }

    pub fn missing_metadata(field: impl Into<String>) -> Self {
        let field = field.into();
        Self {
            code: ErrorCode::MissingMetadata,
            message: format!("Missing metadata field: {}", field.clone()),
            context: Some(field),
        }
    }

    pub fn media_url_invalid(url: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::MediaUrlInvalid,
            message: message.into(),
            context: Some(url.into()),
        }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ParseError,
            message: message.into(),
            context: None,
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Timeout,
            message: message.into(),
            context: None,
        }
    }

    pub fn resource_too_large(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ResourceTooLarge,
            message: message.into(),
            context: None,
        }
    }
}

impl FailedEntry {
    pub fn new(code: FailedCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn connection_timeout() -> Self {
        Self {
            code: FailedCode::ConnectionTimeout,
            message: "Connection timed out".into(),
        }
    }

    pub fn connection_failed(message: impl Into<String>) -> Self {
        Self {
            code: FailedCode::ConnectionFailed,
            message: message.into(),
        }
    }

    pub fn dns_lookup_failed(message: impl Into<String>) -> Self {
        Self {
            code: FailedCode::DnsLookupFailed,
            message: message.into(),
        }
    }

    pub fn invalid_status_code(status: u16) -> Self {
        Self {
            code: FailedCode::InvalidStatusCode,
            message: format!("Invalid HTTP status code: {}", status),
        }
    }

    pub fn unsupported_protocol(protocol: impl Into<String>) -> Self {
        Self {
            code: FailedCode::UnsupportedProtocol,
            message: format!("Unsupported protocol: {}", protocol.into()),
        }
    }

    pub fn no_plugin_match() -> Self {
        Self {
            code: FailedCode::NoPluginMatch,
            message: "No plugin could handle the response".into(),
        }
    }

    pub fn forbidden() -> Self {
        Self {
            code: FailedCode::Forbidden,
            message: "Forbidden from unfurling this URL".into(),
        }
    }

    pub fn cancelled() -> Self {
        Self {
            code: FailedCode::Cancelled,
            message: "Request was cancelled".into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self {
            code: FailedCode::Other,
            message: message.into(),
        }
    }
}

/// In-memory log sink that collects log entries into a vec.
#[derive(Debug, Default, Clone)]
pub struct InMemoryLogSink {
    entries: Vec<LogEntry>,
}

impl InMemoryLogSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_entries(self) -> Vec<LogEntry> {
        self.entries
    }
}

impl LogSink for InMemoryLogSink {
    fn handle(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }
}

/// No-op log sink that discards all entries.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopLogSink;

impl LogSink for NoopLogSink {
    fn handle(&mut self, _entry: LogEntry) {
        // nope!
    }
}
