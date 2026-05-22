//! # MultiGraphRs
//!
//! `multigraphrs` is a versatile and modular graph library built in Rust. 
//! It leverages the **Strategy Pattern** to provide a unified `MultiGraph` data structure 
//! that can behave as a Directed, Undirected, Weighted, or Unweighted graph simply 
//! by swapping out its generic strategy parameter.
//!
//! ## Example
//! ```rust
//! use multigraphrs::{MultiGraph, Directed};
//! 
//! let mut graph = MultiGraph::<&str, u32, Directed>::new();
//! graph.add_node("A").unwrap();
//! graph.add_node("B").unwrap();
//! graph.add_edge("A", "B").unwrap();
//! ```

pub mod core;
pub mod storage;
pub mod strategies;

// Expose the internal types publicly so users can import them easily
pub use strategies::direction_strategy::DirectionStrategy;
pub use strategies::directed::Directed;
pub use strategies::undirected::Undirected;
pub use strategies::weighted::Weighted;
pub use strategies::weighted_directed::WeightedDirected;

pub use core::graph_errors::GraphErrors;
pub use core::multigraph_iterator::{self, NodeIter};
pub use core::edge::EdgeView;
use core::edge::Edge;

pub use storage::disk_storage::disk_multigraph::DiskStorage;
use storage::adjacency_list::RamStorage;
use storage::storage_backend::StorageBackend;

use std::{collections::HashMap, hash::Hash, marker::PhantomData};

/// The core graph structure representing a mathematical graph.
///
/// `MultiGraph` stores nodes and their corresponding edges in an adjacency list.
/// The specific rules for how edges are added (e.g., directed vs. undirected) 
/// are governed by the generic strategy `S`.
///
/// # Type Parameters
/// * `K`: The type of the nodes (Keys). Must implement `Eq`, `Hash`, and `Clone`.
/// * `W`: The type of the edge weights. Must implement `Clone` (allowing floating-point weights).
/// * `S`: The direction strategy (e.g., `Directed`, `Weighted`).
/// * `B`: The storage backend determining how data is stored.
pub struct MultiGraph<K, W, S: DirectionStrategy<W>, B: StorageBackend<W> = RamStorage<W>>
where
    K: Eq + Hash + Clone,
    W: Clone + std::cmp::PartialEq,
{
    hashed_nodes: HashMap<K, u64>,
    pub(crate) reversed_hashed_nodes: Vec<Option<K>>,
    /// The internal adjacency list mapping a node to its outgoing edges.
    pub(crate) adjacency_list: B,
    /// Marker to keep track of the specific strategy `S` and weight `W`.
    _marker: PhantomData<(S, W)>,
    pub(crate) next_id: u64,
    pub(crate) removed_ids: Vec<u64>,
}

pub type RamMultiGraph<K, W, Dir> = MultiGraph<K, W, Dir, RamStorage<W>>;
pub type DiskMultiGraph<K, W, Dir> = MultiGraph<K, W, Dir, DiskStorage<W>>;

// --- Core Methods Shared by ALL Graph Types ---

