use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// a health check report for a server
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Healthcheck {
    /// overall status of the server
    pub status: HealthcheckStatus,

    /// individual statuses for the server's services
    pub services: HealthcheckServices,

    /// issues that were found
    pub issues: Vec<HealthcheckIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthcheckServices {
    pub database: HealthcheckStatus,
    pub object_store: HealthcheckStatus,
    pub messaging: HealthcheckStatus,
    pub queue: HealthcheckStatus,

    /// the email sending service/smtp server
    pub email: HealthcheckStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthcheckIssue {
    /// whats this check is for (eg. database, email, etc...)
    pub source: String,

    /// how bad is it
    pub severity: HealthcheckSeverity,

    /// what's wrong
    pub message: String,

    /// why its a problem
    pub detail: Option<String>,

    /// how to fix it
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
pub enum HealthcheckStatus {
    /// there's nothing wrong with this
    Healthy,

    /// something's wrong with this, but it can keep functioning
    Unhealthy,

    /// this is unable to keep functioning
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum HealthcheckSeverity {
    /// not a problem but still worth knowing
    Info,

    /// this is something you should fix when you have time
    Warning,

    /// this is something you should fix as soon as possible
    Error,

    /// the server can't continue until this is fixed
    Fatal,
}

impl Healthcheck {
    /// is this server ready to accept requests?
    pub fn is_ready(&self) -> bool {
        // as long as it's not failed, it's good enough
        self.status < HealthcheckStatus::Failed
    }
}

impl HealthcheckServices {
    /// get the overall staus of this server
    pub fn overall(&self) -> HealthcheckStatus {
        let critical = self.database.max(self.messaging);
        let other = self
            .database
            .max(self.object_store)
            .max(self.email)
            .max(self.queue);
        critical.max(other.min(HealthcheckStatus::Unhealthy))
    }
}

impl HealthcheckIssue {
    /// create a new `HealthcheckIssue`
    pub fn new(
        source: impl Into<String>,
        message: impl Into<String>,
        severity: HealthcheckSeverity,
    ) -> Self {
        Self {
            source: source.into(),
            severity,
            message: message.into(),
            detail: None,
            suggestion: None,
        }
    }

    /// create a new `HealthcheckIssue` with severity `Info`
    pub fn info(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(source, message, HealthcheckSeverity::Info)
    }

    /// create a new `HealthcheckIssue` with severity `Warning`
    pub fn warning(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(source, message, HealthcheckSeverity::Warning)
    }

    /// create a new `HealthcheckIssue` with severity `Error`
    pub fn error(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(source, message, HealthcheckSeverity::Error)
    }

    /// create a new `HealthcheckIssue` with severity `Fatal`
    pub fn fatal(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(source, message, HealthcheckSeverity::Fatal)
    }

    /// set a detail for this issue
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// set a suggestion for this issue
    pub fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// log this issue with tracing
    pub fn log(&self) {
        match self.severity {
            HealthcheckSeverity::Info => tracing::info!(
                source = %self.source,
                message = %self.message,
                detail = ?self.detail,
                suggestion = ?self.suggestion,
            ),
            HealthcheckSeverity::Warning => tracing::warn!(
                source = %self.source,
                message = %self.message,
                detail = ?self.detail,
                suggestion = ?self.suggestion,
            ),
            HealthcheckSeverity::Error | HealthcheckSeverity::Fatal => tracing::error!(
                source = %self.source,
                message = %self.message,
                detail = ?self.detail,
                suggestion = ?self.suggestion,
            ),
        }
    }
}
