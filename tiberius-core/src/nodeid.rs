use std::sync::Arc;

use rand::RngCore;
use tiberius_dependencies::hex;
use tiberius_dependencies::uuid::timestamp::Timestamp;
use tiberius_dependencies::uuid::{Context, Uuid};

#[derive(Clone)]
pub struct NodeId {
    node_id: [u8; 6],
}

impl std::fmt::Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeId")
            .field("node_id", &hex::encode(self.node_id))
            .finish()
    }
}

impl std::ops::Deref for NodeId {
    fn deref(&self) -> &Self::Target {
        &self.node_id
    }

    type Target = [u8; 6];
}

impl From<[u8; 6]> for NodeId {
    fn from(node_id: [u8; 6]) -> Self {
        NodeId { node_id }
    }
}

impl NodeId {
    /// Generates a new UUID that is monotonically increasing and contains the node ID
    pub fn uuid(&self) -> Uuid {
        Uuid::now_v6(self)
    }
    /// Generate a NodeID from NODE_ID environment variables or randomly
    pub fn new() -> Self {
        if let Ok(node_id) = std::env::var("NODE_ID") {
            let mut node_id = [0u8; 6];
            hex::decode_to_slice(node_id, &mut node_id).expect("NODE_ID is in invalid format; 12 Hex Characters [a-fA-F0-9]");
            Self { node_id }
        } else {
            let mut node_id = [0u8; 6];
            rand::thread_rng().fill_bytes(&mut node_id);
            Self { node_id }
        }
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}