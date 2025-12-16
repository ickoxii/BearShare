use crate::s4vector::S4Vector;
use serde::{Deserialize, Serialize};

// Remote operations - serializable for network transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemoteOp<T: Clone> {
    // Insert(left_id, value) - inserts value after the node with left_id
    // left_id = None means insert at head
    Insert {
        left_id: Option<S4Vector>,
        value: T,
        s4v: S4Vector,
        vector_clock: Vec<u32>,
    },

    // Delete(target_id) - marks the node with target_id as tombstone
    Delete {
        target_id: S4Vector,
        s4v: S4Vector,
        vector_clock: Vec<u32>,
    },

    // Update(target_id, value) - updates the node with target_id
    Update {
        target_id: S4Vector,
        value: T,
        s4v: S4Vector,
        vector_clock: Vec<u32>,
    },
}
