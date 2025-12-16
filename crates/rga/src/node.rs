use crate::s4vector::S4Vector;
use std::cell::RefCell;
use std::rc::Rc;

// Node in the RGA structure
// Combines linked list for document order with hash table for O(1) lookup (SVI scheme)
#[derive(Debug, Clone)]
pub struct Node<T: Clone> {
    // Object value (None indicates tombstone after deletion)
    pub obj: Option<T>,

    // Immutable insert ID - set once when node is created
    // Used for: (1) S4Vector index in hash table, (2) precedence of Inserts
    pub s_k: S4Vector,

    // Mutable precedence ID - updated by Delete and Update operations
    // Used for precedence of Deletes and Updates
    pub s_p: S4Vector,

    // Link to next node in the linked list (document order)
    pub link: Option<Rc<RefCell<Node<T>>>>,
}

impl<T: Clone> Node<T> {
    pub fn new(obj: T, s4v: S4Vector) -> Self {
        Node {
            obj: Some(obj),
            s_k: s4v,
            s_p: s4v,
            link: None,
        }
    }

    pub fn is_tombstone(&self) -> bool {
        self.obj.is_none()
    }
}
