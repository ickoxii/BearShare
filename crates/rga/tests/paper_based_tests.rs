// Additional comprehensive integration tests for RGA based on
// "Replicated abstract data types: Building Blocks for Collaborative Applications"
// by Roh et al., 2011
//
// These tests validate the theoretical properties and examples from the paper:
// - TP1: Intention Preservation
// - TP2: Convergence
// - Operation Commutativity
// - Precedence Transitivity
// - S4Vector ordering (session, sum, site_id)

#[cfg(test)]
mod paper_based_tests {
    use rga::{RemoteOp, Rga, S4Vector};

    // Helper to simulate a network of sites
    struct Network {
        sites: Vec<Rga<char>>,
        pending_ops: Vec<(usize, RemoteOp<char>)>, // (from_site, op)
    }

    impl Network {
        fn new(num_sites: usize) -> Self {
            let mut sites = Vec::new();
            for i in 0..num_sites {
                sites.push(Rga::new(i as u32, num_sites));
            }
            Network {
                sites,
                pending_ops: Vec::new(),
            }
        }

        fn local_op(
            &mut self,
            site: usize,
            op: impl FnOnce(&mut Rga<char>) -> Option<RemoteOp<char>>,
        ) {
            if let Some(remote_op) = op(&mut self.sites[site]) {
                self.pending_ops.push((site, remote_op));
            }
        }

        fn deliver_all(&mut self) {
            while !self.pending_ops.is_empty() {
                let ops = std::mem::take(&mut self.pending_ops);
                for (from, op) in ops {
                    // Broadcast to all other sites
                    for i in 0..self.sites.len() {
                        if i != from {
                            self.sites[i].apply_remote(op.clone());
                        }
                    }
                }
            }
        }

        fn check_convergence(&self) -> bool {
            let first = self.sites[0].read();
            self.sites.iter().all(|site| site.read() == first)
        }
    }

    // ====================================================================
    // Tests validating theoretical properties from the paper
    // ====================================================================

