use multigraphrs::{Directed, GraphErrors, MultiGraph, Undirected, Weighted, WeightedDirected};

// ───────────────────────────────────────────────────────────────────────────
// 1. Basic ID reuse: remove a node, add a new one, the slot gets recycled
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn reuse_slot_after_single_removal_directed() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();

    g.add_node("A").unwrap(); // id 0
    g.add_node("B").unwrap(); // id 1
    g.add_node("C").unwrap(); // id 2

    g.remove_node(&"B").unwrap(); // frees id 1

    // "D" should reuse the freed slot
    g.add_node("D").unwrap();

    // The graph should still have exactly 3 live nodes
    assert_eq!(g.node_count(), 3);

    // All three are queryable
    assert!(g.contains_node(&"A"));
    assert!(!g.contains_node(&"B"));
    assert!(g.contains_node(&"C"));
    assert!(g.contains_node(&"D"));
}

#[test]
fn reuse_slot_after_single_removal_undirected() {
    let mut g = MultiGraph::<u32, u32, Undirected>::new();

    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_node(3).unwrap();

    g.remove_node(&2).unwrap();
    g.add_node(20).unwrap(); // should reuse slot

    assert_eq!(g.node_count(), 3);
    assert!(g.contains_node(&1));
    assert!(g.contains_node(&20));
    assert!(g.contains_node(&3));
    assert!(!g.contains_node(&2));
}

#[test]
fn reuse_slot_after_single_removal_weighted() {
    let mut g = MultiGraph::<&str, f64, Weighted>::new();

    g.add_node("X").unwrap();
    g.add_node("Y").unwrap();

    g.remove_node(&"X").unwrap();
    g.add_node("Z").unwrap(); // reuses X's slot

    assert_eq!(g.node_count(), 2);
    assert!(g.contains_node(&"Y"));
    assert!(g.contains_node(&"Z"));
    assert!(!g.contains_node(&"X"));
}

#[test]
fn reuse_slot_after_single_removal_weighted_directed() {
    let mut g = MultiGraph::<u32, i32, WeightedDirected>::new();

    g.add_node(100).unwrap();
    g.add_node(200).unwrap();

    g.remove_node(&100).unwrap();
    g.add_node(300).unwrap(); // reuses 100's slot

    assert_eq!(g.node_count(), 2);
    assert!(g.contains_node(&200));
    assert!(g.contains_node(&300));
    assert!(!g.contains_node(&100));
}

// ───────────────────────────────────────────────────────────────────────────
// 2. Edges are correct after index reuse
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn edges_work_correctly_after_reuse_directed() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();

    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_edge("A", "B").unwrap();

    // Remove B (which had edges from A)
    g.remove_node(&"B").unwrap();

    // A should have no edges left
    assert_eq!(g.degree(&"A").unwrap(), 0);

    // Add C, which reuses B's slot
    g.add_node("C").unwrap();

    // A -> C should work fine
    g.add_edge("A", "C").unwrap();
    assert_eq!(g.degree(&"A").unwrap(), 1);
    assert!(g.contains_edge(&"A", &"C"));
    assert!(!g.contains_edge(&"A", &"B"));
}

#[test]
fn edges_work_correctly_after_reuse_undirected() {
    let mut g = MultiGraph::<&str, u32, Undirected>::new();

    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_edge("A", "B").unwrap();

    assert_eq!(g.degree(&"A").unwrap(), 1);
    assert_eq!(g.degree(&"B").unwrap(), 1);

    g.remove_node(&"B").unwrap();
    assert_eq!(g.degree(&"A").unwrap(), 0);

    g.add_node("C").unwrap();
    g.add_edge("A", "C").unwrap();

    assert_eq!(g.degree(&"A").unwrap(), 1);
    assert_eq!(g.degree(&"C").unwrap(), 1);
    assert!(g.contains_edge(&"A", &"C"));
}

#[test]
fn edges_work_correctly_after_reuse_weighted() {
    let mut g = MultiGraph::<&str, f64, Weighted>::new();

    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_edge("A", "B", 3.14).unwrap();

    g.remove_node(&"B").unwrap();

    g.add_node("C").unwrap();
    g.add_edge("A", "C", 2.71).unwrap();

    let neighbours = g.get_neighbours(&"A").unwrap();
    assert_eq!(neighbours.len(), 1);
    assert_eq!(neighbours[0].get_target(), &"C");
    assert_eq!(neighbours[0].get_weight(), &2.71);
}