impl<K, W, S, B> MultiGraph<K, W, S, B>
where
    K: Eq + Hash + Clone,
    W: Clone + std::cmp::PartialEq,
    S: DirectionStrategy<W>,
    B: StorageBackend<W>,
{
    /// Creates a new `MultiGraph` using the given storage backend.
    ///
    /// This is the universal constructor shared by all graph variants.
    /// Strategy-specific `new()` helpers call this with a default backend.
    ///
    /// # Returns
    /// An empty graph that **takes ownership** of `backend`.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn with_backend(backend: B) -> Self {
        MultiGraph {
            adjacency_list: backend,
            _marker: PhantomData,
            hashed_nodes: HashMap::new(),
            reversed_hashed_nodes: Vec::new(),
            next_id: 0,
            removed_ids: Vec::new(),
        }
    }
    /// Adds a single, disconnected node to the graph.
    ///
    /// This is useful for building up the vertices of your graph before 
    /// defining the edges between them.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeAlreadyExists` if the node is already present in the graph.
    pub fn add_node(&mut self, source: K) -> Result<K, GraphErrors> {
        if self.hashed_nodes.contains_key(&source) {
            return Err(GraphErrors::NodeAlreadyExists);
        }
        let node_id;
        if self.removed_ids.is_empty() == true{
            node_id=self.next_id;
            self.adjacency_list.add_node();
            self.next_id+=1;
        }
        else{
            node_id = self.removed_ids.pop().unwrap();
            self.adjacency_list.increment_node_counter();
        }
        
        self.hashed_nodes.insert(source.clone(), node_id);
        if node_id >= self.reversed_hashed_nodes.len() as u64{
            self.reversed_hashed_nodes.push(Some(source.clone()));
        }
        else{
            self.reversed_hashed_nodes[node_id as usize] = Some(source.clone());
        }
        Ok(source)

    }

    /// Removes a node and all edges connected to it from the graph.
    ///
    /// The strategy `S` determines how incoming and outgoing edges are cleaned
    /// up (e.g. directed strategies use the reverse adjacency list, undirected
    /// strategies walk the outgoing neighbours).
    ///
    /// The freed internal ID is recycled and may be reused by future
    /// [`add_node`](Self::add_node) calls.
    ///
    /// # Returns
    /// An **owned clone** of the removed node key on success.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if `source` is not present in the graph.
    ///
    /// # Panics
    /// Panics (via `unwrap`) if the internal reverse-lookup vector is out of
    /// sync with the hash map — this indicates a bug in the library itself.
    pub fn remove_node(&mut self, source: &K) -> Result<K, GraphErrors> {
        let index = match self.hashed_nodes.remove(source) {
            Some(idx) => idx,
            None => return Err(GraphErrors::NodeNotFound),
        };

        self.removed_ids.push(index);
        
        let removed_node = self.reversed_hashed_nodes[index as usize].take().unwrap();
        S::remove_node(&mut self.adjacency_list, index);
        
        Ok(removed_node)
    }

    /// Returns the degree (number of outgoing edges) of the given node.
    ///
    /// # Returns
    /// A **copy** of the edge count (`usize` is `Copy`).
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the node is not in the graph.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn degree(&self, source: &K) -> Result<usize, GraphErrors>{
        match self.hashed_nodes.get(source){
            Some(n) => return Ok(self.adjacency_list.node_len(*n)),
            None => return Err(GraphErrors::NodeNotFound),
        }
    }
    /// Collects all neighbours (outgoing edges) of the given node.
    ///
    /// # Returns
    /// A `Vec` of **cloned** `EdgeView` structs. Each `EdgeView` contains
    /// independent copies of the target key and weight; mutating them will
    /// **not** affect the graph.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if the node is not in the graph.
    ///
    /// # Panics
    /// Panics (via `unwrap`) if any edge targets a node whose reverse-lookup
    /// entry is `None` — this indicates an internal inconsistency.
    pub fn get_neighbours(&self, source: &K) -> Result<Vec<EdgeView<K, W>>, GraphErrors>{
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let neighbours = self.adjacency_list.get_edges(*source_hashed);
        Ok(neighbours
            .map(|edge| EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))
            .collect())
    }

    /// Checks whether the given node key exists in the graph.
    ///
    /// # Returns
    /// A **copy** (`bool` is `Copy`). No data from the graph is moved or cloned.
    ///
    /// # Panics
    /// This method does not panic.
    pub fn contains_node(&self, key: &K) -> bool{
        match self.hashed_nodes.get(&key){
            Some(_) => true,
            None => false,
        }
    }

    /// Returns the total number of nodes currently in the graph.
    ///
    /// # Returns
    /// A **copy** of the count (`usize` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn node_count(&self) -> usize{
        self.adjacency_list.node_count()
    }

    /// Returns the total number of edges currently in the graph.
    ///
    /// For undirected graphs each logical connection counts as **two** internal
    /// edges (one per direction).
    ///
    /// # Returns
    /// A **copy** of the count (`usize` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn edge_count(&self) -> usize{
        self.adjacency_list.edge_count()
    }

    /// Returns an iterator over all nodes in the graph and their edges.
    ///
    /// Each item yielded is a tuple of:
    /// * An **immutable reference** (`&K`) to the node key (borrows from `self`).
    /// * A `Vec<EdgeView<K, W>>` of **cloned** edge views for that node.
    ///
    /// Removed ("tombstoned") node slots are automatically skipped.
    ///
    /// # Panics
    /// Panics (via `unwrap`) if the reverse-lookup vector is inconsistent — this
    /// indicates an internal bug.
    pub fn iter(&self) -> multigraph_iterator::NodeIter<'_, K, W, S, B> {
        multigraph_iterator::NodeIter { graph: self, index: 0 }
    }

    /// Checks whether an edge from `source` to `target` exists.
    ///
    /// Returns `false` if either node does not exist (rather than erroring).
    ///
    /// # Returns
    /// A **copy** (`bool` is `Copy`).
    ///
    /// # Panics
    /// This method does not panic.
    pub fn contains_edge(&self, source: &K, target: &K) -> bool{

        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return false,
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return false,
        };

        match self.adjacency_list.contains_edge(*source_hashed, *target_hashed) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

