use multigraphrs::storage::disk_storage::disk_multigraph::DiskStorage;
use multigraphrs::storage::storage_backend::StorageBackend;
use multigraphrs::core::edge::Edge;

use std::env;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_test_id() -> usize {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

struct TempDiskStorage {
    pub storage: DiskStorage<u32, u32>,
    dir: std::path::PathBuf,
}

impl TempDiskStorage {
    fn new(test_name: &str) -> Self {
        let id = next_test_id();
        let mut dir = env::temp_dir();
        dir.push(format!("multigraphrs_disk_test_{}_{}", test_name, id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let storage = DiskStorage::<u32, u32>::new(&dir);

        Self { storage, dir }
    }
}

impl Drop for TempDiskStorage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn test_add_node_increases_node_count() {
    let mut temp = TempDiskStorage::new("add_node");
    
    assert_eq!(temp.storage.node_count(), 0, "Initial node count should be 0");
    temp.storage.add_node().unwrap();
    assert_eq!(temp.storage.node_count(), 1, "Node count should increment after add_node");
    temp.storage.add_node().unwrap();
    assert_eq!(temp.storage.node_count(), 2, "Node count should be 2 after second add_node");
}

#[test]
fn test_increment_node_counter() {
    let mut temp = TempDiskStorage::new("inc_node");
    
    assert_eq!(temp.storage.node_count(), 0);
    temp.storage.increment_node_counter().unwrap();
    assert_eq!(temp.storage.node_count(), 1, "Incrementing node counter should update the internal count");
}

#[test]
fn test_add_edge_to_node_and_counts() {
    let mut temp = TempDiskStorage::new("add_edge");
    temp.storage.add_node().unwrap(); // node 0
    temp.storage.add_node().unwrap(); // node 1

    assert_eq!(temp.storage.edge_count(), 0);
    assert_eq!(temp.storage.node_len(&0), 0);

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(&0, &edge).unwrap();

    assert_eq!(temp.storage.edge_count(), 1, "Global edge count should increment");
    assert_eq!(temp.storage.node_len(&0), 1, "Source node length should increment");
    assert_eq!(temp.storage.node_len(&1), 0, "Target node length should remain 0 for directed addition");
}

#[test]
fn test_get_edges() {
    let mut temp = TempDiskStorage::new("get_edges");
    temp.storage.add_node().unwrap(); // node 0
    temp.storage.add_node().unwrap(); // node 1

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(&0, &edge).unwrap();

    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert_eq!(edges.len(), 1, "Iterator should yield 1 edge");
    assert_eq!(edges[0].get_target(), 1, "Target ID should match");
    assert_eq!(edges[0].get_weight(), 42, "Weight should match");
    
    let empty_edges: Vec<_> = temp.storage.get_edges(&1).collect();
    assert!(empty_edges.is_empty(), "Iterator should yield 0 edges for node with no outgoing edges");
}

#[test]
fn test_contains_edge() {
    let mut temp = TempDiskStorage::new("contains_edge");
    temp.storage.add_node().unwrap(); // node 0
    temp.storage.add_node().unwrap(); // node 1

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(&0, &edge).unwrap();

    // Found edge
    let found = temp.storage.contains_edge(&0, &1);
    assert!(found.is_ok(), "Should confirm that the edge exists");
    assert_eq!(found.unwrap().get_weight(), 42, "Returned edge should have correct weight");

    // Non-existent edge
    let not_found = temp.storage.contains_edge(&1, &0);
    assert!(not_found.is_err(), "Should return an error for non-existent edge");
}

#[test]
fn test_remove_edge() {
    let mut temp = TempDiskStorage::new("remove_edge");
    temp.storage.add_node().unwrap(); // node 0
    temp.storage.add_node().unwrap(); // node 1

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(&0, &edge).unwrap();
    
    assert_eq!(temp.storage.edge_count(), 1);
    assert_eq!(temp.storage.node_len(&0), 1);

    // Remove the edge
    let res = temp.storage.remove_edge(&0, &edge);
    assert!(res.is_ok(), "Removing an existing edge should succeed");
    
    // Edge is logically removed
    assert_eq!(temp.storage.edge_count(), 0, "Edge count should drop to 0");
    assert_eq!(temp.storage.node_len(&0), 0, "Node's edge length should drop to 0");
    assert!(temp.storage.contains_edge(&0, &1).is_err(), "Contains should now return false (Err)");
}

#[test]
fn test_remove_edge_by_target() {
    let mut temp = TempDiskStorage::new("remove_edge_by_target");
    temp.storage.add_node().unwrap(); // node 0
    temp.storage.add_node().unwrap(); // node 1
    temp.storage.add_node().unwrap(); // node 2

    let edge1 = Edge::new(1, &42);
    let edge2 = Edge::new(2, &100);
    temp.storage.add_edge_to_node(&0, &edge1).unwrap();
    temp.storage.add_edge_to_node(&0, &edge2).unwrap();

    assert_eq!(temp.storage.node_len(&0), 2);

    temp.storage.remove_edge_by_target(&0, &1).unwrap();

    assert_eq!(temp.storage.node_len(&0), 1, "Node should have 1 edge after removal");
    assert!(temp.storage.contains_edge(&0, &1).is_err(), "Removed edge should no longer exist");
    assert!(temp.storage.contains_edge(&0, &2).is_ok(), "Other edge should still exist");
}

#[test]
fn test_clear_node_edges() {
    let mut temp = TempDiskStorage::new("clear_node_edges");
    temp.storage.add_node().unwrap(); // node 0
    temp.storage.add_node().unwrap(); // node 1
    temp.storage.add_node().unwrap(); // node 2

    let edge1 = Edge::new(1, &42);
    let edge2 = Edge::new(2, &100);
    
    temp.storage.add_edge_to_node(&0, &edge1).unwrap();
    temp.storage.add_edge_to_node(&0, &edge2).unwrap();
    
    assert_eq!(temp.storage.node_len(&0), 2);

    temp.storage.clear_node_edges(&0).unwrap();

    assert_eq!(temp.storage.node_len(&0), 0, "Cleared node should have 0 edges");
    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert!(edges.is_empty(), "Cleared node should yield 0 edges on iteration");
}

#[test]
fn test_add_reverse_edge() {
    let mut temp = TempDiskStorage::new("add_reverse_edge");
    temp.storage.add_node().unwrap(); // node 0
    temp.storage.add_node().unwrap(); // node 1

    let edge = Edge::new(1, &42);
    temp.storage.add_edge_to_node(&0, &edge).unwrap();
    temp.storage.add_reverse_edge(&1, &0).unwrap();

    let reverse = temp.storage.get_reverse_edges(&1);
    assert_eq!(reverse.len(), 1, "Should have 1 reverse edge");
    assert_eq!(reverse[0], 0, "Reverse edge should point back to source");
}

#[test]
fn test_get_reverse_edges() {
    let mut temp = TempDiskStorage::new("get_reverse_edges");
    temp.storage.add_node().unwrap(); // node 0
    temp.storage.add_node().unwrap(); // node 1
    temp.storage.add_node().unwrap(); // node 2
    temp.storage.add_node().unwrap(); // node 3

    let edge1 = Edge::new(2, &42);
    let edge2 = Edge::new(2, &100);
    let edge3 = Edge::new(2, &100);
    temp.storage.add_edge_to_node(&0, &edge1).unwrap();
    temp.storage.add_edge_to_node(&1, &edge2).unwrap();
    temp.storage.add_edge_to_node(&3, &edge3).unwrap();

    temp.storage.add_reverse_edge(&2, &0).unwrap();
    temp.storage.add_reverse_edge(&2, &1).unwrap();
    temp.storage.add_reverse_edge(&2, &3).unwrap();

    let reverse = temp.storage.get_reverse_edges(&2);
    println!("{:?}", reverse);
    assert_eq!(reverse.len(), 3, "Node 2 should have 2 reverse edges");
    assert!(reverse.contains(&0), "Reverse edges should contain node 0");
    assert!(reverse.contains(&3), "Reverse edges should contain node 3");
    assert!(reverse.contains(&1), "Reverse edges should contain node 1");
}


// ── Capacity overflow / reallocation tests ──────────────────────────────

/// reverse_capacity starts at 1024 bytes.  Each reverse edge is a u64 (8 bytes),
/// so the block can hold 1024 / 8 = 128 entries before it must reallocate.
/// This test adds exactly 129 reverse edges and verifies all survive.
#[test]
fn test_reverse_edge_reallocation_single() {
    let mut temp = TempDiskStorage::new("rev_realloc_single");

    // Node 0 is the target that will receive many incoming reverse edges.
    // Nodes 1..=129 are the sources.
    let total_sources = 129u64;
    for _ in 0..=total_sources {
        temp.storage.add_node().unwrap();
    }

    // Add 129 reverse edges to node 0 — this forces exactly one reallocation.
    for src in 1..=total_sources {
        temp.storage.add_reverse_edge(&0, &src).unwrap();
    }

    let reverse = temp.storage.get_reverse_edges(&0);
    assert_eq!(
        reverse.len(),
        total_sources as usize,
        "All {} reverse edges must be present after reallocation",
        total_sources
    );

    // Verify every source ID is present.
    for src in 1..=total_sources {
        assert!(
            reverse.contains(&src),
            "Reverse edge from source {} should survive reallocation",
            src
        );
    }
}

/// Forces two consecutive reallocations on reverse edges.
/// After first realloc: capacity = 2048 → holds 256 entries.
/// Pushing to 257 entries triggers the second realloc (capacity = 4096).
#[test]
fn test_reverse_edge_reallocation_double() {
    let mut temp = TempDiskStorage::new("rev_realloc_double");

    let total_sources = 257u64;
    for _ in 0..=total_sources {
        temp.storage.add_node().unwrap();
    }

    for src in 1..=total_sources {
        temp.storage.add_reverse_edge(&0, &src).unwrap();
    }

    let reverse = temp.storage.get_reverse_edges(&0);
    assert_eq!(
        reverse.len(),
        total_sources as usize,
        "All {} reverse edges must be present after two reallocations",
        total_sources
    );

    for src in 1..=total_sources {
        assert!(
            reverse.contains(&src),
            "Reverse edge from source {} should survive double reallocation",
            src
        );
    }
}

/// After reallocation, swap-remove must still work correctly on the new block.
#[test]
fn test_remove_reverse_edge_after_reallocation() {
    let mut temp = TempDiskStorage::new("rev_remove_after_realloc");

    let total_sources = 130u64; // triggers realloc at 129
    for _ in 0..=total_sources {
        temp.storage.add_node().unwrap();
    }

    for src in 1..=total_sources {
        temp.storage.add_reverse_edge(&0, &src).unwrap();
    }

    // Remove a few specific reverse edges.
    temp.storage.remove_reverse_edge(&0, &1).unwrap();   // remove first added
    temp.storage.remove_reverse_edge(&0, &65).unwrap();  // remove one in the middle
    temp.storage.remove_reverse_edge(&0, &130).unwrap(); // remove last added

    let reverse = temp.storage.get_reverse_edges(&0);
    assert_eq!(
        reverse.len(),
        (total_sources - 3) as usize,
        "Count should reflect 3 removals"
    );
    assert!(!reverse.contains(&1),   "Source 1 should be removed");
    assert!(!reverse.contains(&65),  "Source 65 should be removed");
    assert!(!reverse.contains(&130), "Source 130 should be removed");

    // A surviving edge should still be readable.
    assert!(reverse.contains(&2),   "Source 2 should still exist");
    assert!(reverse.contains(&100), "Source 100 should still exist");
}

/// capacity starts at 1024 bytes. Each DiskEdge is 24 bytes (3 × u64),
/// so the block holds floor(1024 / 24) = 42 edges before the 43rd triggers reallocation.
#[test]
fn test_forward_edge_reallocation() {
    let mut temp = TempDiskStorage::new("fwd_realloc");

    // Node 0 = source, nodes 1..=50 = targets.
    let total_targets = 50u64;
    for _ in 0..=total_targets {
        temp.storage.add_node().unwrap();
    }

    for target in 1..=total_targets {
        let edge = Edge::new(target, &(target as u32));
        temp.storage.add_edge_to_node(&0, &edge).unwrap();
    }

    // Verify counts.
    assert_eq!(
        temp.storage.node_len(&0),
        total_targets as usize,
        "Node 0 should report {} edges after reallocation",
        total_targets
    );
    assert_eq!(
        temp.storage.edge_count(),
        total_targets as usize,
        "Global edge count should match"
    );

    // Verify each edge is readable with correct weight.
    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert_eq!(edges.len(), total_targets as usize);

    for target in 1..=total_targets {
        assert!(
            edges.iter().any(|e| e.get_target() == target && e.get_weight() == target as u32),
            "Edge to target {} with weight {} should exist after reallocation",
            target,
            target
        );
    }
}

/// After forward-edge reallocation, swap-remove must still work on the new block.
#[test]
fn test_remove_forward_edge_after_reallocation() {
    let mut temp = TempDiskStorage::new("fwd_remove_after_realloc");

    let total_targets = 50u64;
    for _ in 0..=total_targets {
        temp.storage.add_node().unwrap();
    }

    for target in 1..=total_targets {
        let edge = Edge::new(target, &(target as u32));
        temp.storage.add_edge_to_node(&0, &edge).unwrap();
    }

    // Remove a few edges by target.
    temp.storage.remove_edge_by_target(&0, &1).unwrap();
    temp.storage.remove_edge_by_target(&0, &25).unwrap();
    temp.storage.remove_edge_by_target(&0, &50).unwrap();

    assert_eq!(
        temp.storage.node_len(&0),
        (total_targets - 3) as usize,
        "Edge count should reflect 3 removals"
    );

    assert!(temp.storage.contains_edge(&0, &1).is_err(),  "Edge to 1 should be removed");
    assert!(temp.storage.contains_edge(&0, &25).is_err(), "Edge to 25 should be removed");
    assert!(temp.storage.contains_edge(&0, &50).is_err(), "Edge to 50 should be removed");
    assert!(temp.storage.contains_edge(&0, &2).is_ok(),   "Edge to 2 should still exist");
    assert!(temp.storage.contains_edge(&0, &30).is_ok(),  "Edge to 30 should still exist");
}

/// Multiple nodes each independently fill and reallocate their reverse edge blocks,
/// ensuring the free-block pointer stays consistent and blocks don't overlap.
#[test]
fn test_multiple_nodes_reverse_reallocation() {
    let mut temp = TempDiskStorage::new("multi_node_rev_realloc");

    // Nodes 0, 1, 2 each receive 129 reverse edges (each triggers one realloc).
    // Source nodes are 3..=3+129*3-1.
    let edges_per_node = 129u64;
    let target_nodes = 3u64;
    let total_nodes = target_nodes + edges_per_node * target_nodes;
    for _ in 0..total_nodes {
        temp.storage.add_node().unwrap();
    }

    let mut src = target_nodes;
    for target in 0..target_nodes {
        for _ in 0..edges_per_node {
            temp.storage.add_reverse_edge(&target, &src).unwrap();
            src += 1;
        }
    }

    // Verify each target node's reverse edges are intact and non-overlapping.
    for target in 0..target_nodes {
        let reverse = temp.storage.get_reverse_edges(&target);
        assert_eq!(
            reverse.len(),
            edges_per_node as usize,
            "Node {} should have {} reverse edges",
            target,
            edges_per_node
        );
        // Verify no duplicates.
        let mut sorted = reverse.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            edges_per_node as usize,
            "Node {} should have no duplicate reverse edges",
            target
        );
    }
}

/// Verifies that forward and reverse edge blocks are independently managed.
/// One node gets many forward edges (triggers forward realloc) while another
/// gets many reverse edges (triggers reverse realloc). Neither should corrupt the other.
#[test]
fn test_independent_forward_and_reverse_reallocation() {
    let mut temp = TempDiskStorage::new("independent_realloc");

    // We need: node 0 (forward source), node 1 (reverse target),
    // plus enough nodes to serve as forward targets and reverse sources.
    let forward_count = 50u64;  // triggers forward realloc at 43
    let reverse_count = 130u64; // triggers reverse realloc at 129
    let total = 2 + forward_count.max(reverse_count);
    for _ in 0..total {
        temp.storage.add_node().unwrap();
    }

    // Node 0: add many forward edges.
    for target in 2..2 + forward_count {
        let edge = Edge::new(target, &(target as u32));
        temp.storage.add_edge_to_node(&0, &edge).unwrap();
    }

    // Node 1: add many reverse edges.
    for src in 2..2 + reverse_count {
        temp.storage.add_reverse_edge(&1, &src).unwrap();
    }

    // Verify forward edges on node 0.
    let fwd_edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert_eq!(fwd_edges.len(), forward_count as usize);
    for target in 2..2 + forward_count {
        assert!(
            fwd_edges.iter().any(|e| e.get_target() == target),
            "Forward edge to {} should exist",
            target
        );
    }

    // Verify reverse edges on node 1.
    let rev_edges = temp.storage.get_reverse_edges(&1);
    assert_eq!(rev_edges.len(), reverse_count as usize);
    for src in 2..2 + reverse_count {
        assert!(
            rev_edges.contains(&src),
            "Reverse edge from {} should exist",
            src
        );
    }
}

// ── Edge Case Tests for Disk Resize ─────────────────────────────────────

#[test]
fn test_empty_node_iteration() {
    let mut temp = TempDiskStorage::new("empty_node_iter");
    temp.storage.add_node().unwrap(); // node 0

    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert!(edges.is_empty(), "Node with no edges should yield empty iterator");
    assert_eq!(temp.storage.node_len(&0), 0, "Node with no edges should have length 0");
}

/// Initial capacity is 256 bytes. DiskEdge is 24 bytes.
/// 256 / 24 = 10 edges fit. The 11th triggers reallocation.
#[test]
fn test_boundary_capacity_forward_edges() {
    let mut temp = TempDiskStorage::new("boundary_fwd");

    // Node 0 = source, nodes 1..=11 = targets
    for _ in 0..12 {
        temp.storage.add_node().unwrap();
    }

    // Add exactly 10 edges (should fit without realloc)
    for target in 1..=10 {
        let edge = Edge::new(target, &(target as u32));
        temp.storage.add_edge_to_node(&0, &edge).unwrap();
    }
    assert_eq!(temp.storage.node_len(&0), 10);

    // Verify all 10 are readable
    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert_eq!(edges.len(), 10);
    for target in 1..=10u64 {
        assert!(
            edges.iter().any(|e| e.get_target() == target && e.get_weight() == target as u32),
            "Edge to {} should exist before realloc",
            target
        );
    }

    // Add 11th edge (triggers realloc)
    let edge = Edge::new(11, &11u32);
    temp.storage.add_edge_to_node(&0, &edge).unwrap();

    assert_eq!(temp.storage.node_len(&0), 11);
    assert_eq!(temp.storage.edge_count(), 11);

    // All 11 should survive
    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert_eq!(edges.len(), 11);
    for target in 1..=11u64 {
        assert!(
            edges.iter().any(|e| e.get_target() == target && e.get_weight() == target as u32),
            "Edge to {} should survive realloc",
            target
        );
    }
}

/// Initial reverse capacity is 256 bytes. Each reverse entry is 8 bytes (u64).
/// 256 / 8 = 32 entries fit. The 33rd triggers reallocation.
#[test]
fn test_boundary_capacity_reverse_edges() {
    let mut temp = TempDiskStorage::new("boundary_rev");

    // Node 0 = target, nodes 1..=33 = sources
    for _ in 0..34 {
        temp.storage.add_node().unwrap();
    }

    // Add exactly 32 reverse edges (should fit without realloc)
    for src in 1..=32 {
        temp.storage.add_reverse_edge(&0, &src).unwrap();
    }
    let reverse = temp.storage.get_reverse_edges(&0);
    assert_eq!(reverse.len(), 32, "32 reverse edges should fit in initial capacity");
    for src in 1..=32u64 {
        assert!(reverse.contains(&src), "Reverse edge from {} should exist", src);
    }

    // Add 33rd (triggers realloc)
    temp.storage.add_reverse_edge(&0, &33).unwrap();
    let reverse = temp.storage.get_reverse_edges(&0);
    assert_eq!(reverse.len(), 33, "33rd reverse edge should survive realloc");
    for src in 1..=33u64 {
        assert!(reverse.contains(&src), "Reverse edge from {} should survive realloc", src);
    }
}

#[test]
fn test_many_nodes_with_no_edges() {
    let mut temp = TempDiskStorage::new("many_empty_nodes");

    for _ in 0..100 {
        temp.storage.add_node().unwrap();
    }

    assert_eq!(temp.storage.node_count(), 100);
    assert_eq!(temp.storage.edge_count(), 0);

    for node in 0..100u64 {
        assert_eq!(temp.storage.node_len(&node), 0, "Node {} should have 0 edges", node);
        let edges: Vec<_> = temp.storage.get_edges(&node).collect();
        assert!(edges.is_empty(), "Node {} should yield empty iterator", node);
    }
}

#[test]
fn test_single_node_self_edge() {
    let mut temp = TempDiskStorage::new("self_edge");
    temp.storage.add_node().unwrap(); // node 0

    let edge = Edge::new(0, &99u32);
    temp.storage.add_edge_to_node(&0, &edge).unwrap();

    assert_eq!(temp.storage.node_len(&0), 1);
    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].get_target(), 0);
    assert_eq!(edges[0].get_weight(), 99);

    // Remove self-loop
    temp.storage.remove_edge_by_target(&0, &0).unwrap();
    assert_eq!(temp.storage.node_len(&0), 0);
}

