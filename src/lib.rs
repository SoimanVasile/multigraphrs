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
    hashed_nodes: HashMap<K, u32>,
    pub(crate) reversed_hashed_nodes: Vec<Option<K>>,
    /// The internal adjacency list mapping a node to its outgoing edges.
    pub(crate) adjacency_list: B,
    /// Marker to keep track of the specific strategy `S` and weight `W`.
    _marker: PhantomData<(S, W)>,
    pub(crate) next_id: u32,
    pub(crate) removed_ids: Vec<u32>,
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
        if node_id >= self.reversed_hashed_nodes.len() as u32{
            self.reversed_hashed_nodes.push(Some(source.clone()));
        }
        else{
            self.reversed_hashed_nodes[node_id as usize] = Some(source.clone());
        }
        Ok(source)

    }

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

    pub fn degree(&self, source: &K) -> Result<usize, GraphErrors>{
        match self.hashed_nodes.get(source){
            Some(n) => return Ok(self.adjacency_list.node_len(*n)),
            None => return Err(GraphErrors::NodeNotFound),
        }
    }
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

    pub fn contains_node(&self, key: &K) -> bool{
        match self.hashed_nodes.get(&key){
            Some(_) => true,
            None => false,
        }
    }

    pub fn node_count(&self) -> usize{
        self.adjacency_list.node_count()
    }

    pub fn edge_count(&self) -> usize{
        self.adjacency_list.edge_count()
    }

    pub fn iter(&self) -> multigraph_iterator::NodeIter<'_, K, W, S, B> {
        multigraph_iterator::NodeIter { graph: self, index: 0 }
    }

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