    #[test]
    fn test_tp1_intention_preservation() {
        let mut net = Network::new(2);

        // Site 0 creates document "Hello"
        for ch in ['H', 'e', 'l', 'l', 'o'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        assert_eq!(net.sites[0].read(), vec!['H', 'e', 'l', 'l', 'o']);

        // Site 1's intention: insert 'X' between 'e' and first 'l' (index 2)
        net.local_op(1, |rga| rga.insert_local(2, 'X'));
        net.deliver_all();

        // All sites should reflect the intention
        let expected = vec!['H', 'e', 'X', 'l', 'l', 'o'];
        assert_eq!(net.sites[0].read(), expected);
        assert_eq!(net.sites[1].read(), expected);
    }

    #[test]
    fn test_tp2_convergence() {
        let mut net = Network::new(3);

        // Initialize
        net.local_op(0, |rga| rga.insert_local(0, 'a'));
        net.deliver_all();

        // Site 0: insert 'x' at position 1
        net.local_op(0, |rga| rga.insert_local(1, 'x'));

        // Site 1: insert 'y' at position 1
        net.local_op(1, |rga| rga.insert_local(1, 'y'));

        // Site 2: insert 'z' at position 1
        net.local_op(2, |rga| rga.insert_local(1, 'z'));

        // Deliver all operations (simulating eventual delivery)
        net.deliver_all();

        // TP2: All sites must converge to identical state
        assert!(net.check_convergence());
    }

    #[test]
    fn test_tp2_different_delivery_orders() {
        // Create 3 separate sites (not using Network helper to control delivery order)
        let mut site_a = Rga::<char>::new(0, 3);
        let mut site_b = Rga::<char>::new(1, 3);
        let mut site_c = Rga::<char>::new(2, 3);

        // Initialize all sites with same state
        let init_op = site_a.insert_local(0, 'x').unwrap();
        site_b.apply_remote(init_op.clone());
        site_c.apply_remote(init_op.clone());

        // Generate three concurrent operations
        let op1 = site_a.insert_local(1, '1').unwrap();
        let op2 = site_b.insert_local(1, '2').unwrap();
        let op3 = site_c.insert_local(1, '3').unwrap();

        // Site A: op2, op3 (already has op1 local)
        site_a.apply_remote(op2.clone());
        site_a.apply_remote(op3.clone());

        // Site B: op3, op1 (already has op2 local)
        site_b.apply_remote(op3.clone());
        site_b.apply_remote(op1.clone());

        // Site C: op1, op2 (already has op3 local)
        site_c.apply_remote(op1.clone());
        site_c.apply_remote(op2.clone());

        // Despite different delivery orders, all must converge
        assert_eq!(site_a.read(), site_b.read());
        assert_eq!(site_b.read(), site_c.read());
    }

    // ====================================================================
    // Tests for S4Vector ordering (from Section 3.2 of paper)
    // ====================================================================

    #[test]
    fn test_s4vector_session_ordering() {
        // Create two S4Vectors with different sessions
        let v1 = S4Vector::new(1, 0, 5, 5); // Session 1
        let v2 = S4Vector::new(2, 1, 3, 3); // Session 2

        // Session 1 should precede Session 2, regardless of other fields
        assert!(v1.precedes(&v2));
        assert!(!v2.precedes(&v1));
    }

    #[test]
    fn test_s4vector_sum_ordering() {
        // Same session, different sums
        let v1 = S4Vector::new(1, 0, 3, 3); // sum=3
        let v2 = S4Vector::new(1, 1, 5, 2); // sum=5

        // Lower sum precedes higher sum
        assert!(v1.precedes(&v2));
        assert!(!v2.precedes(&v1));
    }

    #[test]
    fn test_s4vector_site_id_ordering() {
        // Same session, same sum, different site_id
        let v1 = S4Vector::new(1, 0, 5, 2); // site_id=0
        let v2 = S4Vector::new(1, 1, 5, 3); // site_id=1

        // Lower site_id precedes higher site_id
        assert!(v1.precedes(&v2));
        assert!(!v2.precedes(&v1));
    }

    // ====================================================================
    // Tests for concurrent operation scenarios from the paper
    // ====================================================================

    #[test]
    fn test_multiple_deletes_same_element() {
        let mut net = Network::new(3);

        // Initialize with "abc"
        for ch in ['a', 'b', 'c'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        // All three sites concurrently delete 'b' (index 1)
        net.local_op(0, |rga| rga.delete_local(1));
        net.local_op(1, |rga| rga.delete_local(1));
        net.local_op(2, |rga| rga.delete_local(1));

        net.deliver_all();

        // All should converge to same result with 'b' deleted
        assert!(net.check_convergence());
        assert_eq!(net.sites[0].read(), vec!['a', 'c']);
    }

    #[test]
    fn test_update_update_conflict_precedence() {
        let mut net = Network::new(4);

        // Initialize with single character
        net.local_op(0, |rga| rga.insert_local(0, 'a'));
        net.deliver_all();

        // Four sites update the same character concurrently
        net.local_op(0, |rga| rga.update_local(0, '0'));
        net.local_op(1, |rga| rga.update_local(0, '1'));
        net.local_op(2, |rga| rga.update_local(0, '2'));
        net.local_op(3, |rga| rga.update_local(0, '3'));

        net.deliver_all();

        // All should converge
        assert!(net.check_convergence());

        // The winner should be deterministic based on S4Vector
        let result = net.sites[0].read();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_delete_then_update_no_resurrection() {
        let mut site0 = Rga::<char>::new(0, 2);
        let mut site1 = Rga::<char>::new(1, 2);

        // Initialize
        let op_init = site0.insert_local(0, 'a').unwrap();
        site1.apply_remote(op_init);

        // Site 0 deletes
        let op_del = site0.delete_local(0).unwrap();
        site1.apply_remote(op_del);

        assert_eq!(site0.read(), Vec::<char>::new());

        // Site 1 tries to update (on tombstone) - generates operation before delete
        // This simulates the case where site 1 hasn't received delete yet
        let op_upd_opt = site1.update_local(0, 'a');

        // Now site 0 receives the update
        if let Some(upd) = op_upd_opt {
            site0.apply_remote(upd);
        }

        // Should still be empty - no resurrection
        assert_eq!(site0.read(), Vec::<char>::new());
        assert_eq!(site1.read(), Vec::<char>::new());
    }

    #[test]
    fn test_causal_dependency_chain() {
        let mut net = Network::new(3);

        // Site 0 creates initial state
        net.local_op(0, |rga| rga.insert_local(0, 'a'));
        net.deliver_all();

        // Site 1 inserts 'b' after 'a'
        net.local_op(1, |rga| rga.insert_local(1, 'b'));
        net.deliver_all();

        // Site 2 inserts 'c' after 'b' (causally depends on 'b')
        net.local_op(2, |rga| rga.insert_local(2, 'c'));
        net.deliver_all();

        // All sites should have correct causal order
        assert!(net.check_convergence());
        assert_eq!(net.sites[0].read(), vec!['a', 'b', 'c']);
    }

    #[test]
    fn test_insert_at_head_concurrently() {
        let mut net = Network::new(4);

        // All sites insert at head (position 0) concurrently
        net.local_op(0, |rga| rga.insert_local(0, '0'));
        net.local_op(1, |rga| rga.insert_local(0, '1'));
        net.local_op(2, |rga| rga.insert_local(0, '2'));
        net.local_op(3, |rga| rga.insert_local(0, '3'));

        net.deliver_all();

        // All should converge
        assert!(net.check_convergence());

        let result = net.sites[0].read();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_long_concurrent_insert_sequence() {
        let mut net = Network::new(5);

        // Initialize with markers
        net.local_op(0, |rga| rga.insert_local(0, '['));
        net.local_op(0, |rga| rga.insert_local(1, ']'));
        net.deliver_all();

        // Each site inserts multiple characters at the same position
        for site in 0..5 {
            for i in 0..3 {
                let ch = char::from_digit(site * 3 + i, 36).unwrap();
                net.local_op(site.try_into().unwrap(), move |rga| rga.insert_local(1, ch));
            }
        }

        net.deliver_all();

        // All should converge
        assert!(net.check_convergence());

        let result = net.sites[0].read();
        assert_eq!(result[0], '[');
        assert_eq!(result[result.len() - 1], ']');
        assert_eq!(result.len(), 17); // [ + 15 chars + ]
    }

    #[test]
    fn test_alternating_insert_delete() {
        let mut net = Network::new(2);

        // Site 0 inserts
        net.local_op(0, |rga| rga.insert_local(0, 'a'));
        net.deliver_all();

        // Site 1 deletes
        net.local_op(1, |rga| rga.delete_local(0));
        net.deliver_all();

        assert_eq!(net.sites[0].read().len(), 0);

        // Site 0 inserts again
        net.local_op(0, |rga| rga.insert_local(0, 'b'));
        net.deliver_all();

        // Site 1 inserts
        net.local_op(1, |rga| rga.insert_local(1, 'c'));
        net.deliver_all();

        // Site 0 deletes
        net.local_op(0, |rga| rga.delete_local(0));
        net.deliver_all();

        assert!(net.check_convergence());
        assert_eq!(net.sites[0].read(), vec!['c']);
    }

    #[test]
    fn test_concurrent_operations_all_types() {
        let mut net = Network::new(3);

        // Initialize with "abcd"
        for ch in ['a', 'b', 'c', 'd'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        // Concurrent operations of different types:
        net.local_op(0, |rga| rga.insert_local(2, 'X'));
        net.local_op(1, |rga| rga.delete_local(1));
        net.local_op(2, |rga| rga.update_local(3, 'Z'));

        net.deliver_all();

        // All should converge
        assert!(net.check_convergence());
    }

    // ====================================================================
    // Stress and edge case tests
    // ====================================================================

    #[test]
    fn test_stress_many_sites_many_ops() {
        let num_sites = 8;
        let ops_per_site = 10;

        let mut net = Network::new(num_sites);

        // Each site performs multiple operations
        for site in 0..num_sites {
            for i in 0..ops_per_site {
                let ch = char::from_digit((site * ops_per_site + i) as u32 % 36, 36).unwrap();
                net.local_op(site, move |rga| {
                    let pos = if !rga.is_empty() { rga.len() / 2 } else { 0 };
                    rga.insert_local(pos, ch)
                });
            }
        }

        net.deliver_all();

        // Verify convergence
        assert!(net.check_convergence());
    }

    #[test]
    fn test_insert_between_tombstones() {
        let mut net = Network::new(2);

        // Create "abc"
        for ch in ['a', 'b', 'c'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        // Delete 'b'
        net.local_op(0, |rga| rga.delete_local(1));
        net.deliver_all();

        // Now site 1 inserts 'X' at what used to be position 2 (after deleted 'b')
        // This tests that tombstone 'b' still serves as a valid cobject
        net.local_op(1, |rga| rga.insert_local(1, 'X'));
        net.deliver_all();

        assert!(net.check_convergence());
    }

    #[test]
    fn test_operation_commutativity() {
        // Create three sites that will receive the same operations in different orders
        let mut site0 = Rga::<char>::new(0, 3);
        let mut site1 = Rga::<char>::new(1, 3);
        let mut site2 = Rga::<char>::new(2, 3);

        // Initialize all sites with same starting state
        let init_op = site0.insert_local(0, 'x').unwrap();
        site1.apply_remote(init_op.clone());
        site2.apply_remote(init_op.clone());

        // Generate two concurrent operations from different sites
        let op_a = site1.insert_local(1, 'a').unwrap(); // Site 1 inserts 'a'
        let op_b = site2.insert_local(1, 'b').unwrap(); // Site 2 inserts 'b' (concurrent)

        // Site 0: Apply in order A then B
        site0.apply_remote(op_a.clone());
        site0.apply_remote(op_b.clone());

        // Site 1: Apply B (already has A local)
        site1.apply_remote(op_b.clone());

        // Site 2: Apply A (already has B local)
        site2.apply_remote(op_a.clone());

        // All should reach same state regardless of application order
        assert_eq!(site0.read(), site1.read());
        assert_eq!(site1.read(), site2.read());
    }

    #[test]
    fn test_precedence_transitivity() {
        // Create S4Vectors with transitive precedence
        let v1 = S4Vector::new(1, 0, 1, 1);
        let v2 = S4Vector::new(1, 0, 2, 2);
        let v3 = S4Vector::new(1, 0, 3, 3);

        // Check transitivity
        assert!(v1.precedes(&v2));
        assert!(v2.precedes(&v3));
        assert!(v1.precedes(&v3)); // Transitive property
    }

    #[test]
    fn test_paper_example_2_concurrent_updates_deletes() {
        let mut net = Network::new(3);

        // Initialize document
        for ch in ['a', 'b', 'c', 'd'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        // Site 0: update 'b' to 'B'
        // Site 1: delete 'b'
        // Site 2: insert 'X' after 'b'
        net.local_op(0, |rga| rga.update_local(1, 'B'));
        net.local_op(1, |rga| rga.delete_local(1));
        net.local_op(2, |rga| rga.insert_local(2, 'X'));

        net.deliver_all();

        // All sites must converge
        assert!(net.check_convergence());

        let result = net.sites[0].read();
        // Delete should win over update, so 'b' should be gone
        // 'X' should still be inserted (using tombstone as cobject)
        assert!(!result.contains(&'b') && !result.contains(&'B'));
    }

    #[test]
    fn test_empty_document_concurrent_inserts() {
        let mut net = Network::new(5);

        // All sites insert at position 0 on empty document
        for site in 0..5 {
            let ch = char::from_digit(site, 10).unwrap();
            net.local_op(site.try_into().unwrap(), move |rga| rga.insert_local(0, ch));
        }

        net.deliver_all();

        assert!(net.check_convergence());

        let result = net.sites[0].read();
        assert_eq!(result.len(), 5);
    }
}