#[test]
fn edges_work_correctly_after_reuse_weighted_directed() {
    let mut g = MultiGraph::<&str, i32, WeightedDirected>::new();

    g.add_node("src").unwrap();
    g.add_node("dst").unwrap();
    g.add_edge("src", "dst", 42).unwrap();

    g.remove_node(&"dst").unwrap();
    g.add_node("new_dst").unwrap();
    g.add_edge("src", "new_dst", 99).unwrap();

    let nb = g.get_neighbours(&"src").unwrap();
    assert_eq!(nb.len(), 1);
    assert_eq!(nb[0].get_target(), &"new_dst");
    assert_eq!(nb[0].get_weight(), &99);
}

// ───────────────────────────────────────────────────────────────────────────
// 3. Multiple removals + re-adds (LIFO reuse order)
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn multiple_removals_then_readd() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();

    // Add 5 nodes (ids 0..4)
    for i in 0..5 {
        g.add_node(i).unwrap();
    }
    assert_eq!(g.node_count(), 5);

    // Remove 3 of them
    g.remove_node(&1).unwrap();
    g.remove_node(&3).unwrap();
    g.remove_node(&4).unwrap();
    assert_eq!(g.node_count(), 2);

    // Add 3 new nodes — they should reuse the freed slots
    g.add_node(10).unwrap();
    g.add_node(11).unwrap();
    g.add_node(12).unwrap();
    assert_eq!(g.node_count(), 5);

    // Original survivors are still there
    assert!(g.contains_node(&0));
    assert!(g.contains_node(&2));

    // New nodes are present
    assert!(g.contains_node(&10));
    assert!(g.contains_node(&11));
    assert!(g.contains_node(&12));

    // Removed nodes are gone
    assert!(!g.contains_node(&1));
    assert!(!g.contains_node(&3));
    assert!(!g.contains_node(&4));
}

#[test]
fn exhaust_free_list_then_grow() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();

    g.add_node(0).unwrap();
    g.add_node(1).unwrap();
    g.remove_node(&0).unwrap();
    g.remove_node(&1).unwrap();

    assert_eq!(g.node_count(), 0);

    // These two reuse the free slots
    g.add_node(10).unwrap();
    g.add_node(11).unwrap();

    // This one must grow (no more free slots)
    g.add_node(12).unwrap();

    assert_eq!(g.node_count(), 3);
    assert!(g.contains_node(&10));
    assert!(g.contains_node(&11));
    assert!(g.contains_node(&12));
}

// ───────────────────────────────────────────────────────────────────────────
// 4. Interleaved add/remove cycles
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn interleaved_add_remove_cycle() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();

    // Round 1: add + remove
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(1, 2).unwrap();
    g.remove_node(&1).unwrap();

    // Round 2: re-add on recycled slot + new edges
    g.add_node(3).unwrap(); // reuses 1's slot
    g.add_edge(3, 2).unwrap();
    assert!(g.contains_edge(&3, &2));
    assert!(!g.contains_node(&1));

    // Round 3: remove and re-add again
    g.remove_node(&2).unwrap();
    g.add_node(4).unwrap(); // reuses 2's slot
    g.add_edge(3, 4).unwrap();

    assert_eq!(g.node_count(), 2);
    assert!(g.contains_edge(&3, &4));
    assert!(!g.contains_node(&2));
}

#[test]
fn repeated_add_remove_same_key() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();

    for _ in 0..10 {
        g.add_node("ping").unwrap();
        g.remove_node(&"ping").unwrap();
    }

    // After all cycles, graph should be empty
    assert_eq!(g.node_count(), 0);
    assert!(!g.contains_node(&"ping"));

    // We should still be able to add it back
    g.add_node("ping").unwrap();
    assert_eq!(g.node_count(), 1);
    assert!(g.contains_node(&"ping"));
}

// ───────────────────────────────────────────────────────────────────────────
// 5. Edge count integrity with free-space reuse
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn edge_count_stays_consistent_through_reuse() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();

    g.add_node(0).unwrap();
    g.add_node(1).unwrap();
    g.add_node(2).unwrap();
    g.add_edge(0, 1).unwrap();
    g.add_edge(0, 2).unwrap();
    g.add_edge(1, 2).unwrap();

    assert_eq!(g.edge_count(), 3);

    // Remove node 1 → edges 0->1 and 1->2 are removed
    g.remove_node(&1).unwrap();
    assert_eq!(g.edge_count(), 1); // only 0->2 remains
    assert_eq!(g.node_count(), 2);

    // Add node 3 (reuses slot of 1)
    g.add_node(3).unwrap();
    g.add_edge(3, 0).unwrap();
    g.add_edge(3, 2).unwrap();

    assert_eq!(g.edge_count(), 3);
    assert_eq!(g.node_count(), 3);
}

