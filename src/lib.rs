//! # MultiGraphRs
//!
//! A strategy-pattern based multigraph library for Rust. One generic [`MultiGraph<K, W, S, B>`]
//! struct adapts its behavior at compile time — directed, undirected, weighted, or unweighted —
//! just by swapping the strategy type parameter `S`.
//!
//! # Quick Start
//!
//! ```rust
//! use multigraphrs::{RamMultiGraph, Directed};
//!
//! let mut graph = RamMultiGraph::<String, u32, Directed>::new();
//!
//! graph.add_node("Berlin".to_string()).unwrap();
//! graph.add_node("Paris".to_string()).unwrap();
//!
//! let edge = graph.add_edge("Berlin".to_string(), "Paris".to_string()).unwrap();
//! assert_eq!(edge.get_target(), &"Paris".to_string());
//! assert_eq!(*edge.get_weight(), 1);
//!
//! // Multigraph: parallel edges between the same pair are allowed
//! graph.add_edge("Berlin".to_string(), "Paris".to_string()).unwrap();
//! assert_eq!(graph.degree(&"Berlin".to_string()), Ok(2));
//! ```
//!
//! # Strategies
//!
//! | Strategy | Directed | Weighted | `add_edge` signature |
//! | :--- | :---: | :---: | :--- |
//! | [`Directed`] | ✅ | ❌ (default `1u32`) | `(source, target)` |
//! | [`Undirected`] | ❌ | ❌ (default `1u32`) | `(source, target)` |
//! | [`WeightedDirected`] | ✅ | ✅ | `(source, target, weight)` |
//! | [`Weighted`] | ❌ | ✅ | `(source, target, weight)` |

pub mod core;
pub mod storage;
pub mod strategies;
pub mod dictionary;

// Expose the internal types publicly so users can import them easily
pub use strategies::direction_strategy::DirectionStrategy;
pub use strategies::directed::Directed;
pub use strategies::undirected::Undirected;
pub use strategies::weighted::Weighted;
pub use strategies::weighted_directed::WeightedDirected;

pub use core::graph_errors::GraphError;
pub use core::multigraph_iterator::{self, NodeIter};
pub use core::edge::EdgeView;
use crate::storage::disk_storage::from_disk_bytes::FromDiskBytes;
use crate::storage::disk_storage::from_disk_bytes::AsDiskBytes;
use core::edge::Edge;

pub use storage::disk_storage::disk_multigraph::DiskStorage;
use storage::adjacency_list::RamStorage;
use storage::storage_backend::StorageBackend;

use std::{hash::Hash, marker::PhantomData};
use ahash::{AHashSet};

const MAX_CAPACITY_BULK: usize = 131_072; // The max size of a buffer for a bulk operation

/// A multigraph that stores nodes of type `K` connected by edges carrying weights of type `W`.
///
/// The behavior of edge insertion and removal (directed vs. undirected, weighted vs. unweighted)
/// is determined at compile time by the strategy `S`. The storage backend `B` controls whether
/// data lives in RAM ([`RamStorage`]) or on disk ([`DiskStorage`]).
///
/// # Type Parameters
/// * `K` — Node key type. Must be `Eq + Hash + Clone`.
/// * `W` — Edge weight type. Must be `Clone + PartialEq`.
/// * `S` — Direction strategy (e.g. [`Directed`], [`Weighted`]).
/// * `B` — Storage backend (e.g. [`RamStorage`], [`DiskStorage`]).
pub struct MultiGraph<K, W, S: DirectionStrategy<K, W>, B: StorageBackend<K, W>>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
{
    /// The internal adjacency list mapping a node to its outgoing edges.
    pub(crate) adjacency_list: B,
    pub(crate) node_count: usize,
    /// Marker to keep track of the specific strategy `S`, node 'K' and weight `W`.
    _marker: PhantomData<(S, K, W)>,
}

pub type RamMultiGraph<K, W, Dir> = MultiGraph<K, W, Dir, RamStorage<K, W>>;
pub type DiskMultiGraph<K, W, Dir> = MultiGraph<K, W, Dir, DiskStorage<K, W>>;

// --- Core Methods Shared by ALL Graph Types ---

