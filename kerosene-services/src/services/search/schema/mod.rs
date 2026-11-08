//! tantivy schema definition

use tantivy::schema::Schema;

pub trait IndexDefinition {
    /// get the tantivy schema for this index
    fn schema(&self) -> &Schema;

    /// the name of this index (path where it should be created)
    fn name(&self) -> String;

    /// the version of this index' schema
    ///
    /// bumping this drops the existing index and queues a full reindex
    fn version(&self) -> u64;
}

mod doctype;
pub mod unified;

pub use doctype::Doctype;
