// Example usage of the RGA implementation
// This file demonstrates how to use the RGA for collaborative editing

use rga::{RemoteOp, Rga, S4Vector};

fn main() {
    println!("=== RGA Collaborative Text Editor Example ===\n");

    // Example 1: Single site editing
    example_single_site();

    // Example 2: Two sites with concurrent operations
    example_two_sites();

    // Example 3: The dOPT puzzle (from the paper)
    example_dopt_puzzle();

    // Example 4: Delete vs Update precedence
    example_delete_update_precedence();

    // Example 5: Vector clock propagation
    example_vector_clock_propagation();
}

fn example_single_site() {
    println!("Example 1: Single Site Editing");
    println!("================================");

    let mut rga = Rga::<char>::new(0, 1);

    // Insert some characters
    rga.insert_local(0, 'H');
    rga.insert_local(1, 'e');
    rga.insert_local(2, 'l');
    rga.insert_local(3, 'l');
    rga.insert_local(4, 'o');

    println!("After insertions: {:?}", rga.read());

    // Update a character
    rga.update_local(1, 'E');
    println!("After update: {:?}", rga.read());

    // Delete a character
    rga.delete_local(2);
    println!("After delete: {:?}", rga.read());

    println!();
}

fn example_two_sites() {
    println!("Example 2: Two Sites with Concurrent Operations");
    println!("================================================");

    // Initialize two sites
    let mut alice = Rga::<String>::new(0, 2);
    let mut bob = Rga::<String>::new(1, 2);

    // Alice creates initial document
    let op1 = alice.insert_local(0, "Hello".to_string()).unwrap();
    let op2 = alice.insert_local(1, "World".to_string()).unwrap();

    // Bob receives Alice's operations
    bob.apply_remote(op1);
    bob.apply_remote(op2);

    println!("Initial state:");
    println!("  Alice: {:?}", alice.read());
    println!("  Bob:   {:?}", bob.read());

    // Concurrent operations
    // Alice inserts between Hello and World
    let alice_op = alice.insert_local(1, "Beautiful".to_string()).unwrap();

    // Bob updates "World" to "Universe"
    let bob_op = bob.update_local(1, "Universe".to_string()).unwrap();

    // Exchange operations
    alice.apply_remote(bob_op);
    bob.apply_remote(alice_op);

    println!("\nAfter concurrent operations:");
    println!("  Alice: {:?}", alice.read());
    println!("  Bob:   {:?}", bob.read());
    println!("  Converged: {}", alice.read() == bob.read());

    println!();
}

fn example_dopt_puzzle() {
    println!("Example 3: The dOPT Puzzle (Example 1 from paper)");
    println!("==================================================");
    println!("Three sites concurrently insert at the same position.");
    println!("This tests Operation Commutativity and Precedence Transitivity.\n");

    // Three sites
    let mut site0 = Rga::<char>::new(0, 3);
    let mut site1 = Rga::<char>::new(1, 3);
    let mut site2 = Rga::<char>::new(2, 3);

    // Initialize with [a, b]
    let op_a = site0.insert_local(0, 'a').unwrap();
    let op_b = site0.insert_local(1, 'b').unwrap();

    // Replicate to all sites
    site1.apply_remote(op_a.clone());
    site1.apply_remote(op_b.clone());
    site2.apply_remote(op_a.clone());
    site2.apply_remote(op_b.clone());

    println!("Initial state at all sites: {:?}\n", site0.read());

    // Concurrent inserts at position 1 (after 'a')
    println!("Concurrent operations:");
    let op1 = site0.insert_local(1, '1').unwrap();
    println!("  Site 0 inserts '1' at index 1");

    let op2 = site1.insert_local(1, '2').unwrap();
    println!("  Site 1 inserts '2' at index 1");

    let op3 = site2.insert_local(1, '3').unwrap();
    println!("  Site 2 inserts '3' at index 1\n");

    // Apply operations in different orders at each site
    println!("Applying operations in different orders:");

    // Site 0: O1 (local), O3, O2
    site0.apply_remote(op3.clone());
    site0.apply_remote(op2.clone());
    println!("  Site 0 execution: O1 → O3 → O2 = {:?}", site0.read());

    // Site 1: O2 (local), O3, O1
    site1.apply_remote(op3.clone());
    site1.apply_remote(op1.clone());
    println!("  Site 1 execution: O2 → O3 → O1 = {:?}", site1.read());

    // Site 2: O3 (local), O2, O1
    site2.apply_remote(op2.clone());
    site2.apply_remote(op1.clone());
    println!("  Site 2 execution: O3 → O2 → O1 = {:?}", site2.read());

    println!(
        "\nAll sites converged: {}",
        site0.read() == site1.read() && site1.read() == site2.read()
    );
    println!("Final state: {:?}", site0.read());
    println!("(Ordering based on S4Vector: site_id 0 < 1 < 2, higher goes closer to left cobject)");

    println!();
}

