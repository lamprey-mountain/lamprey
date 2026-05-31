use common::v1::types::search::Order;
use once_cell::sync::Lazy;
use tantivy::query::{BooleanQuery, Occur, Query};

use crate::services::search::schema::UnifiedSchema;

pub mod visibility;

pub static SCHEMA: Lazy<UnifiedSchema> = Lazy::new(|| UnifiedSchema::default());

pub trait IntoTantivyOrder {
    fn tantivy(self) -> tantivy::Order;
}

impl IntoTantivyOrder for Order {
    fn tantivy(self) -> tantivy::Order {
        match self {
            Order::Ascending => tantivy::Order::Asc,
            Order::Descending => tantivy::Order::Desc,
        }
    }
}

/// boolean query builder
pub struct BqBuilder {
    queries: Vec<(Occur, Box<dyn Query>)>,
}

impl BqBuilder {
    /// create a new boolean query builder
    pub fn new() -> Self {
        Self {
            queries: Vec::new(),
        }
    }

    /// push a new `Occur::Should` query
    pub fn should(&mut self, query: Box<dyn Query>) {
        self.queries.push((Occur::Should, query));
    }

    /// push a new `Occur::Must` query
    pub fn must(&mut self, query: Box<dyn Query>) {
        self.queries.push((Occur::Must, query));
    }

    pub fn build(self) -> Box<dyn Query> {
        Box::new(BooleanQuery::from(self.queries))
    }
}