impl<K, W, S, B> MultiGraph<K, W, S, B>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    S: DirectionStrategy<K, W>,
    B: StorageBackend<K, W>,
    GraphError: From<B::Error>,
{
    /// Creates an empty [`MultiGraph`] backed by the given storage.
    ///
    /// This is the universal constructor — strategy-specific `new()` helpers
    /// call this with a default backend. Use this directly when you need a
    /// custom backend like [`DiskStorage`].
    pub fn with_backend(backend: B) -> Self {
        let node_count = backend.node_count().unwrap_or(0);
        MultiGraph {
            adjacency_list: backend,
            node_count,
            _marker: PhantomData,
        }
    }

    /// Inserts a single, disconnected node into the graph.
    ///
    /// Returns the inserted key on success. If a node with the same key already
    /// exists, the graph is unchanged.
    ///
    /// # Arguments
    /// * `source` — The node key to insert.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeAlreadyExists`] if the key is already present.
    pub fn add_node(&mut self, source: K) -> Result<K, GraphError> {
        if self.adjacency_list.hashed_nodes_contains_key(&source)? {
            return Err(GraphError::NodeAlreadyExists);
        }
        let node_id = self.adjacency_list.add_node()?;
        self.node_count += 1;
        
        self.adjacency_list.hashed_nodes_insert(source.clone(), node_id)?;
        Ok(source)

    }

    fn process_batch (&mut self, nodes: &mut Vec<K>) -> Result<(), GraphError> {
        if nodes.is_empty() {
            return Ok(());
        }
        let adjacency_list = &mut self.adjacency_list;
        let nodes_id = adjacency_list.bulk_add_node(&(nodes.len() as u64))?;

        self.node_count += nodes.len();

        let mut data_id: Vec<(K, u64)> = Vec::with_capacity(nodes.len());
        for (data, id) in nodes.iter().zip(nodes_id.iter()){
            data_id.push((data.clone(), *id));
        }
        self.adjacency_list.hashed_nodes_bulk_insert(&data_id)?;
        nodes.clear();
        Ok(())
    }
    /// Inserts multiple disconnected nodes in bulk, silently skipping duplicates.
    ///
    /// Nodes that already exist in the graph (or appear more than once in `sources`)
    /// are skipped rather than causing an error. Processing is batched internally
    /// in chunks of up to 131,072 for efficiency.
    ///
    /// # Panics
    /// Panics if internal node IDs returned by the backend cannot be mapped correctly
    /// (indicates a bug in the storage backend).
    pub fn bulk_add_node(&mut self, sources: &[K]) -> Result<(), GraphError>{
        if sources.is_empty() {
            return Ok(());
        }

        let mut non_existing_nodes: Vec<K> = Vec::with_capacity(MAX_CAPACITY_BULK);
        let mut seen_in_batch: AHashSet<K> = AHashSet::new();


        for source in sources{
            if self.adjacency_list.hashed_nodes_contains_key(source)? || !seen_in_batch.insert(source.clone()) {
                continue
            }

            if non_existing_nodes.len() >= MAX_CAPACITY_BULK{
                self.process_batch(&mut non_existing_nodes)?;
            }

            non_existing_nodes.push(source.clone());
        }

        seen_in_batch.clear();

        self.process_batch(&mut non_existing_nodes)?;
        
        Ok(())
    }

    /// Removes a node and **all** edges connected to it from the graph.
    ///
    /// The strategy `S` determines how incident edges are cleaned up: directed
    /// strategies use the reverse adjacency index, undirected strategies walk
    /// the outgoing neighbour list. The freed internal ID is recycled and may
    /// be reused by future [`add_node`](Self::add_node) calls.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if `source` is not present.
    ///
    /// # Panics
    /// Panics if the internal reverse-lookup vector is out of sync with the
    /// hash map — this indicates a bug in the library, not user error.
    pub fn remove_node(&mut self, source: &K) -> Result<(), GraphError> {
        let index = match self.adjacency_list.hashed_nodes_remove(source)? {
            Some(idx) => idx,
            None => return Err(GraphError::NodeNotFound),
        };

        S::remove_node(&mut self.adjacency_list, index)?;
        self.node_count -= 1;

        Ok(())
    }

    /// Returns the degree (number of outgoing edges) of the given node.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if the node is not in the graph.
    pub fn degree(&self, source: &K) -> Result<usize, GraphError>{
        match self.adjacency_list.hashed_nodes_get(source)?{
            Some(n) => Ok(self.adjacency_list.node_len(&n)?),
            None => Err(GraphError::NodeNotFound),
        }
    }
    /// Returns all outgoing edges of the given node as a `Vec<EdgeView<K, W>>`.
    ///
    /// Each [`EdgeView`] is an independent clone — mutating it does **not**
    /// affect the graph.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if the node is not in the graph.
    ///
    /// # Panics
    /// Panics if any edge targets a node whose reverse-lookup entry is `None`
    /// (internal inconsistency).
    pub fn get_neighbours(&self, source: &K) -> Result<Vec<EdgeView<K, W>>, GraphError>{
        let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let neighbours = self.adjacency_list.get_edges(&source_hashed);

        Ok(neighbours.map(|edge| {
            EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight())
        }).collect())
    }

    /// Returns `true` if a node with the given key exists in the graph.
    pub fn contains_node(&self, key: &K) -> Result<bool, GraphError>{
        Ok(self.adjacency_list.hashed_nodes_contains_key(key)?)
    }

    /// Returns the total number of nodes currently in the graph.
    pub fn node_count(&self) -> usize{
        self.node_count
    }

    /// Returns the total number of edges currently in the graph.
    ///
    /// For undirected strategies each logical connection is counted as **two**
    /// internal edges (one per direction).
    pub fn edge_count(&self) -> usize{
        self.adjacency_list.edge_count().unwrap_or(0)
    }

    /// Returns an iterator over all nodes and their edges.
    ///
    /// Each item is `(&K, Vec<EdgeView<K, W>>)`. Tombstoned (removed) slots
    /// are automatically skipped.
    ///
    /// # Panics
    /// Panics if the internal reverse-lookup vector is inconsistent (library bug).
    pub fn iter(&self) -> multigraph_iterator::NodeIter<'_, K, W, S, B> {
        multigraph_iterator::NodeIter { graph: self, index: 0, number_of_nodes: self.node_count() as u64}
    }

    /// Returns `true` if at least one edge from `source` to `target` exists.
    ///
    /// Returns `false` (rather than an error) if either node does not exist.
    pub fn contains_edge(&self, source: &K, target: &K) -> Result<bool, GraphError>{

        let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
            Some(t) => t,
            None => return Ok(false),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(target)?{
            Some(t) => t,
            None => return Ok(false),
        };

        Ok(self.adjacency_list.contains_edge(&source_hashed, &target_hashed).is_ok())
    }
}

