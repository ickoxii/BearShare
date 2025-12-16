use crate::node::Node;
use crate::remote_op::RemoteOp;
use crate::s4vector::S4Vector;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Main RGA structure
///
/// Uses:
/// - Linked list (via Node.link) for maintaining document order
/// - Hash map (SVI scheme) for O(1) lookup by S4Vector
/// - Rc<RefCell<>> for shared mutable access (justified: need multiple references
///   from both linked list and hash map, with interior mutability for updates)
#[derive(Debug)]
pub struct Rga<T: Clone> {
    /// Head of the linked list
    head: Option<Rc<RefCell<Node<T>>>>,

    /// Hash map for S4Vector Index (SVI) scheme - enables O(1) lookup
    /// Maps s_k -> Node reference
    hash_map: HashMap<S4Vector, Rc<RefCell<Node<T>>>>,

    /// Local site configuration
    site_id: u32,
    session: u32,
    vector_clock: Vec<u32>,

    /// Cemetery for tombstone management (Section 5.6)
    cemetery: Vec<S4Vector>,
}

impl<T: Clone> Rga<T> {
    /// Create a new RGA for the given site
    pub fn new(site_id: u32, num_sites: usize) -> Self {
        Rga {
            head: None,
            hash_map: HashMap::new(),
            site_id,
            session: 1,
            vector_clock: vec![0; num_sites],
            cemetery: Vec::new(),
        }
    }

    /// Generate S4Vector for current operation
    fn generate_s4vector(&mut self) -> S4Vector {
        self.vector_clock[self.site_id as usize] += 1;
        let sum: u32 = self.vector_clock.iter().sum();
        let seq = self.vector_clock[self.site_id as usize];

        S4Vector::new(self.session, self.site_id, sum, seq)
    }

    /// Find node by index (for local operations)
    /// Implements Algorithm 4: findlist
    /// Skips tombstones to match visible document order
    fn find_by_index(&self, index: usize) -> Option<Rc<RefCell<Node<T>>>> {
        let mut current = self.head.clone();
        let mut count = 0;

        while let Some(node_rc) = current {
            let node = node_rc.borrow();

            // Skip tombstones (they don't count in visible indices)
            if !node.is_tombstone() {
                if count == index {
                    drop(node);
                    return Some(node_rc.clone());
                }
                count += 1;
            }

            current = node.link.clone();
        }

        None
    }

    /// Find node by S4Vector (for remote operations)
    /// Uses SVI scheme for O(1) lookup
    fn find_by_s4vector(&self, s4v: &S4Vector) -> Option<Rc<RefCell<Node<T>>>> {
        self.hash_map.get(s4v).cloned()
    }

    /// Local Insert operation (Algorithm 4)
    /// Returns RemoteOp for broadcasting
    pub fn insert_local(&mut self, index: usize, value: T) -> Option<RemoteOp<T>> {
        let s4v = self.generate_s4vector();

        // Find left cobject
        let left_id = if index == 0 {
            None
        } else {
            self.find_by_index(index - 1).map(|n| n.borrow().s_k)
        };

        self.remote_insert(left_id, value.clone(), s4v);

        Some(RemoteOp::Insert {
            left_id,
            value,
            s4v,
            vector_clock: self.vector_clock.clone(),
        })
    }

    /// Local Delete operation (Algorithm 4)
    pub fn delete_local(&mut self, index: usize) -> Option<RemoteOp<T>> {
        let target = self.find_by_index(index)?;
        let target_id = target.borrow().s_k;
        let s4v = self.generate_s4vector();

        // Make tombstone
        target.borrow_mut().obj = None;
        target.borrow_mut().s_p = s4v;

        // Enroll in cemetery for later purging
        self.cemetery.push(target_id);

        Some(RemoteOp::Delete {
            target_id,
            s4v,
            vector_clock: self.vector_clock.clone(),
        })
    }

    /// Local Update operation (Algorithm 4)
    pub fn update_local(&mut self, index: usize, value: T) -> Option<RemoteOp<T>> {
        let target = self.find_by_index(index)?;
        let target_id = target.borrow().s_k;
        let s4v = self.generate_s4vector();

        // Update the node
        target.borrow_mut().obj = Some(value.clone());
        target.borrow_mut().s_p = s4v;

        Some(RemoteOp::Update {
            target_id,
            value,
            s4v,
            vector_clock: self.vector_clock.clone(),
        })
    }

    /// Read the current visible document state
    pub fn read(&self) -> Vec<T> {
        let mut result = Vec::new();
        let mut current = self.head.clone();

        while let Some(node_rc) = current {
            let node = node_rc.borrow();

            // Only include non-tombstone nodes
            if let Some(ref obj) = node.obj {
                result.push(obj.clone());
            }

            current = node.link.clone();
        }

        result
    }

