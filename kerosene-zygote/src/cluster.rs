//! clustering and pubsub using HyParView and Plumtree

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
};

use bytes::Bytes;
use slotmap::SlotMap;

// TODO: make this work in an "open world" where any node can jon
// - node ids should be cryptographically secure ids (eg. ed25519)
// - gossips should be signed by its origin
// - message ids should be a hash (eg. sha2 or blake3) of the payload
// - figure out how to defend against sybil/eclipse attacks
// - consider taking a gander at gossipsub, as its designed for adversarial networks

// // TODO: use ed25519_dalek
// pub struct NodePublic([u8; 32]);
// pub struct NodeSecret([u8; 32]);

// TODO: rename NodeId, other types? maybe NodeId should be a global rather than local identity.

slotmap::new_key_type! {
    /// a local id for a node
    pub struct NodeId;
}

/// address that can be used to connect to a node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeAddr {
    addr: SocketAddr,
}

/// the state of a single node on the network
pub struct Node {
    /// this node's address
    address: NodeAddr,

    /// active quic connections
    connections: SlotMap<NodeId, Peer>,

    /// active HyParView nodes
    active: HashSet<NodeId>,

    /// passive HyParView nodes
    passive: HashSet<NodeId>,

    /// active random walk length
    active_rwl: u8,

    /// passive random walk length
    passive_rwl: u8,

    /// plumtree broadcasts
    broadcasts: HashMap<String, Broadcast>,
}

/// a handle to a remote node
#[derive(Debug)]
pub struct Peer {
    addr: NodeAddr,
    connection: quinn::Connection,
}

/// a broadcast/gossip tree
#[derive(Debug, Default)]
pub struct Broadcast {
    /// eager Plumtree nodes
    ///
    /// full `Gossip` messages should be sent to these nodes
    eager: HashSet<NodeId>,

    /// lazy Plumtree nodes
    ///
    /// only `Have`s should be sent to these nodes
    lazy: HashSet<NodeId>,

    /// received messages
    ///
    /// bytes are stored in case a `Graft` message is sent
    // TODO: prune received messages after a few seconds
    // use lru or moka for this
    received: HashMap<MessageId, Bytes>,

    /// messages that we don't have
    ///
    /// Stores a map from message id to a list of nodes that have this
    /// message, along with the distance (hops) the message travelled. If a
    /// message isn't received after a timeout, promote one of these nodes
    /// to an eager node; preferribly the one with the lowest latency.
    missing: HashMap<MessageId, Vec<(NodeId, u32)>>,
}

/// globally unique identifier for a message
// NOTE: this is 40 bytes! NodeAddr is 32. i'll probably need 64 bytes for
// ed25519 signatures if/when i make the network open world, but locally this
// probably could be optimized?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageId {
    addr: NodeAddr,
    id: u64,
}

// TODO: plan how this is going to work with quic
//
// - create a new quic stream for each Gossip
// - use datagrams for Have
// - use one quic stream for everything else
//
// handshake: client opens bidi stream and sends Join. that stream is now the "main"/"master" stream

/// a message sent between nodes
pub enum Message {
    /// attempt to join the network
    Join,

    /// a new node joined the network
    ///
    /// This message should be forwarded along a random walk. When
    /// ttl reaches 0 (or a node has no neighbors), the receiving node
    /// should add the new node to its active view. Halfway through (at
    /// passive_rwl), nodes should add the new node to their passive view
    /// instead.
    Joined {
        /// the address of the new node
        addr: NodeAddr,

        /// time to live
        ttl: u8,
    },

    /// notify a node that you are dropping it from your active view
    Disconnect,

    /// ask a node to be an active neighbor
    Neighbor {
        /// if true, the receiver must accept
        ///
        /// set if the sender has no neighbors
        force: bool,
    },

    /// reply to a Neighbor message
    ///
    /// If accepted, both sides have each other in their active sets.
    /// Otherwise, the sender should try another passive candidate.
    NeighborReply { accepted: bool },

    /// shuffle nodes
    ///
    /// This is sent periodically along a random walk. The node this walk
    /// ends up at swaps these nodes into its passive set.
    Shuffle {
        /// the origin node's address
        origin: NodeAddr,

        /// some of the origin's nodes
        nodes: Vec<NodeAddr>,

        /// time to live
        ttl: u8,
    },

    /// reply to a Shuffle message
    ///
    /// Sent to the origin node. The origin should swap out some of its own
    /// nodes with the receiver's nodes.
    ShuffleReply {
        /// Some of the receiver's nodes
        receiver_nodes: Vec<NodeAddr>,

        /// The nodes that the origin node sent to the receiver
        origin_nodes: Vec<NodeAddr>,
    },

    /// a duplicate message was detected
    ///
    /// sent to eager nodes to downgrade them to lazy nodes
    Prune,

    /// i have these messages
    ///
    /// sent to lazy nodes
    // this should probably be sent via datagrams
    // this event should also probably be batched/coalesced to reduce the number of requests (maybe unnecessary with datagrams?)
    Have {
        /// the ids of the messages i have
        // this could maybe be a bloom/cuckoo/xor filter
        ids: Vec<MessageId>,

        /// how many hops this message has travelled
        hops: u8,
    },

    /// upgrade a lazy node to an eager node
    ///
    /// The sender should be sent any missing messages.
    Graft {
        /// the message ids that the sender is missing
        ids: Vec<MessageId>,

        /// how many hops this message has travelled
        hops: u8,
    },

    /// send a full message
    ///
    /// sent to eager nodes. if a node receives this message twice, there's
    /// probably a loop; it should prune one of its peers (presumably the
    /// slowest one).
    Gossip {
        /// unique identifier for this message
        id: MessageId,

        /// how many hops this message has travelled
        hops: u8,

        /// payload for this message
        payload: Bytes,
    },
}

/// an input for a node
pub enum Input {
    /// a new node connected
    Join(NodeAddr),

    /// received a message from a node
    Recv(NodeId, Message),
    // rename join to connect
    // disconnect
    // timeout
}

/// an output for a node
pub enum Output {
    /// send this message to this node
    Send(NodeId, Message),
    // connect
    // disconnect
    // deliver
}

impl Node {
    /// create a new node, listening on an address
    pub fn new(addr: SocketAddr) -> Self {
        // TODO: tune active/passive rwl
        Self {
            address: NodeAddr { addr },
            connections: SlotMap::default(),
            active: HashSet::default(),
            passive: HashSet::default(),
            active_rwl: 6,
            passive_rwl: 3,
            broadcasts: HashMap::default(),
        }
    }
}