// --- Strategy-Specific Implementations ---

impl<K, W> MultiGraph<K, W, Weighted, RamStorage<K, W>>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
{
    /// Creates a new, empty weighted undirected graph backed by RAM.
    pub fn new() -> MultiGraph<K, W, Weighted, RamStorage<K, W>> {
        Self::with_backend(RamStorage::new())
    }
}

impl<K, W> Default for MultiGraph<K, W, Weighted, RamStorage<K, W>>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, W, B> MultiGraph<K, W, Weighted, B>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    B: StorageBackend<K, W>,
    GraphError: From<B::Error>,
{

    /// Adds a weighted, undirected edge between `source` and `target`.
    ///
    /// Both directions are inserted into the adjacency list. If either node
    /// does not exist, the graph is unchanged.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if either node is missing.
    ///
    /// # Panics
    /// Panics if the reverse-lookup entry for the edge target is `None`.
    pub fn add_edge(&mut self, source: K, target: K, weight: W) -> Result<EdgeView<K, W>, GraphError> {
        let source_hashed = match self.adjacency_list.hashed_nodes_get(&source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(&target)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let edge = Weighted::add_edge(&mut self.adjacency_list, source_hashed, target_hashed, &weight)?;
        Ok(EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight()))

    }

    /// Adds multiple weighted, undirected edges in bulk, skipping any whose
    /// source or target node does not exist.
    pub fn bulk_add_edge(&mut self, edges: &[(K, K, W)]) -> Result<(), GraphError>{
        let mut hashed_edges: Vec<(u64, u64, W)> = Vec::with_capacity(MAX_CAPACITY_BULK);

        for (source, target, weight) in edges{
            if hashed_edges.len() >= MAX_CAPACITY_BULK{
                Weighted::bulk_add_edge(&mut self.adjacency_list, &hashed_edges)?;
                hashed_edges.clear();
            }

            let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
                Some(t) => t,
                None => continue
            };

            let target_hashed = match self.adjacency_list.hashed_nodes_get(target)?{
                Some(t) => t,
                None => continue
            };

            hashed_edges.push((source_hashed, target_hashed, weight.clone()));
        }

        if !hashed_edges.is_empty(){
            Weighted::bulk_add_edge(&mut self.adjacency_list, &hashed_edges)?;
        }

        Ok(())
    }

    /// Removes multiple weighted, undirected edges in bulk, skipping any whose
    /// source or target node does not exist.
    pub fn bulk_remove_edge(&mut self, edges: &[(K, K, W)]) -> Result<(), GraphError>{
        let mut hashed_edges: Vec<(u64, u64, W)> = Vec::with_capacity(MAX_CAPACITY_BULK);

        for (source, target, weight) in edges{
            if hashed_edges.len() >= MAX_CAPACITY_BULK{
                Weighted::bulk_remove_edge(&mut self.adjacency_list, &hashed_edges)?;
                hashed_edges.clear();
            }

            let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
                Some(t) => t,
                None => continue,
            };

            let target_hashed = match self.adjacency_list.hashed_nodes_get(target)?{
                Some(t) => t,
                None => continue,
            };

            hashed_edges.push((source_hashed, target_hashed, weight.clone()));
        }

        if !hashed_edges.is_empty(){
            Weighted::bulk_remove_edge(&mut self.adjacency_list, &hashed_edges)?;
        }

        Ok(())
    }

    /// Removes a weighted, undirected edge matching `source`, `target`, and `weight`.
    ///
    /// Both directions of the edge are removed. The match requires both
    /// target identity **and** weight equality.
    ///
    /// # Errors
    /// * [`GraphError::NodeNotFound`] — if either node does not exist.
    /// * [`GraphError::EdgeDoesntExist`] — if no matching edge is found.
    ///
    /// # Panics
    /// Panics if the reverse-lookup entry for the edge target is `None`.
    pub fn remove_edge(&mut self, source: K, target: K, weight: W) -> Result<EdgeView<K, W>, GraphError>{
        let source_hashed = match self.adjacency_list.hashed_nodes_get(&source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(&target)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let edge = Weighted::remove_edge(&mut self.adjacency_list, source_hashed, target_hashed, &weight)?;

        Ok(EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight()))
    }
}


