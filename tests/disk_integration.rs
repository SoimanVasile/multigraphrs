use multigraphrs::{Directed, DiskStorage, MultiGraph, Undirected, Weighted, WeightedDirected};
use std::env;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_test_id() -> usize {
    TEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

struct TempGraph<S, W>
where
    W: Clone + std::cmp::PartialEq + multigraphrs::storage::disk_storage::from_disk_bytes::FromDiskBytes + multigraphrs::storage::disk_storage::from_disk_bytes::AsDiskBytes,
    S: multigraphrs::DirectionStrategy<u32, W>,
{
    pub graph: MultiGraph<u32, W, S, DiskStorage<u32, W>>,
    dir: std::path::PathBuf,
}

impl<S, W> TempGraph<S, W>
where
    W: Clone + std::cmp::PartialEq + multigraphrs::storage::disk_storage::from_disk_bytes::FromDiskBytes + multigraphrs::storage::disk_storage::from_disk_bytes::AsDiskBytes,
    S: multigraphrs::DirectionStrategy<u32, W>,
{
    fn new(test_name: &str) -> Self {
        let id = next_test_id();
        let mut dir = env::temp_dir();
        dir.push(format!("multigraphrs_integration_{}_{}", test_name, id));
        let _ = fs::remove_dir_all(&dir); // Clean up if exists from previous run
        fs::create_dir_all(&dir).unwrap();

        let backend = DiskStorage::<u32, W>::new(&dir);
        let graph = MultiGraph::with_backend(backend);

        Self { graph, dir }
    }
}

impl<S, W> Drop for TempGraph<S, W>
where
    W: Clone + std::cmp::PartialEq + multigraphrs::storage::disk_storage::from_disk_bytes::FromDiskBytes + multigraphrs::storage::disk_storage::from_disk_bytes::AsDiskBytes,
    S: multigraphrs::DirectionStrategy<u32, W>,
{
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// A comprehensive user workflow test simulating real-world usage on a disk graph.
/// We use Directed graph which exercises both forward and reverse edges.
#[test]
fn test_end_to_end_directed_user_workflow() {
    // 1. Initialize graph
    let mut temp = TempGraph::<Directed, u32>::new("directed_workflow");
    
    // 2. Add some initial nodes
    for i in 1..=10 {
        temp.graph.add_node(i).unwrap();
    }
    assert_eq!(temp.graph.node_count(), 10);

    // 3. Add edges creating a star topology (1 connects to 2..10)
    for target in 2..=10 {
        temp.graph.add_edge(1, target).unwrap();
    }

    // Node 1 should have degree 9 (outgoing)
    assert_eq!(temp.graph.degree(&1).unwrap(), 9);
    
    // 4. Force reallocation of forward edges on node 1
    // A forward edge block capacity is initially 1024.
    // Each DiskEdge is 24 bytes, meaning ~42 edges per block before realloc.
    // Let's add 50 more nodes and edges from node 1 to them.
    for i in 11..=60 {
        temp.graph.add_node(i).unwrap();
        temp.graph.add_edge(1, i).unwrap();
    }

    assert_eq!(temp.graph.degree(&1).unwrap(), 59);

    // 5. Force reallocation of reverse edges on a single target
    // A reverse edge is 8 bytes, so capacity 1024 holds 128 edges before realloc.
    // Node 100 will be the target of 150 incoming edges.
    temp.graph.add_node(100).unwrap();
    for i in 201..=350 {
        temp.graph.add_node(i).unwrap();
        temp.graph.add_edge(i, 100).unwrap(); // Incoming to 100
    }

    // Verify reverse edge iterators are working on the disk backend
    // Since we are using Directed strategy, removing node 100 will walk its
    // reverse edges to clean up the outgoing edges from 201..=350.
    
    // 6. Test node removal (which relies heavily on correct forward/reverse edges)
    // Remove node 100
    temp.graph.remove_node(&100).unwrap();
    assert!(temp.graph.degree(&100).is_err());

    // Verify sources pointing to 100 no longer have that edge
    for i in 201..=350 {
        assert_eq!(temp.graph.degree(&i).unwrap(), 0, "Source node should have its outgoing edge removed");
    }

    // 7. Verify swap-remove on forward edges works transparently
    // Node 1 currently points to 2..=60
    assert_eq!(temp.graph.degree(&1).unwrap(), 59);
    
    // Remove edge to 10
    temp.graph.remove_edge(1, 10).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 58);
    
    // Verify edge is gone
    let neighbors: Vec<_> = temp.graph.get_neighbours(&1).unwrap().iter().map(|e| *e.get_target()).collect();
    assert!(!neighbors.contains(&10));
    assert!(neighbors.contains(&11));

    // 8. Re-add removed node and verify it's clean
    temp.graph.remove_node(&10).unwrap();
    temp.graph.add_node(10).unwrap();
    assert_eq!(temp.graph.degree(&10).unwrap(), 0);

    // 9. Add dense graph connections to simulate heavy load
    for i in 1..=5 {
        for j in 1..=5 {
            if i != j {
                let _ = temp.graph.add_edge(i, j); // Ignore duplicates/errors
            }
        }
    }
    
    // Verify some state
    assert!(temp.graph.degree(&2).unwrap() >= 4);
    
    // 10. Remove a heavily connected node
    temp.graph.remove_node(&1).unwrap();
    // Node 2 should no longer have incoming from 1 or outgoing to 1
    let neighbors_of_2: Vec<_> = temp.graph.get_neighbours(&2).unwrap().iter().map(|e| *e.get_target()).collect();
    assert!(!neighbors_of_2.contains(&1));
}

/// End to end test for Undirected graphs
#[test]
fn test_end_to_end_undirected_user_workflow() {
    let mut temp = TempGraph::<Undirected, u32>::new("undirected_workflow");
    
    temp.graph.add_node(1).unwrap();
    temp.graph.add_node(2).unwrap();
    temp.graph.add_node(3).unwrap();
    
    // Undirected edge 1 <-> 2
    temp.graph.add_edge(1, 2).unwrap();
    
    assert_eq!(temp.graph.degree(&1).unwrap(), 1);
    assert_eq!(temp.graph.degree(&2).unwrap(), 1);
    
    // Force many undirected edges
    let total_nodes = 100;
    for i in 4..=total_nodes {
        temp.graph.add_node(i).unwrap();
        temp.graph.add_edge(1, i).unwrap();
    }
    
    assert_eq!(temp.graph.degree(&1).unwrap(), 98); // 2..100 = 98 edges
    
    // Removing an edge in undirected cleans both ways
    temp.graph.remove_edge(1, 50).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 97);
    assert_eq!(temp.graph.degree(&50).unwrap(), 0);
    
    // Remove a node
    temp.graph.remove_node(&2).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 96);
}