#[test]
fn edge_count_undirected_with_reuse() {
    let mut g = MultiGraph::<&str, u32, Undirected>::new();

    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_node("C").unwrap();
    g.add_edge("A", "B").unwrap(); // 2 internal edges (both dirs)
    g.add_edge("B", "C").unwrap(); // 2 more

    assert_eq!(g.edge_count(), 4); // undirected = 2 internal edges per logical edge

    g.remove_node(&"B").unwrap(); // removes all 4 edges
    assert_eq!(g.edge_count(), 0);

    g.add_node("D").unwrap(); // reuses B's slot
    g.add_edge("A", "D").unwrap();
    assert_eq!(g.edge_count(), 2);
}

// ───────────────────────────────────────────────────────────────────────────
// 6. Neighbours resolve to correct keys after reuse
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn neighbours_after_reuse_show_new_key() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();

    g.add_node("A").unwrap();
    g.add_node("OLD").unwrap();
    g.add_edge("A", "OLD").unwrap();

    g.remove_node(&"OLD").unwrap();
    g.add_node("NEW").unwrap(); // reuses OLD's slot

    // A has no edges (the A -> OLD edge was cleaned up on removal)
    assert_eq!(g.degree(&"A").unwrap(), 0);

    // Build fresh edge
    g.add_edge("A", "NEW").unwrap();

    let nb = g.get_neighbours(&"A").unwrap();
    assert_eq!(nb.len(), 1);
    assert_eq!(nb[0].get_target(), &"NEW"); // must resolve to "NEW", not "OLD"
}

#[test]
fn neighbours_weighted_after_reuse() {
    let mut g = MultiGraph::<&str, f64, Weighted>::new();

    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_node("C").unwrap();
    g.add_edge("A", "B", 1.0).unwrap();
    g.add_edge("A", "C", 2.0).unwrap();

    g.remove_node(&"B").unwrap();
    g.add_node("D").unwrap(); // reuses B's slot

    g.add_edge("A", "D", 5.0).unwrap();

    let mut nb = g.get_neighbours(&"A").unwrap();
    nb.sort_by(|a, b| a.get_weight().partial_cmp(b.get_weight()).unwrap());

    // Should have C (2.0) and D (5.0)
    assert_eq!(nb.len(), 2);
    assert_eq!(nb[0].get_target(), &"C");
    assert_eq!(nb[0].get_weight(), &2.0);
    assert_eq!(nb[1].get_target(), &"D");
    assert_eq!(nb[1].get_weight(), &5.0);
}

// ───────────────────────────────────────────────────────────────────────────
// 7. Iterator correctness after reuse
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn iterator_skips_removed_slots_and_shows_reused() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();

    g.add_node("A").unwrap();
    g.add_node("B").unwrap();
    g.add_node("C").unwrap();

    g.remove_node(&"B").unwrap();

    // Before reuse: iterator should yield A and C only
    let keys_before: Vec<&str> = g.iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(keys_before.len(), 2);
    assert!(keys_before.contains(&"A"));
    assert!(keys_before.contains(&"C"));

    // After reuse
    g.add_node("D").unwrap();
    let keys_after: Vec<&str> = g.iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(keys_after.len(), 3);
    assert!(keys_after.contains(&"A"));
    assert!(keys_after.contains(&"C"));
    assert!(keys_after.contains(&"D"));
    assert!(!keys_after.contains(&"B"));
}

// ───────────────────────────────────────────────────────────────────────────
// 8. Stress test: large-scale add/remove/re-add
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn stress_add_remove_readd() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();

    let n = 500;

    // Add N nodes
    for i in 0..n {
        g.add_node(i).unwrap();
    }
    assert_eq!(g.node_count(), n as usize);

    // Remove every other node
    for i in (0..n).step_by(2) {
        g.remove_node(&i).unwrap();
    }
    assert_eq!(g.node_count(), (n / 2) as usize);

    // Re-add with new keys
    for i in 0..(n / 2) {
        g.add_node(n + i).unwrap();
    }
    assert_eq!(g.node_count(), n as usize);

    // Verify: even originals are gone, odd originals survive, new keys present
    for i in (0..n).step_by(2) {
        assert!(!g.contains_node(&i), "removed node {} should be gone", i);
    }
    for i in (1..n).step_by(2) {
        assert!(g.contains_node(&i), "surviving node {} should exist", i);
    }
    for i in 0..(n / 2) {
        assert!(g.contains_node(&(n + i)), "new node {} should exist", n + i);
    }
}