impl<K, W> MultiGraph<K, W, WeightedDirected, RamStorage<K, W>>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
{
    /// Creates a new, empty weighted directed graph backed by RAM.
    pub fn new() -> MultiGraph<K, W, WeightedDirected, RamStorage<K, W>> {
        Self::with_backend(RamStorage::new())
    }
}

impl<K, W, B> MultiGraph<K, W, WeightedDirected, B>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    W: Clone + std::cmp::PartialEq + AsDiskBytes + FromDiskBytes,
    B: StorageBackend<K, W>,
    GraphError: From<B::Error>,
{

    /// Adds a directed edge from `source` to `target` with the given `weight`.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if either node does not exist.
    ///
    /// # Panics
    /// Panics if the reverse-lookup entry for the edge target is `None`.
    pub fn add_edge(&mut self, source: K, target: K, weight: W) -> Result<EdgeView<K, W>, GraphError> {
        let source_hashed = match self.adjacency_list.hashed_nodes_get(&source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(&target)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let edge = WeightedDirected::add_edge(&mut self.adjacency_list, source_hashed, target_hashed, &weight)?;

        Ok(EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight()))
    }

    /// Adds multiple weighted, directed edges in bulk, skipping any whose
    /// source or target node does not exist.
    pub fn bulk_add_edge(&mut self, edges: &[(K, K, W)]) -> Result<(), GraphError>{
        let mut hashed_edges: Vec<(u64, u64, W)> = Vec::with_capacity(MAX_CAPACITY_BULK);

        for (source, target, weight) in edges{
            if hashed_edges.len() >= MAX_CAPACITY_BULK{
                WeightedDirected::bulk_add_edge(&mut self.adjacency_list, &hashed_edges)?;
                hashed_edges.clear();
            }

            let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
                Some(t) => t,
                None => continue
            };

            let target_hashed = match self.adjacency_list.hashed_nodes_get(target)?{
                Some(t) => t,
                None => continue
            };

            hashed_edges.push((source_hashed, target_hashed, weight.clone()));
        }

        if !hashed_edges.is_empty(){
            WeightedDirected::bulk_add_edge(&mut self.adjacency_list, &hashed_edges)?;
            hashed_edges.clear();
        }

        Ok(())
    }

    /// Removes a weighted, directed edge matching `source`, `target`, and `weight`.
    ///
    /// Only the forward edge is removed (no reverse direction for directed graphs).
    ///
    /// # Errors
    /// * [`GraphError::NodeNotFound`] — if either node does not exist.
    /// * [`GraphError::EdgeDoesntExist`] — if no matching edge is found.
    ///
    /// # Panics
    /// Panics if the reverse-lookup entry for the edge target is `None`.
    pub fn remove_edge(&mut self, source: K, target: K, weight: W) -> Result<EdgeView<K, W>, GraphError>{
        let source_hashed = match self.adjacency_list.hashed_nodes_get(&source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(&target)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let edge = WeightedDirected::remove_edge(&mut self.adjacency_list, source_hashed, target_hashed, &weight)?;

        Ok(EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight()))
    }
}

