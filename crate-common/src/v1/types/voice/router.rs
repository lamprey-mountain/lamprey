//! routing logic

use std::collections::{HashMap, HashSet};

use crate::v1::types::SfuId;

/// voice route calculator
pub struct VoiceRouter {
    /// latency in nanoseconds between `(src, dest)`
    pub latencies: HashMap<(SfuId, SfuId), u32>,
}

#[derive(Debug, Clone)]
pub struct VoiceRouterConfig {
    /// the default latency to assume for unknown links
    // default 300ms
    pub default_latency: u32,

    /// the if latency is this high, attempt to rebalance
    // default 80ms
    pub maximum_latency: u32,

    /// if a node has <= merge_threshold connection, attempt to merge
    // default 3
    pub merge_threshold: u32,
}

#[derive(Debug, Clone)]
pub struct VoiceTopology {
    pub links: Vec<VoiceLink>,
}

/// voice links are more or less bidirectional
#[derive(Debug, Clone)]
pub struct VoiceLink {
    pub src: SfuId,
    pub dest: SfuId,
}

// // A simple Union-Find for Kruskal's Algorithm
// struct DisjointSet {
//     parent: HashMap<NodeId, NodeId>,
// }
// impl DisjointSet {
//     fn new(nodes: &HashSet<NodeId>) -> Self {
//         Self { parent: nodes.iter().map(|&n| (n, n)).collect() }
//     }
//     fn find(&mut self, i: NodeId) -> NodeId {
//         if self.parent[&i] == i { i } else {
//             let p = self.parent[&i];
//             let root = self.find(p);
//             self.parent.insert(i, root);
//             root
//         }
//     }
//     fn union(&mut self, i: NodeId, j: NodeId) -> bool {
//         let root_i = self.find(i);
//         let root_j = self.find(j);
//         if root_i != root_j {
//             self.parent.insert(root_i, root_j);
//             true
//         } else { false }
//     }
// }

// TODO: impl Default for RouterConfig

impl VoiceRouter {
    pub fn new(config: VoiceRouterConfig) -> Self {
        todo!()
    }

    /// update the rtt in nanos between two sfus
    pub fn update_latency(&mut self, src: SfuId, dest: SfuId, latency: u32) {
        todo!()
    }

    /// get the rtt in nanos between two sfus
    pub fn get_latency(&self, a: SfuId, b: SfuId) -> u32 {
        // return 0 if same
        // check both directions
        todo!()
    }

    /// calculate minimum spanning tree for a channel's active nodes
    pub fn calculate_topology(&self, active_nodes: &HashSet<SfuId>) -> VoiceTopology {
        // if active_nodes.len() < 2 { return vec![]; }

        // // 1. Generate all possible edges between active nodes
        // let nodes: Vec<NodeId> = active_nodes.iter().copied().collect();
        // let mut edges = Vec::new();

        // for i in 0..nodes.len() {
        //     for j in (i + 1)..nodes.len() {
        //         let r1 = self.nodes[&nodes[i]].region;
        //         let r2 = self.nodes[&nodes[j]].region;
        //         let cost = self.get_latency(r1, r2);
        //         edges.push((cost, nodes[i], nodes[j]));
        //     }
        // }

        // // 2. Sort by lowest latency
        // edges.sort_by_key(|(cost, _, _)| *cost);

        // // 3. Kruskal's algorithm to build the tree
        // let mut mst = Vec::new();
        // let mut ds = DisjointSet::new(active_nodes);

        // for (_, u, v) in edges {
        //     if ds.union(u, v) {
        //         mst.push(BackboneLink { from: u, to: v });
        //         // Stop early if we have N-1 edges
        //         if mst.len() == active_nodes.len() - 1 { break; }
        //     }
        // }
        // mst

        todo!()
    }
}
