use crate::services::search::schema::IndexDefinition;
use tantivy::schema::Schema;

pub use lamprey_search::schema::UnifiedSchema;

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
}