impl<K> MultiGraph<K, u32, Directed, RamStorage<K, u32>>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
{
    /// Creates a new, empty unweighted directed graph backed by RAM.
    pub fn new() -> MultiGraph<K, u32, Directed, RamStorage<K, u32>> {
        Self::with_backend(RamStorage::new())
    }
}

impl<K, B> MultiGraph<K, u32, Directed, B>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    B: StorageBackend<K, u32>,
    GraphError: From<B::Error>,
{

    /// Adds a directed edge from `source` to `target` with weight `1`.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if either node does not exist.
    ///
    /// # Panics
    /// Panics if the reverse-lookup entry for the edge target is `None`.
    pub fn add_edge(&mut self, source: K, target: K) -> Result<EdgeView<K, u32>, GraphError> {
 
        let source_hashed = match self.adjacency_list.hashed_nodes_get(&source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(&target)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let edge = Directed::add_edge(&mut self.adjacency_list, source_hashed, target_hashed, &1)?;
        
        Ok(EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight()))
    }

    /// Adds multiple unweighted, directed edges in bulk, skipping any whose
    /// source or target node does not exist.
    pub fn bulk_add_edge(&mut self, edges: &[(K, K)]) -> Result<(), GraphError>{

        let mut hashed_edges: Vec<(u64, u64, u32)> = Vec::with_capacity(MAX_CAPACITY_BULK);

        for (source, target) in edges{

            if hashed_edges.len() >= MAX_CAPACITY_BULK{
                Directed::bulk_add_edge(&mut self.adjacency_list, &hashed_edges)?;
                hashed_edges.clear();
            }
            
            let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
                Some(t) => t,
                None => continue,
            };

            let target_hashed = match self.adjacency_list.hashed_nodes_get(target)?{
                Some(t) => t,
                None => continue,
            };

            hashed_edges.push((source_hashed, target_hashed, 1u32));
        }

        if !hashed_edges.is_empty(){
            Directed::bulk_add_edge(&mut self.adjacency_list, &hashed_edges)?;
        }
        Ok(())
    }

    /// Removes an unweighted, directed edge from `source` to `target`.
    ///
    /// Matching is on target identity only (weight is always `1`).
    ///
    /// # Errors
    /// * [`GraphError::NodeNotFound`] — if either node does not exist.
    /// * [`GraphError::EdgeDoesntExist`] — if no matching edge is found.
    ///
    /// # Panics
    /// Panics if the reverse-lookup entry for the edge target is `None`.
    pub fn remove_edge(&mut self, source: K, target: K) -> Result<EdgeView<K, u32>, GraphError>{
        let source_hashed = match self.adjacency_list.hashed_nodes_get(&source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(&target)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let edge = Directed::remove_edge(&mut self.adjacency_list, source_hashed, target_hashed, &1)?;

        Ok(EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight()))
    }

    /// Removes multiple unweighted, directed edges in bulk, skipping any whose
    /// source or target node does not exist.
    pub fn bulk_remove_edge(&mut self, edges: &[(K, K)]) -> Result<(), GraphError>{
        let mut hashed_edges: Vec<(u64, u64, u32)> = Vec::with_capacity(MAX_CAPACITY_BULK);

        for (source, target) in edges{
            if hashed_edges.len() >= MAX_CAPACITY_BULK{
                Directed::bulk_remove_edge(&mut self.adjacency_list, &hashed_edges)?;
                hashed_edges.clear();
            }

            let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
                Some(t) => t,
                None => continue,
            };

            let target_hashed = match self.adjacency_list.hashed_nodes_get(target)?{
                Some(t) => t,
                None => continue,
            };

            hashed_edges.push((source_hashed, target_hashed, 1));
        }

        if !hashed_edges.is_empty(){
            Directed::bulk_remove_edge(&mut self.adjacency_list, &hashed_edges)?;
        }

        Ok(())
    }
}