/// End to end test for WeightedDirected graphs
#[test]
fn test_end_to_end_weighted_directed_user_workflow() {
    let mut temp = TempGraph::<WeightedDirected, f64>::new("weighted_directed_workflow");
    
    temp.graph.add_node(1).unwrap();
    temp.graph.add_node(2).unwrap();
    
    // Edge with weight
    temp.graph.add_edge(1, 2, 42.5).unwrap();
    temp.graph.add_edge(1, 2, 99.9).unwrap(); // parallel edge with different weight
    
    assert_eq!(temp.graph.degree(&1).unwrap(), 2);
    
    // Removing specific weighted edge
    temp.graph.remove_edge(1, 2, 42.5).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 1);
    
    // Get the remaining edge and verify weight
    let edges: Vec<_> = temp.graph.get_neighbours(&1).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(*edges[0].get_weight(), 99.9);
}

/// End to end test for checking comprehensive edge removal using reverse edges
#[test]
fn test_end_to_end_directed_edge_removal_and_reverse_edges() {
    let mut temp = TempGraph::<Directed, u32>::new("directed_edge_removal");
    
    // Create 200 nodes
    for i in 1..=200 {
        temp.graph.add_node(i).unwrap();
    }
    
    // Node 1 receives incoming edges from nodes 2 to 150
    for src in 2..=150 {
        temp.graph.add_edge(src, 1).unwrap();
    }
    
    assert_eq!(temp.graph.node_count(), 200);
    
    // Verify Node 1 has no outgoing edges but Node 2 has 1 outgoing edge
    assert_eq!(temp.graph.degree(&1).unwrap(), 0);
    assert_eq!(temp.graph.degree(&2).unwrap(), 1);
    
    // Node 1 points to nodes 151 to 200
    for target in 151..=200 {
        temp.graph.add_edge(1, target).unwrap();
    }
    
    assert_eq!(temp.graph.degree(&1).unwrap(), 50);
    
    // Remove node 1
    // This relies heavily on the reverse edges being correct to clean up 2..=150
    temp.graph.remove_node(&1).unwrap();
    
    assert!(temp.graph.degree(&1).is_err());
    
    // Check that sources 2 to 150 no longer point to 1
    for src in 2..=150 {
        assert_eq!(temp.graph.degree(&src).unwrap(), 0, "Source node should have its outgoing edge to 1 removed");
    }
    
    // Check that nodes 151 to 200 no longer receive edges from 1
    // (We would check their reverse edges, but MultiGraph doesn't expose incoming degree directly)
    // We can just verify their degree is 0
    for target in 151..=200 {
        assert_eq!(temp.graph.degree(&target).unwrap(), 0);
    }
}

