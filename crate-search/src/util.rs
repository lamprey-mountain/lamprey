//! various utility types

use common::v1::types::search::Order;
use tantivy::query::{BooleanQuery, Occur, Query};

pub mod doctype;

// TODO: copy Reindex types to common

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
// TODO: rename?
#[derive(Debug, Default)]
pub struct BqBuilder {
    queries: Vec<(Occur, Box<dyn Query>)>,
}

impl BqBuilder {
    /// create a new boolean query builder
    pub fn new() -> Self {
        Self::default()
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