// ── Stress/Correctness After Multiple Removes ───────────────────────────

#[test]
fn test_add_remove_add_edges_repeatedly() {
    let mut temp = TempDiskStorage::new("add_remove_readd");

    for _ in 0..5 {
        temp.storage.add_node().unwrap();
    }

    // Add edges: 0->1, 0->2, 0->3, 0->4
    for target in 1..5u64 {
        let edge = Edge::new(target, &(target as u32 * 10));
        temp.storage.add_edge_to_node(&0, &edge).unwrap();
    }
    assert_eq!(temp.storage.node_len(&0), 4);

    // Remove edge 0->2
    temp.storage.remove_edge_by_target(&0, &2).unwrap();
    assert_eq!(temp.storage.node_len(&0), 3);
    assert!(temp.storage.contains_edge(&0, &2).is_err(), "Edge 0->2 should be removed");

    // Re-add edge 0->2 with different weight
    let edge = Edge::new(2, &200u32);
    temp.storage.add_edge_to_node(&0, &edge).unwrap();
    assert_eq!(temp.storage.node_len(&0), 4);

    let found = temp.storage.contains_edge(&0, &2);
    assert!(found.is_ok(), "Edge 0->2 should exist again");
}

#[test]
fn test_clear_then_readd_edges() {
    let mut temp = TempDiskStorage::new("clear_readd");

    for _ in 0..3 {
        temp.storage.add_node().unwrap();
    }

    let edge1 = Edge::new(1, &42u32);
    let edge2 = Edge::new(2, &100u32);
    temp.storage.add_edge_to_node(&0, &edge1).unwrap();
    temp.storage.add_edge_to_node(&0, &edge2).unwrap();
    assert_eq!(temp.storage.node_len(&0), 2);

    temp.storage.clear_node_edges(&0).unwrap();
    assert_eq!(temp.storage.node_len(&0), 0);

    // Re-add a new edge
    let edge3 = Edge::new(1, &999u32);
    temp.storage.add_edge_to_node(&0, &edge3).unwrap();
    assert_eq!(temp.storage.node_len(&0), 1);

    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].get_target(), 1);
    assert_eq!(edges[0].get_weight(), 999);
}