impl<K> MultiGraph<K, u32, Undirected, RamStorage<K, u32>>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
{
    /// Creates a new, empty unweighted undirected graph backed by RAM.
    pub fn new() -> MultiGraph<K, u32, Undirected, RamStorage<K, u32>> {
        Self::with_backend(RamStorage::new())
    }
}

impl<K, B> MultiGraph<K, u32, Undirected, B>
where
    K: Eq + Hash + Clone + AsDiskBytes + FromDiskBytes,
    B: StorageBackend<K, u32>,
    GraphError: From<B::Error>,
{

    /// Adds an undirected edge between `source` and `target` with weight `1`.
    ///
    /// Both directions are inserted into the adjacency list.
    ///
    /// # Errors
    /// Returns [`GraphError::NodeNotFound`] if either node does not exist.
    ///
    /// # Panics
    /// Panics if the reverse-lookup entry for the edge target is `None`.
    pub fn add_edge(&mut self, source: K, target: K) -> Result<EdgeView<K, u32>, GraphError> {
 
        let source_hashed = match self.adjacency_list.hashed_nodes_get(&source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(&target)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let edge = Undirected::add_edge(&mut self.adjacency_list, source_hashed, target_hashed, &1)?;

        Ok(EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight()))
    }

    /// Adds multiple unweighted, undirected edges in bulk, skipping any whose
    /// source or target node does not exist.
    pub fn bulk_add_edge(&mut self, edges: &[(K, K)]) -> Result<(), GraphError>{
        let mut hashed_nodes : Vec<(u64, u64, u32)> = Vec::with_capacity(MAX_CAPACITY_BULK);
        for (source, target) in edges{
            if hashed_nodes.len() >= MAX_CAPACITY_BULK{
                Undirected::bulk_add_edge(&mut self.adjacency_list, &hashed_nodes)?;
                hashed_nodes.clear();
            }

            let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
                Some(t) => t,
                None => continue
            };

            let target_hashed = match self.adjacency_list.hashed_nodes_get(target)?{
                Some(t) => t,
                None => continue,
            };

            hashed_nodes.push((source_hashed, target_hashed, 1));
        }

        if !hashed_nodes.is_empty(){
            Undirected::bulk_add_edge(&mut self.adjacency_list, &hashed_nodes)?;
        }

        Ok(())
    }

    /// Removes multiple unweighted, undirected edges in bulk, skipping any whose
    /// source or target node does not exist.
    pub fn bulk_remove_edge(&mut self, edges: &[(K, K)]) -> Result<(), GraphError>{
        let mut hashed_edges: Vec<(u64, u64, u32)> = Vec::with_capacity(MAX_CAPACITY_BULK);

        for (source, target) in edges{
            if hashed_edges.len() >= MAX_CAPACITY_BULK{
                Undirected::bulk_remove_edge(&mut self.adjacency_list, &hashed_edges)?;
                hashed_edges.clear();
            }

            let source_hashed = match self.adjacency_list.hashed_nodes_get(source)?{
                Some(t) => t,
                None => continue,
            };

            let target_hashed = match self.adjacency_list.hashed_nodes_get(target)?{
                Some(t) => t,
                None => continue,
            };

            hashed_edges.push((source_hashed, target_hashed, 1));
        }

        if !hashed_edges.is_empty(){
            Undirected::bulk_remove_edge(&mut self.adjacency_list, &hashed_edges)?;
        }

        Ok(())
    }

    /// Removes an unweighted, undirected edge between `source` and `target`.
    ///
    /// Both directions are removed.
    ///
    /// # Errors
    /// * [`GraphError::NodeNotFound`] — if either node does not exist.
    /// * [`GraphError::EdgeDoesntExist`] — if no matching edge is found.
    ///
    /// # Panics
    /// Panics if the reverse-lookup entry for the edge target is `None`.
    pub fn remove_edge(&mut self, source: K, target: K) -> Result<EdgeView<K, u32>, GraphError>{
        let source_hashed = match self.adjacency_list.hashed_nodes_get(&source)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };

        let target_hashed = match self.adjacency_list.hashed_nodes_get(&target)?{
            Some(t) => t,
            None => return Err(GraphError::NodeNotFound),
        };
        let edge = Undirected::remove_edge(&mut self.adjacency_list, source_hashed, target_hashed, &1)?;

        Ok(EdgeView::new(&self.adjacency_list.reverse_hashing_get_node_data(edge.get_target()).unwrap(), &edge.get_weight()))
    }
}