#[test]
fn stress_edges_survive_reuse() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();

    // Build a chain: 0 -> 1 -> 2 -> ... -> 9
    for i in 0..10 {
        g.add_node(i).unwrap();
    }
    for i in 0..9 {
        g.add_edge(i, i + 1).unwrap();
    }
    assert_eq!(g.edge_count(), 9);

    // Remove node 5 (breaks edges 4->5 and 5->6)
    g.remove_node(&5).unwrap();
    assert_eq!(g.edge_count(), 7);

    // Add node 50 (reuses 5's slot), reconnect
    g.add_node(50).unwrap();
    g.add_edge(4, 50).unwrap();
    g.add_edge(50, 6).unwrap();

    assert_eq!(g.edge_count(), 9);
    assert!(g.contains_edge(&4, &50));
    assert!(g.contains_edge(&50, &6));
    assert!(!g.contains_edge(&4, &5));
    assert!(!g.contains_edge(&5, &6));
}

// ───────────────────────────────────────────────────────────────────────────
// 9. Edge case: remove all nodes, then re-add
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn remove_all_nodes_then_rebuild() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();

    for i in 0..5 {
        g.add_node(i).unwrap();
    }
    for i in 0..4 {
        g.add_edge(i, i + 1).unwrap();
    }

    // Remove everything
    for i in 0..5 {
        g.remove_node(&i).unwrap();
    }
    assert_eq!(g.node_count(), 0);
    assert_eq!(g.edge_count(), 0);

    // Rebuild entirely on recycled slots
    for i in 100..105 {
        g.add_node(i).unwrap();
    }
    for i in 100..104 {
        g.add_edge(i, i + 1).unwrap();
    }

    assert_eq!(g.node_count(), 5);
    assert_eq!(g.edge_count(), 4);
    assert!(g.contains_edge(&100, &101));
    assert!(g.contains_edge(&103, &104));
}

// ───────────────────────────────────────────────────────────────────────────
// 10. Edge case: duplicate add after remove should succeed
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn can_readd_same_key_after_removal() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();

    g.add_node("A").unwrap();
    g.remove_node(&"A").unwrap();

    // Re-adding the same key should succeed
    assert_eq!(g.add_node("A"), Ok("A"));
    assert!(g.contains_node(&"A"));
    assert_eq!(g.node_count(), 1);
}

#[test]
fn cannot_add_duplicate_without_removal() {
    let mut g = MultiGraph::<&str, u32, Directed>::new();

    g.add_node("A").unwrap();
    // Without removing, should fail
    assert_eq!(g.add_node("A"), Err(GraphErrors::NodeAlreadyExists));
}

// ───────────────────────────────────────────────────────────────────────────
// 11. Self-loop on reused slot
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn self_loop_on_reused_slot() {
    let mut g = MultiGraph::<u32, u32, Directed>::new();

    g.add_node(0).unwrap();
    g.add_node(1).unwrap();
    g.remove_node(&1).unwrap();

    g.add_node(2).unwrap(); // reuses slot 1
    g.add_edge(2, 2).unwrap(); // self-loop

    assert!(g.contains_edge(&2, &2));
    assert_eq!(g.degree(&2).unwrap(), 1);

    let nb = g.get_neighbours(&2).unwrap();
    assert_eq!(nb.len(), 1);
    assert_eq!(nb[0].get_target(), &2);
}

// ───────────────────────────────────────────────────────────────────────────
// 12. Weighted directed full cycle: add, connect, remove, reuse, reconnect
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn full_lifecycle_weighted_directed() {
    let mut g = MultiGraph::<&str, f64, WeightedDirected>::new();

    g.add_node("alpha").unwrap();
    g.add_node("beta").unwrap();
    g.add_node("gamma").unwrap();

    g.add_edge("alpha", "beta", 1.5).unwrap();
    g.add_edge("beta", "gamma", 2.5).unwrap();
    g.add_edge("gamma", "alpha", 3.5).unwrap();

    assert_eq!(g.edge_count(), 3);

    // Remove beta
    g.remove_node(&"beta").unwrap();
    assert_eq!(g.edge_count(), 1); // only gamma -> alpha
    assert_eq!(g.node_count(), 2);

    // Add delta on beta's recycled slot
    g.add_node("delta").unwrap();
    g.add_edge("alpha", "delta", 10.0).unwrap();
    g.add_edge("delta", "gamma", 20.0).unwrap();

    assert_eq!(g.edge_count(), 3);
    assert!(g.contains_edge(&"alpha", &"delta"));
    assert!(g.contains_edge(&"delta", &"gamma"));
    assert!(g.contains_edge(&"gamma", &"alpha"));
    assert!(!g.contains_edge(&"alpha", &"beta"));
}