// ── Weighted Undirected Disk Tests ──────────────────────────────────────

#[test]
fn test_end_to_end_weighted_undirected_user_workflow() {
    let mut temp = TempGraph::<Weighted, f64>::new("weighted_undirected_workflow");

    temp.graph.add_node(1).unwrap();
    temp.graph.add_node(2).unwrap();
    temp.graph.add_node(3).unwrap();

    // Weighted undirected edge 1 <-> 2
    temp.graph.add_edge(1, 2, 3.5).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 1);
    assert_eq!(temp.graph.degree(&2).unwrap(), 1);

    // Weighted undirected edge 1 <-> 3
    temp.graph.add_edge(1, 3, 2.71).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 2);

    // Remove specific weighted edge
    temp.graph.remove_edge(1, 2, 3.5).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 1);
    assert_eq!(temp.graph.degree(&2).unwrap(), 0);

    // Verify remaining edge weight
    let edges = temp.graph.get_neighbours(&1).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(*edges[0].get_weight(), 2.71);
    assert_eq!(*edges[0].get_target(), 3);

    // Force reallocation by adding many edges
    for i in 4..=60 {
        temp.graph.add_node(i).unwrap();
        temp.graph.add_edge(1, i, i as f64).unwrap();
    }
    // degree = 1 (edge to 3) + 57 (edges to 4..=60) = 58
    assert_eq!(temp.graph.degree(&1).unwrap(), 58);

    // Remove a node and verify cleanup
    temp.graph.remove_node(&3).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 57);
    assert!(!temp.graph.contains_node(&3).unwrap());
}

#[test]
fn test_weighted_undirected_disk_parallel_edges() {
    let mut temp = TempGraph::<Weighted, f64>::new("weighted_undirected_parallel");

    temp.graph.add_node(1).unwrap();
    temp.graph.add_node(2).unwrap();

    // Add 3 parallel edges with different weights
    temp.graph.add_edge(1, 2, 1.0).unwrap();
    temp.graph.add_edge(1, 2, 2.0).unwrap();
    temp.graph.add_edge(1, 2, 3.0).unwrap();

    assert_eq!(temp.graph.degree(&1).unwrap(), 3);
    assert_eq!(temp.graph.degree(&2).unwrap(), 3);

    // Remove the edge with weight 2.0
    temp.graph.remove_edge(1, 2, 2.0).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 2);
    assert_eq!(temp.graph.degree(&2).unwrap(), 2);

    // Verify remaining weights are 1.0 and 3.0
    let edges = temp.graph.get_neighbours(&1).unwrap();
    let mut weights: Vec<f64> = edges.iter().map(|e| *e.get_weight()).collect();
    weights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(weights, vec![1.0, 3.0]);
}

// ── Disk-Backed Node ID Reuse Tests ────────────────────────────────────

#[test]
fn test_disk_directed_node_id_reuse() {
    let mut temp = TempGraph::<Directed, u32>::new("disk_id_reuse");

    temp.graph.add_node(1).unwrap();
    temp.graph.add_node(2).unwrap();
    temp.graph.add_node(3).unwrap();

    temp.graph.add_edge(1, 2).unwrap();
    temp.graph.add_edge(1, 3).unwrap();

    // Remove node 2, freeing its slot
    temp.graph.remove_node(&2).unwrap();

    // Add node 4, should reuse the freed slot
    temp.graph.add_node(4).unwrap();
    assert_eq!(temp.graph.node_count(), 3);

    // Edge to old node 2 should be gone, add edge to new node 4
    temp.graph.add_edge(1, 4).unwrap();
    assert!(temp.graph.contains_edge(&1, &4).unwrap());
    assert!(!temp.graph.contains_node(&2).unwrap());
    // degree of 1: edge to 3 (survived) + edge to 4 (new) = 2
    assert_eq!(temp.graph.degree(&1).unwrap(), 2);
}

