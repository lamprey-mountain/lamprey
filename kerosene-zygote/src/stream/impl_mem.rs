use crate::stream::{StreamRead, StreamWrite, WriteError, config::Config};

/// a stream that exists in memory
pub struct MemoryStream {
    name: String,
    config: Config,
}

impl StreamRead for MemoryStream {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &Config {
        &self.config
    }

    // TODO
}

impl StreamWrite for MemoryStream {
    async fn freeze(&self) -> Result<bool, WriteError> {
        todo!()
    }

    // TODO
}
