//! tantivy schema definition

use tantivy::schema::Schema;

pub trait IndexDefinition {
    /// get the tantivy schema for this index
    fn schema(&self) -> &Schema;

    /// the name of this index (path where it should be created)
    fn name(&self) -> String;
}

mod doctype;
mod transform;
pub mod unified;

pub use doctype::Doctype;
pub use unified::UnifiedSchema;