#[test]
fn test_disk_directed_multiple_reuse_cycles() {
    let mut temp = TempGraph::<Directed, u32>::new("disk_multi_reuse");

    // Add 10 nodes and build a chain
    for i in 0..10 {
        temp.graph.add_node(i).unwrap();
    }
    for i in 0..9 {
        temp.graph.add_edge(i, i + 1).unwrap();
    }
    assert_eq!(temp.graph.edge_count(), 9);

    // Remove nodes 3, 5, 7 (breaks chain at those points)
    temp.graph.remove_node(&3).unwrap();
    temp.graph.remove_node(&5).unwrap();
    temp.graph.remove_node(&7).unwrap();
    assert_eq!(temp.graph.node_count(), 7);

    // Add 3 new nodes that reuse freed slots
    temp.graph.add_node(100).unwrap();
    temp.graph.add_node(200).unwrap();
    temp.graph.add_node(300).unwrap();
    assert_eq!(temp.graph.node_count(), 10);

    // Build new edges with reused nodes
    temp.graph.add_edge(2, 100).unwrap();
    temp.graph.add_edge(100, 4).unwrap();
    temp.graph.add_edge(4, 200).unwrap();
    temp.graph.add_edge(200, 6).unwrap();
    temp.graph.add_edge(6, 300).unwrap();
    temp.graph.add_edge(300, 8).unwrap();

    // Verify new edges work
    assert!(temp.graph.contains_edge(&2, &100).unwrap());
    assert!(temp.graph.contains_edge(&100, &4).unwrap());
    assert!(temp.graph.contains_edge(&300, &8).unwrap());

    // Verify old nodes are gone
    assert!(!temp.graph.contains_node(&3).unwrap());
    assert!(!temp.graph.contains_node(&5).unwrap());
    assert!(!temp.graph.contains_node(&7).unwrap());
}

#[test]
fn test_disk_remove_all_nodes_then_rebuild() {
    let mut temp = TempGraph::<Directed, u32>::new("disk_rebuild");

    // Build initial graph
    for i in 0..5 {
        temp.graph.add_node(i).unwrap();
    }
    for i in 0..4 {
        temp.graph.add_edge(i, i + 1).unwrap();
    }

    // Remove everything
    for i in 0..5 {
        temp.graph.remove_node(&i).unwrap();
    }
    assert_eq!(temp.graph.node_count(), 0);
    assert_eq!(temp.graph.edge_count(), 0);

    // Rebuild on recycled slots
    for i in 100..105 {
        temp.graph.add_node(i).unwrap();
    }
    for i in 100..104 {
        temp.graph.add_edge(i, i + 1).unwrap();
    }

    assert_eq!(temp.graph.node_count(), 5);
    assert_eq!(temp.graph.edge_count(), 4);
    assert!(temp.graph.contains_edge(&100, &101).unwrap());
    assert!(temp.graph.contains_edge(&103, &104).unwrap());
}

#[test]
fn test_disk_undirected_node_id_reuse() {
    let mut temp = TempGraph::<Undirected, u32>::new("disk_undirected_reuse");

    temp.graph.add_node(1).unwrap();
    temp.graph.add_node(2).unwrap();
    temp.graph.add_node(3).unwrap();

    temp.graph.add_edge(1, 2).unwrap();
    temp.graph.add_edge(2, 3).unwrap();

    // Remove node 2
    temp.graph.remove_node(&2).unwrap();
    assert_eq!(temp.graph.degree(&1).unwrap(), 0);
    assert_eq!(temp.graph.degree(&3).unwrap(), 0);

    // Add node 4 (reuses slot)
    temp.graph.add_node(4).unwrap();
    temp.graph.add_edge(1, 4).unwrap();

    assert_eq!(temp.graph.degree(&1).unwrap(), 1);
    assert_eq!(temp.graph.degree(&4).unwrap(), 1);
    assert!(temp.graph.contains_edge(&1, &4).unwrap());
    assert!(temp.graph.contains_edge(&4, &1).unwrap());
    assert!(!temp.graph.contains_node(&2).unwrap());
}

#[test]
fn test_allocator_large_block_split_edge_case() {
    let mut temp = TempGraph::<Directed, u32>::new("alloc_split");
    temp.graph.add_node(1).unwrap();
    temp.graph.add_node(2).unwrap();

    // Node 1 resizes from 128 -> 256 -> 512 -> 1024
    for i in 100..122 {
        temp.graph.add_node(i).unwrap();
        temp.graph.add_edge(1, i).unwrap();
    }

    // Node 1 is removed. Its 1024-byte block goes to bucket 3.
    temp.graph.remove_node(&1).unwrap();

    // Now adding new nodes and edges, which requires 128-byte blocks.
    temp.graph.add_node(3).unwrap();
    temp.graph.add_edge(2, 3).unwrap();
    
    // Add more to trigger subsequent splits or allocations
    for i in 200..230 {
        temp.graph.add_node(i).unwrap();
        temp.graph.add_edge(2, i).unwrap();
    }

    assert_eq!(temp.graph.degree(&2).unwrap(), 31);
}
