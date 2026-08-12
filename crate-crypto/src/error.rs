use openmls::group::ExportSecretError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// failed to export a secret from a mls group
    #[error("{0}")]
    ExportSecret(#[from] ExportSecretError),
}
