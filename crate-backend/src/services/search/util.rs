use common::v1::types::{search::Order, ChannelId};
use once_cell::sync::Lazy;
use tantivy::{
    query::{BooleanQuery, Occur, Query, TermSetQuery},
    Term,
};

use crate::services::search::schema::UnifiedSchema;

mod visibility;

pub static SCHEMA: Lazy<UnifiedSchema> = Lazy::new(|| UnifiedSchema::default());

/// generate a tantivy query to restrict visibility
pub fn generate_tantivy_query_for_channel_visibility(
    visible_channel_ids: &[(ChannelId, bool)],
) -> BooleanQuery {
    let mut channel_terms = vec![];
    let mut parent_channel_terms = vec![];
    for (id, can_view_private_threads) in visible_channel_ids {
        let id_str = id.to_string();
        channel_terms.push(Term::from_field_text(SCHEMA.channel_id, &id_str));

        if *can_view_private_threads {
            parent_channel_terms.push(Term::from_field_text(SCHEMA.parent_channel_id, &id_str));
        }
    }

    let mut vis_queries: Vec<(Occur, Box<dyn Query>)> = vec![];

    if !channel_terms.is_empty() {
        vis_queries.push((Occur::Should, Box::new(TermSetQuery::new(channel_terms))));
    }

    if !parent_channel_terms.is_empty() {
        vis_queries.push((
            Occur::Should,
            Box::new(TermSetQuery::new(parent_channel_terms)),
        ));
    }

    BooleanQuery::new(vis_queries)
}

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
