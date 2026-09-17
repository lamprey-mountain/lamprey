use std::collections::{HashMap, HashSet, VecDeque};

use common::{
    v1::types::{
        ChannelType,
        components::{Component, ComponentState, ComponentType},
        document::{WikiGraph, WikiGraphDocument, WikiGraphQuery},
    },
    v2::types::ChannelId,
};
use lamprey_markdown::{Parser, ast::inline::MentionData, query::QueryableExt};

use crate::{prelude::*, services::documents::ServiceDocuments};

use kerosene_core::{
    error::{ApiError, ErrorCode},
    types::documents::EditContextId,
};

// TODO: use DocumentId instead of ChannelId

#[derive(Debug, Default, Clone)]
struct Graph {
    edges: Vec<Edge>,
    nodes: HashMap<ChannelId, Node>,
    // TODO(future): filtering by tag id
    // tags: HashMap<TagId, HashSet<DocumentId>>,
}

#[derive(Debug, Clone)]
struct Edge {
    from: ChannelId,
    to: ChannelId,
    // TODO(future): weighing edges?
    // weight: u64,
}

#[derive(Debug, Clone)]
struct Node {
    title: String,
    // TODO(future): weighing nodes?
    // weight: u64,
}

// TODO: more robust text extraction
fn extract_text<C: ComponentState>(component: &Component<C>, out: &mut String) {
    if let ComponentType::Text { content } = &component.ty {
        out.push_str(content);
        out.push('\n');
    }

    for child in component.children() {
        extract_text(child, out);
    }
}

impl Graph {
    /// Build an undirected adjacency list from the edge list.
    fn adjacency(&self) -> HashMap<ChannelId, Vec<ChannelId>> {
        let mut adj: HashMap<ChannelId, Vec<ChannelId>> = HashMap::new();
        for edge in &self.edges {
            adj.entry(edge.from).or_default().push(edge.to);
            adj.entry(edge.to).or_default().push(edge.from);
        }
        adj
    }

    pub fn extract(&self, query: &WikiGraphQuery) -> WikiGraph {
        let limit = query.limit as usize;

        let selected_ids: HashSet<ChannelId> = if let Some(center_id) = query.document_id {
            // BFS from center document to select the closest `limit` nodes
            let adj = self.adjacency();
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();

            if self.nodes.contains_key(&center_id) {
                visited.insert(center_id);
                queue.push_back(center_id);
            }

            while let Some(id) = queue.pop_front() {
                if visited.len() >= limit {
                    break;
                }

                if let Some(neighbors) = adj.get(&id) {
                    for &neighbor in neighbors {
                        if visited.insert(neighbor) {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }

            visited
        } else {
            // No center document: take up to `limit` documents
            self.nodes.keys().take(limit).copied().collect()
        };

        let documents: Vec<WikiGraphDocument> = selected_ids
            .iter()
            .filter_map(|id| {
                self.nodes.get(id).map(|node| WikiGraphDocument {
                    id: *id,
                    title: node.title.clone(),
                    weight: 1,
                })
            })
            .collect();

        let links: Vec<(ChannelId, ChannelId)> = self
            .edges
            .iter()
            .filter(|e| selected_ids.contains(&e.from) && selected_ids.contains(&e.to))
            .map(|e| (e.from, e.to))
            .collect();

        WikiGraph { links, documents }
    }
}

impl ServiceDocuments {
    pub async fn query_wiki_graph(
        &self,
        wiki_id: ChannelId,
        query: &WikiGraphQuery,
    ) -> Result<WikiGraph> {
        // PERF: cache computed graphs
        let graph = self.compute_graph(wiki_id).await?;
        Ok(graph.extract(query))
    }

    /// compute a graph for a wiki from scratch
    async fn compute_graph(&self, wiki_id: ChannelId) -> Result<Graph> {
        let mut txn = self.globals.begin_read().await?;

        // TODO: check wiki_channel channel type
        let wiki_channel = txn.channel_get(wiki_id).await?;
        let room_id = wiki_channel
            .room_id
            .ok_or_else(|| Error::ApiError(ApiError::from_code(ErrorCode::ChannelNotInRoom)))?;

        // TODO: also fetch archived documents
        let all_channels = txn.channel_list(room_id).await?;
        let documents: Vec<_> = all_channels
            .into_iter()
            .filter(|c| c.parent_id == Some(wiki_id) && c.ty == ChannelType::Document)
            .collect();
        let doc_ids: HashSet<ChannelId> =
            documents.iter().map(|c| ChannelId::from(*c.id)).collect();

        let mut edges = Vec::new();
        let mut nodes = HashMap::new();
        let parser = Parser::new();

        for doc in documents {
            let doc_id = ChannelId::from(*doc.id);
            nodes.insert(
                doc_id,
                Node {
                    title: doc.name.clone(),
                },
            );

            let context_id = EditContextId::from_prose(wiki_id, (*doc_id).into());

            // PERF(?): don't create actors for EVERY document in a room (or at least unload them immediately after)
            if let Ok(serdoc) = self.get_content(context_id).await {
                let mut full_text = String::new();
                for component in &serdoc.components {
                    extract_text(component, &mut full_text);
                }

                let parsed = parser.parse(&full_text);
                let tree = parsed.tree();

                for mention in tree.iter_mentions() {
                    if let MentionData::Channel(target_id) = mention.parse() {
                        if doc_ids.contains(&target_id) {
                            edges.push(Edge {
                                from: doc_id,
                                to: target_id,
                            });
                        }
                    }
                }

                // TODO: handle iter_urls
            }
        }

        // TODO: incrementally update the graph as edits are made
        // TODO: include all documents in the graph, cache in db

        Ok(Graph { edges, nodes })
    }
}
