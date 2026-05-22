//! routing logic

use std::collections::{HashMap, HashSet};

use crate::v1::types::SfuId;

/// voice route calculator
pub struct VoiceRouter {
    /// latency in nanoseconds between `(src, dest)`
    pub latencies: HashMap<(SfuId, SfuId), u32>,
    pub config: VoiceRouterConfig,
}

#[derive(Debug, Clone)]
pub struct VoiceRouterConfig {
    /// the default latency to assume for unknown links
    pub default_latency: u32,

    /// if latency is this high, attempt to rebalance
    pub maximum_latency: u32,

    /// if a node has <= merge_threshold connections, attempt to merge
    pub merge_threshold: u32,
}

impl Default for VoiceRouterConfig {
    fn default() -> Self {
        Self {
            // 300ms in nanoseconds
            default_latency: 300_000_000,
            // 80ms in nanoseconds
            maximum_latency: 80_000_000,
            merge_threshold: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VoiceTopology {
    pub links: Vec<VoiceLink>,
}

/// voice links are bidirectional
#[derive(Debug, Clone)]
pub struct VoiceLink {
    pub src: SfuId,
    pub dest: SfuId,
}

// A simple Union-Find for Kruskal's Algorithm
struct DisjointSet {
    parent: HashMap<SfuId, SfuId>,
}

impl DisjointSet {
    fn new(nodes: &HashSet<SfuId>) -> Self {
        Self {
            parent: nodes.iter().map(|&n| (n, n)).collect(),
        }
    }

    /// Finds the representative of the set containing `i` with path compression.
    ///
    /// This iterative implementation avoids borrow checker conflicts.
    fn find(&mut self, i: SfuId) -> SfuId {
        let mut root = i;
        while self.parent[&root] != root {
            root = self.parent[&root];
        }

        // Path compression
        let mut curr = i;
        while curr != root {
            let next = self.parent[&curr];
            self.parent.insert(curr, root);
            curr = next;
        }

        root
    }

    fn union(&mut self, i: SfuId, j: SfuId) -> bool {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent.insert(root_i, root_j);
            true
        } else {
            false
        }
    }
}

impl VoiceRouter {
    pub fn new(config: VoiceRouterConfig) -> Self {
        Self {
            latencies: HashMap::new(),
            config,
        }
    }

    /// update the rtt in nanos between two sfus
    pub fn update_latency(&mut self, src: SfuId, dest: SfuId, latency: u32) {
        self.latencies.insert((src, dest), latency);
    }

    /// get the rtt in nanos between two sfus
    pub fn get_latency(&self, a: SfuId, b: SfuId) -> u32 {
        if a == b {
            return 0;
        }

        match (self.latencies.get(&(a, b)), self.latencies.get(&(b, a))) {
            (Some(&lat_ab), Some(&lat_ba)) => std::cmp::min(lat_ab, lat_ba),
            (Some(&lat), None) | (None, Some(&lat)) => lat,
            (None, None) => self.config.default_latency,
        }
    }

    /// calculate minimum spanning tree for a channel's active nodes
    pub fn calculate_topology(&self, active_nodes: &HashSet<SfuId>) -> VoiceTopology {
        if active_nodes.len() < 2 {
            return VoiceTopology { links: vec![] };
        }

        // 1. Generate all unique edges between active nodes
        let nodes: Vec<SfuId> = active_nodes.iter().copied().collect();
        let mut edges = Vec::new();

        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let u = nodes[i];
                let v = nodes[j];
                let cost = self.get_latency(u, v);
                edges.push((cost, u, v));
            }
        }

        // 2. Sort by lowest latency (cost)
        edges.sort_by_key(|(cost, _, _)| *cost);

        // 3. Kruskal's algorithm to build the tree
        let mut mst = Vec::new();
        let mut ds = DisjointSet::new(active_nodes);

        for (_, u, v) in edges {
            if ds.union(u, v) {
                mst.push(VoiceLink { src: u, dest: v });
                // Stop early once we've joined all nodes (requires N-1 links)
                if mst.len() == active_nodes.len() - 1 {
                    break;
                }
            }
        }

        VoiceTopology { links: mst }
    }
}