fn example_delete_update_precedence() {
    println!("Example 4: Delete vs Update Precedence");
    println!("=======================================");
    println!("Demonstrates that Delete always wins over Update.\n");

    let mut alice = Rga::<String>::new(0, 2);
    let mut bob = Rga::<String>::new(1, 2);

    // Create initial document
    let op_init = alice.insert_local(0, "original".to_string()).unwrap();
    bob.apply_remote(op_init);

    println!("Initial state: {:?}\n", alice.read());

    // Concurrent operations
    println!("Concurrent operations:");
    let delete_op = alice.delete_local(0).unwrap();
    println!("  Alice deletes index 0");

    let update_op = bob.update_local(0, "modified".to_string()).unwrap();
    println!("  Bob updates index 0 to 'modified'\n");

    // Apply in different orders
    alice.apply_remote(update_op.clone());
    bob.apply_remote(delete_op.clone());

    println!("Results:");
    println!("  Alice: {:?}", alice.read());
    println!("  Bob:   {:?}", bob.read());
    println!("  Converged: {}", alice.read() == bob.read());
    println!("  Delete won: {}", alice.is_empty());

    println!();
}

fn example_vector_clock_propagation() {
    println!("Example 5: Vector Clock Propagation");
    println!("====================================");

    // Initialize three sites
    let mut site0 = Rga::<char>::new(0, 3);
    let mut site1 = Rga::<char>::new(1, 3);
    let mut site2 = Rga::<char>::new(2, 3);

    println!("Demonstrating how vector clocks are propagated with operations...\n");

    // Site 0 creates initial document
    println!("Site 0 creates initial document:");
    let op_a = site0.insert_local(0, 'a').unwrap();
    let op_b = site0.insert_local(1, 'b').unwrap();

    // Inspect vector clock in operation
    if let RemoteOp::Insert { vector_clock, .. } = &op_b {
        println!("  Site 0's vector clock after 'b': {:?}", vector_clock);
        println!("  Meaning: Site 0 has performed 2 operations, others have 0");
    }

    // Sites 1 and 2 receive operations
    println!("\nSites 1 and 2 receive operations from Site 0:");
    site1.apply_remote(op_a.clone());
    site1.apply_remote(op_b.clone());
    site2.apply_remote(op_a.clone());
    site2.apply_remote(op_b.clone());
    println!("  Vector clocks updated: Site 1 and 2 now know about Site 0's operations");

    // Now Site 1 and Site 2 perform concurrent inserts
    println!("\nSite 1 and Site 2 perform concurrent inserts:");
    let op_1 = site1.insert_local(1, '1').unwrap();
    let op_2 = site2.insert_local(1, '2').unwrap();

    if let RemoteOp::Insert { vector_clock, .. } = &op_1 {
        println!("  Site 1's vector clock: {:?}", vector_clock);
        println!("    [0]=2 (from Site 0), [1]=1 (own operation), [2]=0");
    }

    if let RemoteOp::Insert { vector_clock, .. } = &op_2 {
        println!("  Site 2's vector clock: {:?}", vector_clock);
        println!("    [0]=2 (from Site 0), [1]=0, [2]=1 (own operation)");
    }

    // Exchange operations
    println!("\nExchanging operations between all sites:");
    site0.apply_remote(op_1.clone());
    site0.apply_remote(op_2.clone());
    site1.apply_remote(op_2.clone());
    site2.apply_remote(op_1.clone());

    println!("  Site 0: {:?}", site0.read());
    println!("  Site 1: {:?}", site1.read());
    println!("  Site 2: {:?}", site2.read());

    // Verify convergence
    if site0.read() == site1.read() && site1.read() == site2.read() {
        println!("\n✓ All sites converged successfully!");
        println!("  Vector clocks enabled correct S4Vector generation");
        println!("  S4Vector ordering determined final position of concurrent inserts");
    }

    println!();
}

// Helper function to demonstrate serialization
fn demonstrate_serialization() {
    use serde_json;

    let op = RemoteOp::Insert {
        left_id: None,
        value: "Hello".to_string(),
        s4v: S4Vector::new(1, 0, 1, 1),
        vector_clock: vec![1, 0], // Vector clock for 2 sites
    };

    // Serialize to JSON for network transmission
    let json = serde_json::to_string(&op).unwrap();
    println!("Serialized operation: {}", json);

    // Deserialize
    let deserialized: RemoteOp<String> = serde_json::from_str(&json).unwrap();
    println!("Deserialized successfully");
}
