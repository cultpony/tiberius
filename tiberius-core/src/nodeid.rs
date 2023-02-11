
use std::sync::Arc;

use tiberius_dependencies::uuid::{Uuid, Context};
use tiberius_dependencies::uuid::timestamp::Timestamp;
use tiberius_dependencies::hex;

#[derive(Clone)]
pub struct NodeId{
    node_id: [u8; 6],
}

impl std::fmt::Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeId").field("node_id", &hex::encode(&self.node_id)).finish()
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
        NodeId{
            node_id,
        }
    }
}

impl NodeId {
    // Generates a new UUID that is monotonically increasing and contains the node ID
    pub fn uuid(&self) -> Uuid {
        Uuid::now_v6(&self)
    }
}