// --- Strategy-Specific Implementations ---

impl<K, W> MultiGraph<K, W, Weighted, RamStorage<W>>
where
    K: Eq + Hash + Clone,
    W: Clone + std::cmp::PartialEq,
{
    /// Creates a new, empty `Weighted` (undirected) graph.
    pub fn new() -> MultiGraph<K, W, Weighted, RamStorage<W>> {
        Self::with_backend(RamStorage::new())
    }
}

impl<K, W, B> MultiGraph<K, W, Weighted, B>
where
    K: Eq + Hash + Clone,
    W: Clone + std::cmp::PartialEq,
    B: StorageBackend<W>,
{

    /// Adds a weighted edge between two nodes in both directions.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if either the `source` or `target` node 
    /// does not exist in the graph prior to adding the edge.
    pub fn add_edge(&mut self, source: K, target: K, weight: W) -> Result<EdgeView<K, W>, GraphErrors> {
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let edge = Weighted::add_edge(&mut self.adjacency_list, *source_hashed, *target_hashed, &weight)?;
        Ok(EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))

    }

    /// Removes a weighted, undirected edge matching the given `source`, `target`, and `weight`.
    ///
    /// Both directions of the edge are removed. The match is performed on
    /// both target identity **and** weight equality.
    ///
    /// # Returns
    /// A **cloned** `EdgeView` of the removed edge on success.
    ///
    /// # Errors
    /// * `GraphErrors::NodeNotFound` — if either node does not exist.
    /// * `GraphErrors::EdgeDoesntExists` — if no matching edge is found.
    ///
    /// # Panics
    /// Panics (via `unwrap`) if the reverse-lookup entry for the edge target is `None`.
    pub fn remove_edge(&mut self, source: K, target: K, weight: W) -> Result<EdgeView<K, W>, GraphErrors>{
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let edge = Weighted::remove_edge(&mut self.adjacency_list, *source_hashed, *target_hashed, &weight)?;

        Ok(EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))
    }
}


impl<K, W> MultiGraph<K, W, WeightedDirected, RamStorage<W>>
where
    K: Eq + Hash + Clone,
    W: Clone + std::cmp::PartialEq,
{
    /// Creates a new, empty `WeightedDirected` graph.
    pub fn new() -> MultiGraph<K, W, WeightedDirected, RamStorage<W>> {
        Self::with_backend(RamStorage::new())
    }
}

impl<K, W, B> MultiGraph<K, W, WeightedDirected, B>
where
    K: Eq + Hash + Clone,
    W: Clone + std::cmp::PartialEq,
    B: StorageBackend<W>,
{

    /// Adds a directed edge from `source` to `target` with the given `weight`.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if either node does not exist.
    pub fn add_edge(&mut self, source: K, target: K, weight: W) -> Result<EdgeView<K, W>, GraphErrors> {
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let edge = WeightedDirected::add_edge(&mut self.adjacency_list, *source_hashed, *target_hashed, &weight)?;

        Ok(EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))
    }

    /// Removes a weighted, directed edge matching the given `source`, `target`, and `weight`.
    ///
    /// Only the single forward edge is removed. The match is performed on
    /// both target identity **and** weight equality.
    ///
    /// # Returns
    /// A **cloned** `EdgeView` of the removed edge on success.
    ///
    /// # Errors
    /// * `GraphErrors::NodeNotFound` — if either node does not exist.
    /// * `GraphErrors::EdgeDoesntExists` — if no matching edge is found.
    ///
    /// # Panics
    /// Panics (via `unwrap`) if the reverse-lookup entry for the edge target is `None`.
    pub fn remove_edge(&mut self, source: K, target: K, weight: W) -> Result<EdgeView<K, W>, GraphErrors>{
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let edge = WeightedDirected::remove_edge(&mut self.adjacency_list, *source_hashed, *target_hashed, &weight)?;

        Ok(EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))
    }
}

