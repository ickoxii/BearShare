// Comprehensive integration tests for RGA
// Tests convergence under various concurrent scenarios

#[cfg(test)]
mod integration_tests {
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

        fn print_states(&self) {
            for (i, site) in self.sites.iter().enumerate() {
                println!("  Site {}: {:?}", i, site.read());
            }
        }
    }

    #[test]
    fn test_sequential_inserts() {
        println!("\n=== Test: Sequential Inserts ===");
        let mut net = Network::new(2);

        // Site 0 inserts "Hello"
        net.local_op(0, |rga| rga.insert_local(0, 'H'));
        net.deliver_all();

        net.local_op(0, |rga| rga.insert_local(1, 'e'));
        net.deliver_all();

        net.local_op(0, |rga| rga.insert_local(2, 'l'));
        net.deliver_all();

        net.local_op(0, |rga| rga.insert_local(3, 'l'));
        net.deliver_all();

        net.local_op(0, |rga| rga.insert_local(4, 'o'));
        net.deliver_all();

        net.print_states();
        assert!(net.check_convergence());
        assert_eq!(net.sites[0].read(), vec!['H', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn test_concurrent_inserts_different_positions() {
        println!("\n=== Test: Concurrent Inserts at Different Positions ===");
        let mut net = Network::new(3);

        // Initialize with "ac"
        net.local_op(0, |rga| rga.insert_local(0, 'a'));
        net.local_op(0, |rga| rga.insert_local(1, 'c'));
        net.deliver_all();

        // Concurrent inserts at different positions
        net.local_op(0, |rga| rga.insert_local(1, 'b')); // Site 0: insert 'b' between 'a' and 'c'
        net.local_op(1, |rga| rga.insert_local(2, 'd')); // Site 1: insert 'd' after 'c'
        net.local_op(2, |rga| rga.insert_local(0, 'z')); // Site 2: insert 'z' at beginning

        net.deliver_all();

        println!("After concurrent inserts:");
        net.print_states();
        assert!(net.check_convergence());
        assert_eq!(net.sites[0].read(), vec!['z', 'a', 'b', 'c', 'd']);
    }

    #[test]
    fn test_dopt_puzzle_detailed() {
        println!("\n=== Test: dOPT Puzzle (Detailed) ===");
        let mut net = Network::new(3);

        // Initialize
        net.local_op(0, |rga| rga.insert_local(0, 'a'));
        net.local_op(0, |rga| rga.insert_local(1, 'b'));
        net.deliver_all();

        println!("Initial: {:?}", net.sites[0].read());

        // Three sites insert at same position concurrently
        println!("\nConcurrent operations:");
        println!("  Site 0 inserts '1' at index 1");
        println!("  Site 1 inserts '2' at index 1");
        println!("  Site 2 inserts '3' at index 1");

        net.local_op(0, |rga| rga.insert_local(1, '1'));
        net.local_op(1, |rga| rga.insert_local(1, '2'));
        net.local_op(2, |rga| rga.insert_local(1, '3'));

        net.deliver_all();

        println!("\nFinal states:");
        net.print_states();

        assert!(net.check_convergence());

        // All sites should converge
        // Based on S4Vector ordering: site 0 < site 1 < site 2 (when sum is equal)
        // Higher site IDs insert closer to left cobject
        let result = net.sites[0].read();
        println!("Converged to: {:?}", result);
        assert_eq!(result.len(), 5); // a, one of {1,2,3}, another, b
        assert_eq!(result[0], 'a');
        assert_eq!(result[4], 'b');
    }

    #[test]
    fn test_concurrent_deletes() {
        println!("\n=== Test: Concurrent Deletes ===");
        let mut net = Network::new(2);

        // Initialize with "abcd"
        for ch in ['a', 'b', 'c', 'd'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        println!("Initial: {:?}", net.sites[0].read());

        // Concurrent deletes
        net.local_op(0, |rga| rga.delete_local(1)); // Delete 'b'
        net.local_op(1, |rga| rga.delete_local(2)); // Delete 'c'

        net.deliver_all();

        println!("After concurrent deletes:");
        net.print_states();

        assert!(net.check_convergence());
        assert_eq!(net.sites[0].read(), vec!['a', 'd']);
    }

    #[test]
    fn test_concurrent_updates() {
        println!("\n=== Test: Concurrent Updates ===");
        let mut net = Network::new(3);

        // Initialize with "abc"
        for ch in ['a', 'b', 'c'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        println!("Initial: {:?}", net.sites[0].read());

        // Three sites update middle character concurrently
        println!("\nConcurrent updates to index 1:");
        println!("  Site 0 updates to 'X'");
        println!("  Site 1 updates to 'Y'");
        println!("  Site 2 updates to 'Z'");

        net.local_op(0, |rga| rga.update_local(1, 'X'));
        net.local_op(1, |rga| rga.update_local(1, 'Y'));
        net.local_op(2, |rga| rga.update_local(1, 'Z'));

        net.deliver_all();

        println!("\nFinal states:");
        net.print_states();

        assert!(net.check_convergence());

        // Highest priority update wins (by S4Vector order)
        let result = net.sites[0].read();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], 'a');
        assert_eq!(result[2], 'c');
        // Middle should be one of the updates
        assert!(['X', 'Y', 'Z'].contains(&result[1]));
    }

    #[test]
    fn test_delete_update_conflict() {
        println!("\n=== Test: Delete vs Update Conflict ===");
        let mut net = Network::new(2);

        // Initialize with "abc"
        for ch in ['a', 'b', 'c'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        println!("Initial: {:?}", net.sites[0].read());

        // Concurrent Delete and Update on same character
        println!("\nConcurrent operations on index 1:");
        println!("  Site 0 deletes");
        println!("  Site 1 updates to 'X'");

        net.local_op(0, |rga| rga.delete_local(1));
        net.local_op(1, |rga| rga.update_local(1, 'X'));

        net.deliver_all();

        println!("\nFinal states:");
        net.print_states();

        assert!(net.check_convergence());

        // Delete always wins
        assert_eq!(net.sites[0].read(), vec!['a', 'c']);
        println!("Delete won over Update");
    }

    #[test]
    fn test_complex_scenario() {
        println!("\n=== Test: Complex Mixed Operations ===");
        let mut net = Network::new(3);

        // Site 0 creates initial document
        for ch in ['H', 'e', 'l', 'l', 'o'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        println!("Initial: {:?}", net.sites[0].read());

        // Complex concurrent operations:
        // Site 0: Insert space at end, update first char
        // Site 1: Delete second 'l', insert '!' at end
        // Site 2: Update 'e' to 'a', insert 'x' in middle

        println!("\nConcurrent complex operations:");
        net.local_op(0, |rga| rga.insert_local(5, ' '));
        net.local_op(0, |rga| rga.update_local(0, 'h'));

        net.local_op(1, |rga| rga.delete_local(3));
        net.local_op(1, |rga| rga.insert_local(rga.len(), '!'));

        net.local_op(2, |rga| rga.update_local(1, 'a'));
        net.local_op(2, |rga| rga.insert_local(2, 'x'));

        net.deliver_all();

        println!("\nFinal states:");
        net.print_states();

        assert!(net.check_convergence());

        let result = net.sites[0].read();
        println!("Converged to: {:?}", result);
        // All sites should have the same result
    }

    #[test]
    fn test_insert_after_delete() {
        println!("\n=== Test: Insert After Delete (Tombstone as Cobject) ===");
        let mut net = Network::new(2);

        // Initialize with "abc"
        for ch in ['a', 'b', 'c'] {
            net.local_op(0, move |rga| rga.insert_local(rga.len(), ch));
        }
        net.deliver_all();

        println!("Initial: {:?}", net.sites[0].read());

        // Site 0 deletes 'b'
        net.local_op(0, |rga| rga.delete_local(1));
        net.deliver_all();

        println!("After delete: {:?}", net.sites[0].read());

        // Site 1 inserts 'x' after 'b' (which is now tombstone)
        // This should work because tombstone is preserved as cobject
        net.local_op(1, |rga| {
            // Before delete arrives, 'b' is at index 1
            // Insert 'x' at index 2 (after 'b', before 'c')
            rga.insert_local(1, 'x')
        });

        net.deliver_all();

        println!("After insert:");
        net.print_states();

        assert!(net.check_convergence());

        // 'b' is deleted but 'x' inserted after it should remain
        let result = net.sites[0].read();
        println!("Result: {:?}", result);
        // Should have a, x, c (assuming insert succeeded with tombstone as cobject)
    }

    #[test]
    fn test_many_concurrent_inserts_same_position() {
        println!("\n=== Test: Many Concurrent Inserts at Same Position ===");
        let mut net = Network::new(5);

        // Initialize with markers
        net.local_op(0, |rga| rga.insert_local(0, '['));
        net.local_op(0, |rga| rga.insert_local(1, ']'));
        net.deliver_all();

        println!("Initial: {:?}", net.sites[0].read());

        // All sites insert at same position (index 1, between [ and ])
        println!("\nAll sites insert at index 1:");
        for i in 0..5 {
            let ch = char::from_digit(i, 10).unwrap();
            println!("  Site {} inserts '{}'", i, ch);
            net.local_op(i as usize, move |rga| rga.insert_local(1, ch));
        }

        net.deliver_all();

        println!("\nFinal states:");
        net.print_states();

        assert!(net.check_convergence());

        let result = net.sites[0].read();
        assert_eq!(result[0], '[');
        assert_eq!(result[6], ']');
        println!("Converged order: {:?}", result);
        // Order determined by S4Vector
    }

    #[test]
    fn test_causality_violation_detection() {
        println!("\n=== Test: Causality Violation Detection ===");
        // This tests that operations missing their cobjects fail gracefully

        let mut site = Rga::<char>::new(0, 2);

        // Try to insert with non-existent left cobject
        let fake_s4v = S4Vector::new(1, 1, 100, 50);
        let op = RemoteOp::Insert {
            left_id: Some(fake_s4v),
            value: 'x',
            s4v: S4Vector::new(1, 1, 101, 51),
            vector_clock: vec![100, 0], // Fake vector clock
        };

        site.apply_remote(op);

        // Operation should have failed gracefully
        assert_eq!(site.read().len(), 0);
        println!("Causality violation handled gracefully");
    }

    #[test]
    fn test_interleaved_operations() {
        println!("\n=== Test: Interleaved Operations ===");
        let mut net = Network::new(2);

        // Simulate real editing pattern with interleaved ops
        net.local_op(0, |rga| rga.insert_local(0, 'a'));
        net.deliver_all();

        net.local_op(1, |rga| rga.insert_local(1, 'b'));
        net.deliver_all();

        net.local_op(0, |rga| rga.insert_local(2, 'c'));
        net.local_op(1, |rga| rga.update_local(0, 'A'));
        net.deliver_all();

        net.local_op(0, |rga| rga.delete_local(1));
        net.local_op(1, |rga| rga.insert_local(3, 'd'));
        net.deliver_all();

        println!("Final states:");
        net.print_states();

        assert!(net.check_convergence());
        println!("Converged: {:?}", net.sites[0].read());
    }

    #[test]
    fn test_empty_document_operations() {
        println!("\n=== Test: Operations on Empty Document ===");
        let mut net = Network::new(2);

        // Concurrent inserts on empty document
        net.local_op(0, |rga| rga.insert_local(0, 'a'));
        net.local_op(1, |rga| rga.insert_local(0, 'b'));

        net.deliver_all();

        println!("After concurrent head inserts:");
        net.print_states();

        assert!(net.check_convergence());

        // Both inserted at head, order by S4Vector
        let result = net.sites[0].read();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&'a'));
        assert!(result.contains(&'b'));
    }

    #[test]
    fn test_vector_clock_propagation() {
        println!("\n=== Test: Vector Clock Propagation ===");
        let mut site0 = Rga::<char>::new(0, 3);
        let mut site1 = Rga::<char>::new(1, 3);
        let mut site2 = Rga::<char>::new(2, 3);

        // Site 0 performs operations
        let op_a = site0.insert_local(0, 'a').unwrap();
        let op_b = site0.insert_local(1, 'b').unwrap();

        // Check that vector clocks are included
        match &op_a {
            RemoteOp::Insert { vector_clock, .. } => {
                println!("Site 0 VC after first insert: {:?}", vector_clock);
                assert_eq!(vector_clock[0], 1);
                assert_eq!(vector_clock[1], 0);
                assert_eq!(vector_clock[2], 0);
            }
            _ => panic!("Expected Insert"),
        }

        match &op_b {
            RemoteOp::Insert { vector_clock, .. } => {
                println!("Site 0 VC after second insert: {:?}", vector_clock);
                assert_eq!(vector_clock[0], 2);
                assert_eq!(vector_clock[1], 0);
                assert_eq!(vector_clock[2], 0);
            }
            _ => panic!("Expected Insert"),
        }

        // Site 1 receives operations and updates its vector clock
        site1.apply_remote(op_a.clone());
        site1.apply_remote(op_b.clone());

        // Site 1 now performs an operation
        let op_c = site1.insert_local(2, 'c').unwrap();

        match &op_c {
            RemoteOp::Insert { vector_clock, .. } => {
                println!("Site 1 VC after insert: {:?}", vector_clock);
                // Should have max(0,2) for site 0, and 1 for site 1
                assert_eq!(vector_clock[0], 2); // Updated from site 0
                assert_eq!(vector_clock[1], 1); // Site 1's own count
                assert_eq!(vector_clock[2], 0); // Site 2 hasn't done anything
            }
            _ => panic!("Expected Insert"),
        }

        // Site 2 receives all operations
        site2.apply_remote(op_a);
        site2.apply_remote(op_b);
        site2.apply_remote(op_c);

        // All sites should converge
        assert_eq!(site0.read(), vec!['a', 'b']);
        assert_eq!(site1.read(), vec!['a', 'b', 'c']);
        assert_eq!(site2.read(), vec!['a', 'b', 'c']);

        println!("Vector clock propagation verified!");
    }
}
