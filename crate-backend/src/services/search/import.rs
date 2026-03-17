use async_trait::async_trait;
use kameo::Actor;
use tantivy::TantivyDocument;

use crate::Result;

/// an actor representing an extract transform load job to copy data from postgres into tantivy
#[derive(Actor)]
pub struct ImportActor {
    controller: Box<dyn ImportController>,
}

#[async_trait]
pub trait ImportController: Send + Sync {
    /// get the latest cursor
    async fn cursor(&self) -> Result<String>;

    /// save the cursor
    async fn set_cursor(&self, cursor: String) -> Result<()>;

    /// fetch a batch of documents since this cursor
    async fn batch(&self, limit: usize, cursor: &str) -> Result<(Vec<TantivyDocument>, String)>;
}