impl<K> MultiGraph<K, u32, Directed, RamStorage<u32>>
where
    K: Eq + Hash + Clone,
{
    /// Creates a new, empty, unweighted `Directed` graph.
    pub fn new() -> MultiGraph<K, u32, Directed, RamStorage<u32>> {
        Self::with_backend(RamStorage::new())
    }
}

impl<K, B> MultiGraph<K, u32, Directed, B>
where
    K: Eq + Hash + Clone,
    B: StorageBackend<u32>,
{

    /// Adds a directed edge from `source` to `target` with a default weight of 1.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if either node does not exist.
    pub fn add_edge(&mut self, source: K, target: K) -> Result<EdgeView<K, u32>, GraphErrors> {
 
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let edge = Directed::add_edge(&mut self.adjacency_list, *source_hashed, *target_hashed, &1)?;
        
        Ok(EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))
    }

    /// Removes an unweighted, directed edge from `source` to `target`.
    ///
    /// Matching is performed on target identity only (weight is always `1`).
    ///
    /// # Returns
    /// A **cloned** `EdgeView` of the removed edge on success.
    ///
    /// # Errors
    /// * `GraphErrors::NodeNotFound` — if either node does not exist.
    /// * `GraphErrors::EdgeDoesntExists` — if no matching edge is found.
    ///
    /// # Panics
    /// Panics (via `unwrap`) if the reverse-lookup entry for the edge target is `None`.
    pub fn remove_edge(&mut self, source: K, target: K) -> Result<EdgeView<K, u32>, GraphErrors>{
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let edge = Directed::remove_edge(&mut self.adjacency_list, *source_hashed, *target_hashed, &1)?;

        Ok(EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))
    }
}

impl<K> MultiGraph<K, u32, Undirected, RamStorage<u32>>
where
    K: Eq + Hash + Clone,
{
    /// Creates a new, empty, unweighted `Undirected` graph.
    pub fn new() -> MultiGraph<K, u32, Undirected, RamStorage<u32>> {
        Self::with_backend(RamStorage::new())
    }
}

impl<K, B> MultiGraph<K, u32, Undirected, B>
where
    K: Eq + Hash + Clone,
    B: StorageBackend<u32>,
{

    /// Adds an undirected connection (edges in both directions) between `source` and `target`, defaulting weight to 1.
    ///
    /// # Errors
    /// Returns `GraphErrors::NodeNotFound` if either node does not exist.
    pub fn add_edge(&mut self, source: K, target: K) -> Result<EdgeView<K, u32>, GraphErrors> {
 
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let edge = Undirected::add_edge(&mut self.adjacency_list, *source_hashed, *target_hashed, &1)?;

        Ok(EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))
    }

    /// Removes an unweighted, undirected edge between `source` and `target`.
    ///
    /// Both directions of the edge are removed.
    ///
    /// # Returns
    /// A **cloned** `EdgeView` of the removed edge on success.
    ///
    /// # Errors
    /// * `GraphErrors::NodeNotFound` — if either node does not exist.
    /// * `GraphErrors::EdgeDoesntExists` — if no matching edge is found.
    ///
    /// # Panics
    /// Panics (via `unwrap`) if the reverse-lookup entry for the edge target is `None`.
    pub fn remove_edge(&mut self, source: K, target: K) -> Result<EdgeView<K, u32>, GraphErrors>{
        let source_hashed = match self.hashed_nodes.get(&source){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };

        let target_hashed = match self.hashed_nodes.get(&target){
            Some(t) => t,
            None => return Err(GraphErrors::NodeNotFound),
        };
        let edge = Undirected::remove_edge(&mut self.adjacency_list, *source_hashed, *target_hashed, &1)?;

        Ok(EdgeView::new(self.reversed_hashed_nodes[edge.get_target() as usize].as_ref().unwrap(), &edge.get_weight()))
    }
}