    /// Apply remote operation (dispatches to specific handlers)
    /// Implements Algorithm 1 lines 16-17: update vector clock then execute
    pub fn apply_remote(&mut self, op: RemoteOp<T>) {
        // Algorithm 1 line 16: ∀k: v_i[k] := max(v_i[k], v_O[k])
        let op_vc = match &op {
            RemoteOp::Insert { vector_clock, .. } => vector_clock,
            RemoteOp::Delete { vector_clock, .. } => vector_clock,
            RemoteOp::Update { vector_clock, .. } => vector_clock,
        };

        for (i, &op_count) in op_vc.iter().enumerate() {
            if i < self.vector_clock.len() {
                self.vector_clock[i] = self.vector_clock[i].max(op_count);
            }
        }

        // Algorithm 1 line 17: RADT.remoteAlgorithm(O)
        match op {
            RemoteOp::Insert {
                left_id,
                value,
                s4v,
                ..
            } => {
                self.remote_insert(left_id, value, s4v);
            }
            RemoteOp::Delete { target_id, s4v, .. } => {
                self.remote_delete(target_id, s4v);
            }
            RemoteOp::Update {
                target_id,
                value,
                s4v,
                ..
            } => {
                self.remote_update(target_id, value, s4v);
            }
        }
    }

    /// Remote Insert operation (Algorithm 8)
    /// Implements Operation Commutativity (OC) and Precedence Transitivity (PT)
    fn remote_insert(&mut self, left_id: Option<S4Vector>, value: T, s4v: S4Vector) {
        let new_node = Rc::new(RefCell::new(Node::new(value, s4v)));

        // (i) Find left cobject via hash map - O(1)
        if let Some(left_s4v) = left_id {
            let left_node = match self.find_by_s4vector(&left_s4v) {
                Some(n) => n,
                None => {
                    // Cobject not found - should not happen with proper causality
                    eprintln!("Warning: Left cobject not found for Insert");
                    return;
                }
            };

            // (iii) Scan for correct position based on PT
            // From Algorithm 8, line 20: while(ref.link != nil and ins.s_k ≺ ref.link.s_k)
            // Continue scanning while INSERT precedes NEXT node (ins.s_k ≺ next.s_k)
            // Stop when NEXT precedes INSERT (next.s_k ≺ ins.s_k) or they're equal
            // This implements PT: succeeding inserts go closer to left cobject
            let mut ref_node = left_node.clone();
            loop {
                let next = ref_node.borrow().link.clone();

                match next {
                    Some(ref next_rc) => {
                        let next_node = next_rc.borrow();
                        // Algorithm 8 line 20: while(ins.s_k ≺ ref.link.s_k)
                        // Continue if INSERT precedes NEXT (new node should go before next)
                        // Stop if NEXT precedes INSERT (new node should go after ref, before next)
                        if s4v.precedes(&next_node.s_k) {
                            // Insert precedes next, continue scanning
                            drop(next_node);
                            ref_node = next_rc.clone();
                        } else {
                            // Next precedes or equals insert, stop here
                            drop(next_node);
                            break;
                        }
                    }
                    None => break,
                }
            }

            // (iv) Link new node into list (before the first preceding node)
            let next = ref_node.borrow().link.clone();
            new_node.borrow_mut().link = next;
            ref_node.borrow_mut().link = Some(new_node.clone());
        } else {
            // Insert at head (left_id = nil)
            // From Algorithm 8, lines 14-18
            if let Some(ref head_rc) = self.head {
                // Line 15: if(head = nil or head.s_k ≺ ins.s_k)
                if head_rc.borrow().s_k.precedes(&s4v) {
                    // Current head precedes new node, insert new node at head
                    // Lines 16-17
                    new_node.borrow_mut().link = self.head.clone();
                    self.head = Some(new_node.clone());
                } else {
                    // New node precedes head, scan forward from head
                    // Lines 19-22: same scanning logic as above
                    let mut ref_node = head_rc.clone();
                    loop {
                        let next = ref_node.borrow().link.clone();
                        match next {
                            Some(ref next_rc) => {
                                // ✅ FIXED: Continue while ins.s_k ≺ next.s_k
                                if s4v.precedes(&next_rc.borrow().s_k) {
                                    ref_node = next_rc.clone();
                                } else {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    // Lines 21-22
                    let next = ref_node.borrow().link.clone();
                    new_node.borrow_mut().link = next;
                    ref_node.borrow_mut().link = Some(new_node.clone());
                }
            } else {
                // Empty list - new node becomes head
                self.head = Some(new_node.clone());
            }
        }

        // (ii) Add to hash map (SVI scheme)
        self.hash_map.insert(s4v, new_node);
    }

    /// Remote Delete operation (Algorithm 9)
    /// Delete always wins (Delete 99K Update) regardless of s4vector order
    fn remote_delete(&mut self, target_id: S4Vector, s4v: S4Vector) {
        if let Some(target) = self.find_by_s4vector(&target_id) {
            let mut target_mut = target.borrow_mut();

            // Mark as tombstone (preserves cobject for future operations)
            if !target_mut.is_tombstone() {
                self.cemetery.push(target_id);
            }

            target_mut.obj = None;
            target_mut.s_p = s4v;
        } else {
            eprintln!("Warning: Target not found for Delete");
        }
    }

    /// Remote Update operation (Algorithm 10)
    /// Update only succeeds if s4v succeeds current s_p AND target is not tombstone
    /// This implements: Update 99K Delete (Updates don't resurrect tombstones)
    fn remote_update(&mut self, target_id: S4Vector, value: T, s4v: S4Vector) {
        if let Some(target) = self.find_by_s4vector(&target_id) {
            let mut target_mut = target.borrow_mut();

            // Don't update tombstones (Update 99K Delete precedence)
            if target_mut.is_tombstone() {
                return;
            }

            // Only update if new s4v succeeds current s_p (PT)
            if target_mut.s_p.precedes(&s4v) {
                target_mut.obj = Some(value);
                target_mut.s_p = s4v;
            }
        } else {
            eprintln!("Warning: Target not found for Update");
        }
    }

    /// Get current document length (excluding tombstones)
    pub fn len(&self) -> usize {
        self.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use crate::Rga;

    #[test]
    fn test_basic_insert_and_read() {
        let mut rga = Rga::<char>::new(0, 1);

        rga.insert_local(0, 'a');
        rga.insert_local(1, 'b');
        rga.insert_local(2, 'c');

        let content = rga.read();
        assert_eq!(content, vec!['a', 'b', 'c']);
    }

    #[test]
    fn test_delete() {
        let mut rga = Rga::<char>::new(0, 1);

        rga.insert_local(0, 'a');
        rga.insert_local(1, 'b');
        rga.insert_local(2, 'c');
        rga.delete_local(1);

        let content = rga.read();
        assert_eq!(content, vec!['a', 'c']);
    }

    #[test]
    fn test_update() {
        let mut rga = Rga::<char>::new(0, 1);

        rga.insert_local(0, 'a');
        rga.insert_local(1, 'b');
        rga.update_local(1, 'x');

        let content = rga.read();
        assert_eq!(content, vec!['a', 'x']);
    }

    #[test]
    fn test_concurrent_inserts_dopt_puzzle() {
        // Example 1 from the paper: dOPT puzzle
        // Three sites concurrently insert at the same position

        let mut site0 = Rga::<char>::new(0, 3);
        let mut site1 = Rga::<char>::new(1, 3);
        let mut site2 = Rga::<char>::new(2, 3);

        // Initial state: [a, b]
        let op_a = site0.insert_local(0, 'a').unwrap();
        let op_b = site0.insert_local(1, 'b').unwrap();

        site1.apply_remote(op_a.clone());
        site1.apply_remote(op_b.clone());
        site2.apply_remote(op_a.clone());
        site2.apply_remote(op_b.clone());

        // Concurrent inserts after 'a' (index 1)
        let op1 = site0.insert_local(1, '1').unwrap(); // Site 0 inserts '1'
        let op2 = site1.insert_local(1, '2').unwrap(); // Site 1 inserts '2'
        let op3 = site2.insert_local(1, '3').unwrap(); // Site 2 inserts '3'

        // Apply in different orders at each site
        // Site 0: local op1, then remote op3, op2
        site0.apply_remote(op3.clone());
        site0.apply_remote(op2.clone());

        // Site 1: local op2, then remote op3, op1
        site1.apply_remote(op3.clone());
        site1.apply_remote(op1.clone());

        // Site 2: local op3, then remote op2, op1
        site2.apply_remote(op2.clone());
        site2.apply_remote(op1.clone());

        // All sites should converge to same order
        let result0 = site0.read();
        let result1 = site1.read();
        let result2 = site2.read();

        assert_eq!(result0, result1);
        assert_eq!(result1, result2);

        // Based on S4Vector ordering: site 0 < site 1 < site 2
        // Higher priority (later in order) goes closer to left cobject
        println!("Converged result: {:?}", result0);
    }

    #[test]
    fn test_delete_wins_over_update() {
        let mut site0 = Rga::<char>::new(0, 2);
        let mut site1 = Rga::<char>::new(1, 2);

        // Insert 'a'
        let op_insert = site0.insert_local(0, 'a').unwrap();
        site1.apply_remote(op_insert);

        // Concurrent operations: site 0 deletes, site 1 updates
        let op_delete = site0.delete_local(0).unwrap();
        let op_update = site1.update_local(0, 'x').unwrap();

        // Apply in different orders
        site0.apply_remote(op_update.clone());
        site1.apply_remote(op_delete.clone());

        // Both should have tombstone (delete wins)
        assert_eq!(site0.read(), vec![]);
        assert_eq!(site1.read(), vec![]);
    }
}
