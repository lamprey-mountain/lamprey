use crate::services::search::schema::IndexDefinition;
use tantivy::schema::Schema;

pub use kerosene_search::schema::{SCHEMA_VERSION, UnifiedSchema};

/// an index containing any lamprey data type
#[derive(Default)]
pub struct UnifiedIndex {
    schema: UnifiedSchema,
}

impl IndexDefinition for UnifiedIndex {
    fn schema(&self) -> &Schema {
        &self.schema.schema
    }

    fn name(&self) -> String {
        "unified".to_owned()
    }

    fn version(&self) -> u64 {
        SCHEMA_VERSION
    }
}