#[test]
fn test_swap_remove_preserves_other_edges() {
    let mut temp = TempDiskStorage::new("swap_remove_integrity");

    for _ in 0..6 {
        temp.storage.add_node().unwrap();
    }

    // Add edges from node 0: 0->1(10), 0->2(20), 0->3(30), 0->4(40), 0->5(50)
    for target in 1..=5u64 {
        let edge = Edge::new(target, &(target as u32 * 10));
        temp.storage.add_edge_to_node(&0, &edge).unwrap();
    }

    // Remove 0->3
    temp.storage.remove_edge_by_target(&0, &3).unwrap();
    assert_eq!(temp.storage.node_len(&0), 4);

    // Verify remaining edges contain exactly {1, 2, 4, 5} by content
    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    let mut targets: Vec<u64> = edges.iter().map(|e| e.get_target()).collect();
    targets.sort();
    assert_eq!(targets, vec![1, 2, 4, 5], "Remaining targets should be 1, 2, 4, 5");

    // Verify weights are correct
    for edge in &edges {
        let expected_weight = edge.get_target() as u32 * 10;
        assert_eq!(edge.get_weight(), expected_weight, "Weight for target {} should be {}", edge.get_target(), expected_weight);
    }

    // Remove 0->1
    temp.storage.remove_edge_by_target(&0, &1).unwrap();
    assert_eq!(temp.storage.node_len(&0), 3);

    let edges: Vec<_> = temp.storage.get_edges(&0).collect();
    let mut targets: Vec<u64> = edges.iter().map(|e| e.get_target()).collect();
    targets.sort();
    assert_eq!(targets, vec![2, 4, 5], "Remaining targets should be 2, 4, 5");
}

#[test]
fn test_remove_reverse_edge_preserves_others() {
    let mut temp = TempDiskStorage::new("rev_remove_integrity");

    for _ in 0..5 {
        temp.storage.add_node().unwrap();
    }

    // Add reverse edges to node 0 from: 1, 2, 3, 4
    for src in 1..=4u64 {
        temp.storage.add_reverse_edge(&0, &src).unwrap();
    }

    // Remove reverse edge from 2
    temp.storage.remove_reverse_edge(&0, &2).unwrap();
    let reverse = temp.storage.get_reverse_edges(&0);
    assert_eq!(reverse.len(), 3);
    let mut sorted = reverse.clone();
    sorted.sort();
    assert_eq!(sorted, vec![1, 3, 4], "After removing 2, should have 1, 3, 4");

    // Remove reverse edge from 4
    temp.storage.remove_reverse_edge(&0, &4).unwrap();
    let reverse = temp.storage.get_reverse_edges(&0);
    assert_eq!(reverse.len(), 2);
    let mut sorted = reverse.clone();
    sorted.sort();
    assert_eq!(sorted, vec![1, 3], "After removing 4, should have 1, 3");
}

